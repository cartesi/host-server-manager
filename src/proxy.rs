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

#[cfg(test)]
use mockall::{automock, predicate::*};

use async_trait::async_trait;
use std::{error::Error, sync::Arc};
use tokio::{
    self,
    sync::{mpsc, oneshot, Mutex},
};

use crate::model::{AdvanceMetadata, AdvanceRequest, InspectRequest, Notice, Report, Voucher};

/// Create the proxy service and the proxy channel.
/// The service should be ran in the background and the channel can be used to communicate with it.
pub fn new(
    repository_channel: Arc<Mutex<dyn RepositoryChannel + Send + Sync>>,
    dapp_channel: Arc<Mutex<dyn DAppChannel + Send + Sync>>,
) -> (ProxyChannel, ProxyService) {
    let (advance_tx, advance_rx) = mpsc::channel::<AdvanceRequest>(1000);
    let (inspect_tx, inspect_rx) = mpsc::channel::<SyncInspectRequest>(1000);
    let (voucher_tx, voucher_rx) = mpsc::channel::<SyncVoucherRequest>(1000);
    let (notice_tx, notice_rx) = mpsc::channel::<SyncNoticeRequest>(1000);
    let (report_tx, report_rx) = mpsc::channel::<Report>(1000);
    let (accept_tx, accept_rx) = mpsc::channel::<()>(1000);
    let (reject_tx, reject_rx) = mpsc::channel::<()>(1000);

    let channel = ProxyChannel {
        advance_tx,
        inspect_tx,
        voucher_tx,
        notice_tx,
        report_tx,
        accept_tx,
        reject_tx,
    };

    let service = ProxyService {
        state: IdleState::new(repository_channel, dapp_channel, 0),
        advance_rx,
        inspect_rx,
        voucher_rx,
        notice_rx,
        report_rx,
        accept_rx,
        reject_rx,
    };

    (channel, service)
}

/// Channel used to communicate with the DApp
#[cfg_attr(test, automock)]
#[async_trait]
pub trait DAppChannel {
    /// Send an advance request to the client
    async fn advance(&mut self, request: AdvanceRequest) -> Result<(), Box<dyn Error>>;

    /// Send an inspect request to the client
    async fn inspect(&mut self, request: InspectRequest) -> Result<Vec<Report>, Box<dyn Error>>;
}

#[derive(Debug)]
pub struct Identified<T> {
    pub id: u64,
    pub value: T,
}

/// Channel used to communicate with the repository
#[cfg_attr(test, automock)]
#[async_trait]
pub trait RepositoryChannel {
    /// Send request to store the vouchers
    async fn store_vouchers(
        &mut self,
        metadata: &AdvanceMetadata,
        vouchers: Vec<Identified<Voucher>>,
    ) -> Result<(), Box<dyn Error>>;

    /// Send request to store the notices
    async fn store_notices(
        &mut self,
        metadata: &AdvanceMetadata,
        notices: Vec<Identified<Notice>>,
    ) -> Result<(), Box<dyn Error>>;

    /// Send request to store a report
    async fn store_report(&mut self, report: Report) -> Result<(), Box<dyn Error>>;
}

/// Channel used to communicate with the Proxy
#[derive(Clone)]
pub struct ProxyChannel {
    advance_tx: mpsc::Sender<AdvanceRequest>,
    inspect_tx: mpsc::Sender<SyncInspectRequest>,
    voucher_tx: mpsc::Sender<SyncVoucherRequest>,
    notice_tx: mpsc::Sender<SyncNoticeRequest>,
    report_tx: mpsc::Sender<Report>,
    accept_tx: mpsc::Sender<()>,
    reject_tx: mpsc::Sender<()>,
}

impl ProxyChannel {
    /// Send an advance request
    pub async fn advance(&mut self, request: AdvanceRequest) -> Result<(), Box<dyn Error>> {
        Ok(self.advance_tx.send(request).await?)
    }

    /// Send an inspect request and wait for the report
    pub async fn inspect(
        &mut self,
        request: InspectRequest,
    ) -> Result<Vec<Report>, Box<dyn Error>> {
        Self::make_sync_request(request, &mut self.inspect_tx).await
    }

