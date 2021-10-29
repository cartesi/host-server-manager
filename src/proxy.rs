// Copyright 2021 Cartesi Pte. Ltd.
//
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

use std::{error::Error, fmt, mem, sync::Arc};
use tokio::sync::{mpsc, oneshot, Mutex, Notify};

use super::model::{AdvanceMetadata, AdvanceRequest, InspectRequest, Notice, Report, Voucher};
use super::repository::{Identified, Repository};

/// Create the necessary structs to manage the communication between the gRPC interface and the
/// DApp backend.
///
/// The advancer schedules new advance requests.
/// The inspector calls the inspect method on the DApp.
/// The dapp_interface calls the DApp and receives the vouchers, notices, reports, and finish.
/// The worker is responsible for the synchronization. It should be ran in the background.
pub fn setup(
    repository: Arc<Mutex<Repository>>,
) -> (Advancer, Inspector, Arc<Mutex<DAppInterface>>, Worker) {
    let (advance_tx, advance_rx) = mpsc::channel::<AdvanceRequest>(1000);
    let (inspect_tx, inspect_rx) = mpsc::channel::<InspectRequestSync>(1000);

    let inspector = Inspector { inspect_tx };
    let dapp_interface = Arc::new(Mutex::new(DAppInterface::new(repository)));
    let worker = Worker {
        advance_rx,
        inspect_rx,
        dapp_interface: dapp_interface.clone(),
    };

    (advance_tx, inspector, dapp_interface, worker)
}

/// A regular mpsc sender; use the send method.
pub type Advancer = mpsc::Sender<AdvanceRequest>;

#[derive(Debug)]
struct InspectRequestSync {
    request: InspectRequest,
    response_tx: oneshot::Sender<Vec<Report>>,
}

/// Send inspect requests.
pub struct Inspector {
    inspect_tx: mpsc::Sender<InspectRequestSync>,
}

impl Inspector {
    /// Inspect the DApp.
    /// If the DApp is currently processing an input, the method waits for the DApp to finish
    /// before running the inspect.
    pub async fn inspect(
        &mut self,
        request: InspectRequest,
    ) -> Result<Vec<Report>, Box<dyn Error>> {
        let (response_tx, response_rx) = oneshot::channel();
        self.inspect_tx
            .send(InspectRequestSync {
                request,
                response_tx,
            })
            .await?;
        Ok(response_rx.await?)
    }
}

enum DAppState {
    Idle,
    Advancing {
        metadata: AdvanceMetadata,
        finish: Arc<Notify>,
    },
}

/// Manages the state-machine for the asynchornous advance.
/// The dapp_interface state can be either idle or advancing the rollups.
/// When the dapp_interface is idle, it can wait for the advance rollups request or respond to
/// inspect requests.
/// When the dapp_interface is advancing the rollups, it can receive vouchers, notices, and reports
/// until accept or reject is called.
pub struct DAppInterface {
    state: DAppState,
    last_valid_id: u64,
    curr_id: u64,
    vouchers: Vec<Identified<Voucher>>,
    notices: Vec<Identified<Notice>>,
    reports: Vec<Report>,
    repository: Arc<Mutex<Repository>>,
}

impl DAppInterface {
    fn new(repository: Arc<Mutex<Repository>>) -> Self {
        Self {
            state: DAppState::Idle,
            last_valid_id: 0,
            curr_id: 0,
            vouchers: vec![],
            notices: vec![],
            reports: vec![],
            repository,
        }
    }

    async fn advance(
        &mut self,
        request: AdvanceRequest,
        finish: Arc<Notify>,
    ) -> Result<(), Box<dyn Error>> {
        match self.state {
            DAppState::Idle => {
                self.state = DAppState::Advancing {
                    metadata: request.metadata.clone(),
                    finish,
                };
                super::dapp_client::advance(request).await
            }
            _ => Err(DAppStateError::AdvanceError.into()),
        }
    }

    async fn inspect(&mut self, request: InspectRequest) -> Result<Vec<Report>, Box<dyn Error>> {
        match self.state {
            DAppState::Idle => Ok(super::dapp_client::inspect(request).await?),
            _ => Err(DAppStateError::InspectError.into()),
        }
    }

    /// Add a new voucher and return the voucher id.
    pub fn add_voucher(&mut self, voucher: Voucher) -> Result<u64, DAppStateError> {
        match self.state {
            DAppState::Advancing { .. } => Ok(add_identified(
                &mut self.curr_id,
                &mut self.vouchers,
                voucher,
            )),
            _ => Err(DAppStateError::AddVoucherError.into()),
        }
    }

