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
use mockall::{automock, predicate::*, Sequence};

use async_trait::async_trait;
use std::error::Error;
use tokio::{
    self,
    sync::{mpsc, oneshot},
};

use crate::model::{AdvanceMetadata, AdvanceRequest, InspectRequest, Notice, Report, Voucher};

/// Create the proxy service and the proxy channel.
/// The service should be ran in the background and the channel can be used to communicate with it.
pub fn new(
    repository: Box<dyn Repository + Send>,
    dapp: Box<dyn DApp + Send>,
) -> (ProxyChannel, ProxyService) {
    let (advance_tx, advance_rx) = mpsc::channel::<AdvanceRequest>(1000);
    let (inspect_tx, inspect_rx) = mpsc::channel::<SyncInspectRequest>(1000);
    let (voucher_tx, voucher_rx) = mpsc::channel::<SyncVoucherRequest>(1000);
    let (notice_tx, notice_rx) = mpsc::channel::<SyncNoticeRequest>(1000);
    let (report_tx, report_rx) = mpsc::channel::<Report>(1000);
    let (accept_tx, accept_rx) = mpsc::channel::<()>(1000);
    let (reject_tx, reject_rx) = mpsc::channel::<()>(1000);
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1000);

    let channel = ProxyChannel {
        advance_tx,
        inspect_tx,
        voucher_tx,
        notice_tx,
        report_tx,
        accept_tx,
        reject_tx,
        shutdown_tx,
    };

    let service = ProxyService {
        state: IdleState::new(repository, dapp, 0),
        advance_rx,
        inspect_rx,
        voucher_rx,
        notice_rx,
        report_rx,
        accept_rx,
        reject_rx,
        shutdown_rx,
    };

    (channel, service)
}