    /// Send a voucher request
    pub async fn add_voucher(&mut self, voucher: Voucher) -> Result<u64, Box<dyn Error>> {
        Self::make_sync_request(voucher, &mut self.voucher_tx).await
    }

    /// Send a notice request
    pub async fn add_notice(&mut self, notice: Notice) -> Result<u64, Box<dyn Error>> {
        Self::make_sync_request(notice, &mut self.notice_tx).await
    }

    /// Send a report request
    pub async fn add_report(&mut self, request: Report) -> Result<(), Box<dyn Error>> {
        Ok(self.report_tx.send(request).await?)
    }

    /// Send a accept request
    pub async fn accept(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(self.accept_tx.send(()).await?)
    }

    /// Send a reject request
    pub async fn reject(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(self.reject_tx.send(()).await?)
    }

    async fn make_sync_request<T: std::fmt::Debug + 'static, U: std::fmt::Debug + 'static>(
        value: T,
        tx: &mut mpsc::Sender<SyncRequest<T, U>>,
    ) -> Result<U, Box<dyn Error>> {
        let (response_tx, response_rx) = oneshot::channel();
        tx.send(SyncRequest { value, response_tx }).await?;
        Ok(response_rx.await?)
    }
}

pub struct ProxyService {
    state: State,
    advance_rx: mpsc::Receiver<AdvanceRequest>,
    inspect_rx: mpsc::Receiver<SyncInspectRequest>,
    voucher_rx: mpsc::Receiver<SyncVoucherRequest>,
    notice_rx: mpsc::Receiver<SyncNoticeRequest>,
    report_rx: mpsc::Receiver<Report>,
    accept_rx: mpsc::Receiver<()>,
    reject_rx: mpsc::Receiver<()>,
}