    /// Add a new notice and return the notice id.
    pub fn add_notice(&mut self, notice: Notice) -> Result<u64, DAppStateError> {
        match self.state {
            DAppState::Advancing { .. } => {
                Ok(add_identified(&mut self.curr_id, &mut self.notices, notice))
            }
            _ => Err(DAppStateError::AddNoticeError.into()),
        }
    }

    /// Add a new report.
    pub fn add_report(&mut self, report: Report) -> Result<(), DAppStateError> {
        match self.state {
            DAppState::Advancing { .. } => Ok(self.reports.push(report)),
            _ => Err(DAppStateError::AddReportError.into()),
        }
    }

    /// Accept the rollups advance request.
    /// This method stores the vouchers and notices received.
    pub async fn accept(&mut self) -> Result<(), DAppStateError> {
        let mut state = DAppState::Idle;
        mem::swap(&mut self.state, &mut state);
        match state {
            DAppState::Advancing {
                metadata: _metadata,
                finish,
            } => {
                {
                    let _repository = self.repository.lock();
                    // TODO save stuff to repository
                    self.vouchers.clear();
                    self.notices.clear();
                    self.reports.clear();
                }
                self.last_valid_id = self.curr_id;
                finish.notify_one();
                Ok(())
            }
            _ => Err(DAppStateError::AcceptError.into()),
        }
    }

    /// Accept the rollups advance request.
    /// This method discards the vouchers and notices received.
    pub async fn reject(&mut self) -> Result<(), DAppStateError> {
        let mut state = DAppState::Idle;
        mem::swap(&mut self.state, &mut state);
        match state {
            DAppState::Advancing {
                metadata: _metadata,
                finish,
            } => {
                {
                    let _repository = self.repository.lock();
                    // TODO save reports to repository
                    self.reports.clear();
                }
                self.curr_id = self.last_valid_id;
                self.vouchers.clear();
                self.notices.clear();
                finish.notify_one();
                Ok(())
            }
            _ => Err(DAppStateError::RejectError.into()),
        }
    }
}

/// Add an identified value to the given vector and increase the current id.
fn add_identified<T>(curr_id: &mut u64, v: &mut Vec<Identified<T>>, value: T) -> u64 {
    let id = *curr_id;
    *curr_id = id + 1;
    v.push(Identified { id, value });

    id
}

pub struct Worker {
    advance_rx: mpsc::Receiver<AdvanceRequest>,
    inspect_rx: mpsc::Receiver<InspectRequestSync>,
    dapp_interface: Arc<Mutex<DAppInterface>>,
}

impl Worker {
    /// This should be running in the background to send the requests to the dapp_interface
    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            tokio::select! {
                Some(sync) = self.inspect_rx.recv() => {
                    let mut dapp_interface = self.dapp_interface.lock().await;
                    let reports = dapp_interface.inspect(sync.request).await?;
                    if let Err(_) = sync.response_tx.send(reports) {
                        return Err(Box::new(OutOfSyncError{}));
                    }
                }
                Some(request) = self.advance_rx.recv() => {
                    let finish = Arc::new(Notify::new());
                    {
                        let mut dapp_interface = self.dapp_interface.lock().await;
                        dapp_interface.advance(request, finish.clone()).await?;
                    }
                    finish.notified().await;
                }
            }
        }
    }
}

/// Error caused by calling the wrong operations for given the dapp_interface state.
#[derive(Debug)]
pub enum DAppStateError {
    AdvanceError,
    InspectError,
    AddVoucherError,
    AddNoticeError,
    AddReportError,
    AcceptError,
    RejectError,
}

impl Error for DAppStateError {}

impl fmt::Display for DAppStateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DAppStateError::AdvanceError => "cannot advance state when not idle".fmt(f),
            DAppStateError::InspectError => "cannot inspect when not idle".fmt(f),
            DAppStateError::AddVoucherError => "cannot add voucher when not advancing".fmt(f),
            DAppStateError::AddNoticeError => "cannot add notice when not advancing".fmt(f),
            DAppStateError::AddReportError => "cannot add report when not advancing".fmt(f),
            DAppStateError::AcceptError => "cannot accept when not advancing".fmt(f),
            DAppStateError::RejectError => "cannot reject when not advancing".fmt(f),
        }
    }
}

#[derive(Debug)]
pub struct OutOfSyncError;

impl Error for OutOfSyncError {}

impl fmt::Display for OutOfSyncError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        "Out of sync (this should not happen).".fmt(f)
    }
}
