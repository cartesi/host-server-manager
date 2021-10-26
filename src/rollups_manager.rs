// Copyright 2021 Cartesi Pte. Ltd.
//
// This file is part of the machine-emulator. The machine-emulator is free
// software: you can redistribute it and/or modify it under the terms of the GNU
// Lesser General Public License as published by the Free Software Foundation,
// either version 3 of the License, or (at your option) any later version.
//
// The machine-emulator is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE. See the GNU Lesser General Public License
// for more details.
//
// You should have received a copy of the GNU Lesser General Public License
// along with the machine-emulator. If not, see http://www.gnu.org/licenses/.

use std::{error::Error, fmt, sync::Arc};
use tokio::sync::{mpsc, Mutex, Notify};

use super::model::{AdvanceMetadata, AdvanceRequest, Notice, Voucher};
use super::repository::{Identified, Repository};

/// Create the necessary structs to manage the rollups state.
/// To advance the rollups state, the client code should send an AdvanceRequest through the sender.
/// The client code should run the RollupsWorker in the background to process the advance requests.
/// The client code should use the RollupsManager to finish the advance requests and to add
/// notices and vouchers.
pub fn setup(
    repository: Arc<Mutex<Repository>>,
) -> (
    mpsc::Sender<AdvanceRequest>,
    RollupsWorker,
    Arc<Mutex<RollupsManager>>,
) {
    let (advance_tx, advance_rx) = mpsc::channel::<AdvanceRequest>(1000);
    let finish_notify = Arc::new(Notify::new());
    let manager = Arc::new(Mutex::new(RollupsManager::new(
        repository,
        Arc::clone(&finish_notify),
    )));
    let worker = RollupsWorker::new(Arc::clone(&manager), advance_rx, finish_notify);

    (advance_tx, worker, manager)
}

enum RollupsState {
    Idle,
    Advancing(AdvanceMetadata),
}

/// Manages the state machine for the advance rollups logic.
/// The manager state can be either idle or advancing the rollups.
/// When the manager is idle, it can only wait for the advance rollups request.
/// When the manager is advancing the rollups, it can receive notices and vouchers until accept or
/// reject is called.
pub struct RollupsManager {
    state: RollupsState,
    last_valid_id: u64,
    curr_id: u64,
    vouchers: Vec<Identified<Voucher>>,
    notices: Vec<Identified<Notice>>,
    repository: Arc<Mutex<Repository>>,
    finish_notify: Arc<Notify>,
}

impl RollupsManager {
    fn new(repository: Arc<Mutex<Repository>>, finish_notify: Arc<Notify>) -> Self {
        Self {
            state: RollupsState::Idle,
            last_valid_id: 0,
            curr_id: 0,
            vouchers: vec![],
            notices: vec![],
            repository,
            finish_notify,
        }
    }

    async fn advance(&mut self, request: AdvanceRequest) -> Result<(), Box<dyn Error>> {
        match self.state {
            RollupsState::Idle => {
                self.state = RollupsState::Advancing(request.metadata.clone());

                super::dapp_client::advance(request).await
            }
            _ => Err(RollupsStateError::AdvanceError.into()),
        }
    }

    /// Add a new voucher and return the voucher id.
    pub fn add_voucher(&mut self, voucher: Voucher) -> Result<u64, RollupsStateError> {
        match self.state {
            RollupsState::Advancing(_) => Ok(add_identified(
                &mut self.curr_id,
                &mut self.vouchers,
                voucher,
            )),
            _ => Err(RollupsStateError::AddVoucherError.into()),
        }
    }

    /// Add a new notice and return the notice id.
    pub fn add_notice(&mut self, notice: Notice) -> Result<u64, RollupsStateError> {
        match self.state {
            RollupsState::Advancing(_) => {
                Ok(add_identified(&mut self.curr_id, &mut self.notices, notice))
            }
            _ => Err(RollupsStateError::AddNoticeError.into()),
        }
    }

    /// Accept the rollups advance request.
    /// This method stores the vouchers and notices received.
    pub async fn accept(&mut self) -> Result<(), RollupsStateError> {
        match &mut self.state {
            RollupsState::Advancing(_metadata) => {
                {
                    let _repository = self.repository.lock();
                    // TODO save stuff to repository
                }
                self.last_valid_id = self.curr_id;
                Ok(self.finish())
            }
            _ => Err(RollupsStateError::AcceptError.into()),
        }
    }

    /// Accept the rollups advance request.
    /// This method discards the vouchers and notices received.
    pub async fn reject(&mut self) -> Result<(), RollupsStateError> {
        match self.state {
            RollupsState::Advancing(_) => {
                self.curr_id = self.last_valid_id;
                Ok(self.finish())
            }
            _ => Err(RollupsStateError::RejectError.into()),
        }
    }

    /// Clean up all vouchers and notices and set state to idle.
    fn finish(&mut self) {
        self.state = RollupsState::Idle;
        self.vouchers.clear();
        self.notices.clear();
        self.finish_notify.notify_one();
    }
}

/// Add an identified value to the given vector and increase the current id.
fn add_identified<T>(curr_id: &mut u64, v: &mut Vec<Identified<T>>, value: T) -> u64 {
    let id = *curr_id;
    *curr_id = id + 1;
    v.push(Identified { id, value });

    id
}

/// Worker that advances the RollupManager when it receives an advance request
pub struct RollupsWorker {
    manager: Arc<Mutex<RollupsManager>>,
    advance_rx: mpsc::Receiver<AdvanceRequest>,
    finish_notify: Arc<Notify>,
}

impl RollupsWorker {
    fn new(
        manager: Arc<Mutex<RollupsManager>>,
        advance_rx: mpsc::Receiver<AdvanceRequest>,
        finish_notify: Arc<Notify>,
    ) -> Self {
        Self {
            manager,
            advance_rx,
            finish_notify,
        }
    }

    /// Advances the state
    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        while let Some(request) = self.advance_rx.recv().await {
            // Update manager and release lock
            {
                let mut manager = self.manager.lock().await;
                manager.advance(request).await?;
            }
            // Wait for the manager to notify
            self.finish_notify.notified().await;
        }

        Ok(())
    }
}

/// Errors that are caused by doing the wrong operations for given the state
#[derive(Debug)]
pub enum RollupsStateError {
    AdvanceError,
    AddVoucherError,
    AddNoticeError,
    AcceptError,
    RejectError,
}

impl fmt::Display for RollupsStateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RollupsStateError::AdvanceError => "cannot advance state when not idle".fmt(f),
            RollupsStateError::AddVoucherError => "cannot add voucher when not advancing".fmt(f),
            RollupsStateError::AddNoticeError => "cannot add notice when not advancing".fmt(f),
            RollupsStateError::AcceptError => "cannot accept when not advancing".fmt(f),
            RollupsStateError::RejectError => "cannot reject when not advancing".fmt(f),
        }
    }
}

impl Error for RollupsStateError {}