impl ProxyService {
    pub async fn run(mut self) -> Result<(), Box<dyn Error>> {
        loop {
            self.state = match self.state {
                State::Idle(idle) => {
                    tokio::select! {
                        Some(request) = self.advance_rx.recv() => {
                            idle.advance(request).await?
                        }
                        Some(request) = self.inspect_rx.recv() => {
                            idle.inspect(request).await?
                        }
                    }
                }
                State::Advancing(advancing) => {
                    tokio::select! {
                        Some(request) = self.voucher_rx.recv() => {
                            advancing.add_voucher(request).await?
                        }
                        Some(request) = self.notice_rx.recv() => {
                            advancing.add_notice(request).await?
                        }
                        Some(report) = self.report_rx.recv() => {
                            advancing.add_report(report).await?
                        }
                        Some(_) = self.accept_rx.recv() => {
                            advancing.accept().await?
                        }
                        Some(_) = self.reject_rx.recv() => {
                            advancing.reject().await?
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct SyncRequest<T, U> {
    value: T,
    response_tx: oneshot::Sender<U>,
}

type SyncInspectRequest = SyncRequest<InspectRequest, Vec<Report>>;
type SyncVoucherRequest = SyncRequest<Voucher, u64>;
type SyncNoticeRequest = SyncRequest<Notice, u64>;

enum State {
    Idle(IdleState),
    Advancing(AdvancingState),
}

struct IdleState {
    repository_channel: Arc<Mutex<dyn RepositoryChannel + Send + Sync>>,
    dapp_channel: Arc<Mutex<dyn DAppChannel + Send + Sync>>,
    id: u64,
}

impl IdleState {
    fn new(
        repository_channel: Arc<Mutex<dyn RepositoryChannel + Send + Sync>>,
        dapp_channel: Arc<Mutex<dyn DAppChannel + Send + Sync>>,
        id: u64,
    ) -> State {
        State::Idle(Self {
            repository_channel,
            dapp_channel,
            id,
        })
    }

    async fn advance(self, request: AdvanceRequest) -> Result<State, Box<dyn Error>> {
        let metadata = request.metadata.clone();
        self.dapp_channel.lock().await.advance(request).await?;
        Ok(AdvancingState::new(
            self.repository_channel,
            self.dapp_channel,
            metadata,
            self.id,
        ))
    }

    async fn inspect(self, request: SyncInspectRequest) -> Result<State, Box<dyn Error>> {
        let response = self
            .dapp_channel
            .lock()
            .await
            .inspect(request.value)
            .await?;
        request
            .response_tx
            .send(response)
            .expect("oneshot channel dropped");
        Ok(State::Idle(self))
    }
}

struct AdvancingState {
    repository_channel: Arc<Mutex<dyn RepositoryChannel + Send + Sync>>,
    dapp_channel: Arc<Mutex<dyn DAppChannel + Send + Sync>>,
    metadata: AdvanceMetadata,
    previous_id: u64,
    current_id: u64,
    vouchers: Vec<Identified<Voucher>>,
    notices: Vec<Identified<Notice>>,
}

impl AdvancingState {
    fn new(
        repository_channel: Arc<Mutex<dyn RepositoryChannel + Send + Sync>>,
        dapp_channel: Arc<Mutex<dyn DAppChannel + Send + Sync>>,
        metadata: AdvanceMetadata,
        id: u64,
    ) -> State {
        State::Advancing(Self {
            repository_channel,
            dapp_channel,
            metadata,
            previous_id: id,
            current_id: id,
            vouchers: vec![],
            notices: vec![],
        })
    }

    async fn add_voucher(mut self, request: SyncVoucherRequest) -> Result<State, Box<dyn Error>> {
        Self::add_identified(request, &mut self.vouchers, &mut self.current_id);
        Ok(State::Advancing(self))
    }

    async fn add_notice(mut self, request: SyncNoticeRequest) -> Result<State, Box<dyn Error>> {
        Self::add_identified(request, &mut self.notices, &mut self.current_id);
        Ok(State::Advancing(self))
    }

    async fn add_report(self, report: Report) -> Result<State, Box<dyn Error>> {
        {
            let mut repository_channel = self.repository_channel.lock().await;
            repository_channel.store_report(report).await?;
        }
        Ok(State::Advancing(self))
    }

    async fn accept(mut self) -> Result<State, Box<dyn Error>> {
        {
            let mut repository_channel = self.repository_channel.lock().await;
            repository_channel
                .store_vouchers(&mut self.metadata, self.vouchers)
                .await?;
            repository_channel
                .store_notices(&mut self.metadata, self.notices)
                .await?;
        }
        Ok(IdleState::new(
            self.repository_channel,
            self.dapp_channel,
            self.current_id,
        ))
    }

    async fn reject(self) -> Result<State, Box<dyn Error>> {
        Ok(IdleState::new(
            self.repository_channel,
            self.dapp_channel,
            self.previous_id,
        ))
    }

    fn add_identified<T>(
        request: SyncRequest<T, u64>,
        vector: &mut Vec<Identified<T>>,
        id: &mut u64,
    ) {
        vector.push(Identified {
            id: *id,
            value: request.value,
        });
        request
            .response_tx
            .send(*id)
            .expect("oneshot channel dropped");
        *id = *id + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::task::JoinHandle;

    #[tokio::test]
    async fn test_it_advance_and_accepts() {
        let (_repository, dapp, mut proxy, _) = setup();
        let request = fake_advance_request();
        dapp.lock()
            .await
            .expect_advance()
            .with(eq(request.clone()))
            .times(2);
        proxy.advance(request).await.unwrap();
        proxy.accept().await.unwrap();
    }

    fn setup() -> (
        Arc<Mutex<MockRepositoryChannel>>,
        Arc<Mutex<MockDAppChannel>>,
        ProxyChannel,
        JoinHandle<()>,
    ) {
        let repository_channel = Arc::new(Mutex::new(MockRepositoryChannel::new()));
        let dapp_channel = Arc::new(Mutex::new(MockDAppChannel::new()));
        let (proxy_channel, proxy_service) = new(repository_channel.clone(), dapp_channel.clone());
        let handle = tokio::spawn(async move {
            proxy_service.run().await.expect("error when running the proxy");
        });
        (
            repository_channel,
            dapp_channel,
            proxy_channel,
            handle,
        )
    }

    fn fake_advance_request() -> AdvanceRequest {
        AdvanceRequest {
            metadata: AdvanceMetadata {
                address: String::from(""),
                epoch_number: 1,
                input_number: 2,
                block_number: 3,
                timestamp: 1635946561,
            },
            payload: String::from("0xdeadbeef"),
        }
    }
}