/// Channel used to communicate with the DApp
#[cfg_attr(test, automock)]
#[async_trait]
pub trait DApp {
    /// Send an advance request to the client
    async fn advance(&self, request: AdvanceRequest) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Send an inspect request to the client
    async fn inspect(
        &self,
        request: InspectRequest,
    ) -> Result<Vec<Report>, Box<dyn Error + Send + Sync>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identified<T> {
    pub id: u64,
    pub value: T,
}

/// Channel used to communicate with the repository
#[cfg_attr(test, automock)]
#[async_trait]
pub trait Repository {
    /// Send request to store the vouchers
    async fn store_vouchers(
        &self,
        metadata: &AdvanceMetadata,
        vouchers: Vec<Identified<Voucher>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Send request to store the notices
    async fn store_notices(
        &self,
        metadata: &AdvanceMetadata,
        notices: Vec<Identified<Notice>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Send request to store a report
    async fn store_report(
        &self,
        metadata: &AdvanceMetadata,
        report: Report,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
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
    shutdown_tx: mpsc::Sender<()>,
}

impl ProxyChannel {
    /// Send an advance request
    pub async fn advance(
        &self,
        request: AdvanceRequest,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(self.advance_tx.send(request).await?)
    }

    /// Send an inspect request and wait for the report
    pub async fn inspect(
        &self,
        request: InspectRequest,
    ) -> Result<Vec<Report>, Box<dyn Error + Send + Sync>> {
        Self::make_sync_request(request, &self.inspect_tx).await
    }

    /// Send a voucher request
    pub async fn add_voucher(&self, voucher: Voucher) -> Result<u64, Box<dyn Error + Send + Sync>> {
        Self::make_sync_request(voucher, &self.voucher_tx).await
    }

    /// Send a notice request
    pub async fn add_notice(&self, notice: Notice) -> Result<u64, Box<dyn Error + Send + Sync>> {
        Self::make_sync_request(notice, &self.notice_tx).await
    }

    /// Send a report request
    pub async fn add_report(&self, request: Report) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(self.report_tx.send(request).await?)
    }

    /// Send a accept request
    pub async fn accept(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(self.accept_tx.send(()).await?)
    }

    /// Send a reject request
    pub async fn reject(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(self.reject_tx.send(()).await?)
    }

    /// Send a shutdown request
    pub async fn shutdown(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(self.shutdown_tx.send(()).await?)
    }

    async fn make_sync_request<
        T: std::fmt::Debug + Send + Sync + 'static,
        U: std::fmt::Debug + Send + Sync + 'static,
    >(
        value: T,
        tx: &mpsc::Sender<SyncRequest<T, U>>,
    ) -> Result<U, Box<dyn Error + Send + Sync>> {
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
    shutdown_rx: mpsc::Receiver<()>,
}

impl ProxyService {
    pub async fn run(mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        loop {
            self.state = match self.state {
                State::Idle(idle) => {
                    tokio::select! {
                        // prioritize inspect over advance
                        biased;

                        Some(request) = self.inspect_rx.recv() => {
                            idle.inspect(request).await?
                        }
                        Some(request) = self.advance_rx.recv() => {
                            idle.advance(request).await?
                        }
                        Some(_) = self.shutdown_rx.recv() => {
                            return Ok(());
                        }
                    }
                }
                State::Advancing(advancing) => {
                    tokio::select! {
                        // prioritize vouchers/notices/reports over accept/reject
                        biased;

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
                        Some(_) = self.shutdown_rx.recv() => {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct SyncRequest<T: Send + Sync, U: Send + Sync> {
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
    repository: Box<dyn Repository + Send>,
    dapp: Box<dyn DApp + Send>,
    id: u64,
}

impl IdleState {
    fn new(repository: Box<dyn Repository + Send>, dapp: Box<dyn DApp + Send>, id: u64) -> State {
        State::Idle(Self {
            repository,
            dapp,
            id,
        })
    }

    async fn advance(self, request: AdvanceRequest) -> Result<State, Box<dyn Error + Send + Sync>> {
        let metadata = request.metadata.clone();
        self.dapp.advance(request).await?;
        Ok(AdvancingState::new(
            self.repository,
            self.dapp,
            metadata,
            self.id,
        ))
    }

    async fn inspect(
        self,
        request: SyncInspectRequest,
    ) -> Result<State, Box<dyn Error + Send + Sync>> {
        let response = self.dapp.inspect(request.value).await?;
        request
            .response_tx
            .send(response)
            .expect("oneshot channel dropped");
        Ok(State::Idle(self))
    }
}

struct AdvancingState {
    repository: Box<dyn Repository + Send>,
    dapp: Box<dyn DApp + Send>,
    metadata: AdvanceMetadata,
    previous_id: u64,
    current_id: u64,
    vouchers: Vec<Identified<Voucher>>,
    notices: Vec<Identified<Notice>>,
}

impl AdvancingState {
    fn new(
        repository: Box<dyn Repository + Send>,
        dapp: Box<dyn DApp + Send>,
        metadata: AdvanceMetadata,
        id: u64,
    ) -> State {
        State::Advancing(Self {
            repository,
            dapp,
            metadata,
            previous_id: id,
            current_id: id,
            vouchers: vec![],
            notices: vec![],
        })
    }

    async fn add_voucher(
        mut self,
        request: SyncVoucherRequest,
    ) -> Result<State, Box<dyn Error + Send + Sync>> {
        Self::add_identified(request, &mut self.vouchers, &mut self.current_id);
        Ok(State::Advancing(self))
    }

    async fn add_notice(
        mut self,
        request: SyncNoticeRequest,
    ) -> Result<State, Box<dyn Error + Send + Sync>> {
        Self::add_identified(request, &mut self.notices, &mut self.current_id);
        Ok(State::Advancing(self))
    }

    async fn add_report(self, report: Report) -> Result<State, Box<dyn Error + Send + Sync>> {
        self.repository.store_report(&self.metadata, report).await?;
        Ok(State::Advancing(self))
    }

    async fn accept(self) -> Result<State, Box<dyn Error + Send + Sync>> {
        if !self.vouchers.is_empty() {
            self.repository
                .store_vouchers(&self.metadata, self.vouchers)
                .await?;
        }
        if !self.notices.is_empty() {
            self.repository
                .store_notices(&self.metadata, self.notices)
                .await?;
        }
        Ok(IdleState::new(self.repository, self.dapp, self.current_id))
    }

    async fn reject(self) -> Result<State, Box<dyn Error + Send + Sync>> {
        Ok(IdleState::new(self.repository, self.dapp, self.previous_id))
    }

    fn add_identified<T: Send + Sync>(
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
    use std::sync::Arc;
    use tokio::sync::{Notify, Semaphore};

    #[tokio::test]
    async fn test_it_sends_an_advance_request() {
        let notify = Arc::new(Notify::new());
        let request = fake_advance_request();
        let repository = Box::new(MockRepository::new());
        let mut dapp = Box::new(MockDApp::new());
        {
            let notify = notify.clone();
            dapp.expect_advance()
                .times(1)
                .with(eq(request.clone()))
                .return_once(move |_| {
                    notify.notify_one();
                    Ok(())
                });
        }
        let (mut proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        proxy.advance(request).await.unwrap();
        notify.notified().await;
        proxy.shutdown().await.unwrap();
        service.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_it_sends_an_inspect_request() {
        let request = InspectRequest {
            payload: String::from("0xdeadbeef"),
        };
        let expected = vec![
            Report {
                payload: String::from("result1"),
            },
            Report {
                payload: String::from("result2"),
            },
        ];
        let repository = Box::new(MockRepository::new());
        let mut dapp = Box::new(MockDApp::new());
        {
            let response = expected.clone();
            dapp.expect_inspect()
                .times(1)
                .with(eq(request.clone()))
                .return_once(move |_| Ok(response));
        }
        let (mut proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        let got = proxy.inspect(request).await.unwrap();
        assert!(got == expected);
        proxy.shutdown().await.unwrap();
        service.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_it_processes_multiple_advance_requests() {
        let n = 3;
        let request = fake_advance_request();
        let semaphore = Arc::new(Semaphore::new(0));
        let repository = Box::new(MockRepository::new());
        let mut dapp = Box::new(MockDApp::new());
        {
            let semaphore = semaphore.clone();
            dapp.expect_advance()
                .times(3)
                .with(eq(request.clone()))
                .returning(move |_| {
                    semaphore.add_permits(1);
                    Ok(())
                });
        }
        let (mut proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        for _ in 0..n {
            proxy.advance(request.clone()).await.unwrap();
            proxy.accept().await.unwrap();
        }
        let _ = semaphore.acquire_many(n).await.unwrap();
        assert_eq!(semaphore.available_permits(), 3);
        proxy.shutdown().await.unwrap();
        service.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_it_prioritizes_inspect_requests_over_advance_requests() {
        let advance_request = fake_advance_request();
        let inspect_request = InspectRequest {
            payload: String::from("0xdeadbeef"),
        };
        let advance_notify = &[Arc::new(Notify::new()), Arc::new(Notify::new())];
        let repository = Box::new(MockRepository::new());
        let mut dapp = Box::new(MockDApp::new());
        {
            let advance_notify_0 = advance_notify[0].clone();
            let advance_notify_1 = advance_notify[1].clone();
            let mut sequence = Sequence::new();
            dapp.expect_advance()
                .times(1)
                .in_sequence(&mut sequence)
                .returning(move |_| {
                    advance_notify_0.notify_one();
                    Ok(())
                });
            dapp.expect_inspect()
                .times(1)
                .in_sequence(&mut sequence)
                .returning(|_| Ok(vec![]));
            dapp.expect_advance()
                .times(1)
                .in_sequence(&mut sequence)
                .returning(move |_| {
                    advance_notify_1.notify_one();
                    Ok(())
                });
        }
        let (mut proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        proxy.advance(advance_request.clone()).await.unwrap();
        advance_notify[0].notified().await;
        // Send both an advance and an inspect while processing the first advance
        proxy.advance(advance_request.clone()).await.unwrap();
        let inspect_handle = {
            let mut proxy = proxy.clone();
            tokio::spawn(async move { proxy.inspect(inspect_request).await.unwrap() })
        };
        proxy.accept().await.unwrap(); // Finish processing the first advance
        inspect_handle.await.unwrap(); // The inspect comes first
        proxy.accept().await.unwrap(); // Then comes the second advance
        advance_notify[1].notified().await;
        proxy.shutdown().await.unwrap();
        service.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_it_stores_vouchers_in_repository_when_accepted() {
        let request = fake_advance_request();
        let notify = Arc::new(Notify::new());
        let vouchers = vec![
            Identified {
                id: 0,
                value: Voucher {
                    address: String::from("0x0001"),
                    payload: String::from("voucher 0"),
                },
            },
            Identified {
                id: 1,
                value: Voucher {
                    address: String::from("0x0002"),
                    payload: String::from("voucher 1"),
                },
            },
        ];
        let mut repository = Box::new(MockRepository::new());
        repository
            .expect_store_vouchers()
            .times(1)
            .with(eq(request.metadata.clone()), eq(vouchers.clone()))
            .return_once(|_, _| Ok(()));
        let mut dapp = Box::new(MockDApp::new());
        {
            let notify = notify.clone();
            dapp.expect_advance().times(1).return_once(move |_| {
                notify.notify_one();
                Ok(())
            });
        }
        let (mut proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        proxy.advance(request).await.unwrap();
        notify.notified().await;
        for voucher in vouchers {
            proxy.add_voucher(voucher.value.clone()).await.unwrap();
        }
        proxy.accept().await.unwrap();
        proxy.shutdown().await.unwrap();
        service.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_it_store_notices_in_repository_when_accepted() {
        let request = fake_advance_request();
        let notify = Arc::new(Notify::new());
        let notices = vec![
            Identified {
                id: 0,
                value: Notice {
                    payload: String::from("notice 0"),
                },
            },
            Identified {
                id: 1,
                value: Notice {
                    payload: String::from("notice 1"),
                },
            },
        ];
        let mut repository = Box::new(MockRepository::new());
        repository
            .expect_store_notices()
            .times(1)
            .with(eq(request.metadata.clone()), eq(notices.clone()))
            .return_once(|_, _| Ok(()));
        let mut dapp = Box::new(MockDApp::new());
        {
            let notify = notify.clone();
            dapp.expect_advance().times(1).return_once(move |_| {
                notify.notify_one();
                Ok(())
            });
        }
        let (mut proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        proxy.advance(request.clone()).await.unwrap();
        notify.notified().await;
        for notice in notices {
            proxy.add_notice(notice.value.clone()).await.unwrap();
        }
        proxy.accept().await.unwrap();
        proxy.shutdown().await.unwrap();
        service.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_it_store_reports_in_repository_when_advancing() {
        let request = fake_advance_request();
        let notify = Arc::new(Notify::new());
        let report = Report {
            payload: String::from("0xdeadbeef"),
        };
        let mut repository = Box::new(MockRepository::new());
        repository
            .expect_store_report()
            .times(1)
            .with(eq(request.metadata.clone()), eq(report.clone()))
            .return_once(|_, _| Ok(()));
        let mut dapp = Box::new(MockDApp::new());
        {
            let notify = notify.clone();
            dapp.expect_advance().times(1).return_once(move |_| {
                notify.notify_one();
                Ok(())
            });
        }
        let (mut proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        proxy.advance(request.clone()).await.unwrap();
        notify.notified().await;
        proxy.add_report(report).await.unwrap();
        proxy.accept().await.unwrap();
        proxy.shutdown().await.unwrap();
        service.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_it_discard_notices_and_vouchers_when_rejected() {
        let request = fake_advance_request();
        let voucher = Voucher {
            address: String::from("0x0001"),
            payload: String::from("voucher 0"),
        };
        let notice = Notice {
            payload: String::from("notice 0"),
        };
        let notify = Arc::new(Notify::new());
        let repository = Box::new(MockRepository::new());
        let mut dapp = Box::new(MockDApp::new());
        {
            let notify = notify.clone();
            dapp.expect_advance().times(1).return_once(move |_| {
                notify.notify_one();
                Ok(())
            });
        }
        let (mut proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        proxy.advance(request).await.unwrap();
        notify.notified().await;
        proxy.add_voucher(voucher).await.unwrap();
        proxy.add_notice(notice).await.unwrap();
        proxy.reject().await.unwrap(); // changing to accept makes the test fail as expected
        proxy.shutdown().await.unwrap();
        service.await.unwrap().unwrap();
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
