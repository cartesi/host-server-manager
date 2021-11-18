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
use std::{error::Error, fmt};
use tokio::sync::{mpsc, oneshot};

use crate::model::{AdvanceMetadata, AdvanceRequest, InspectRequest, Notice, Report, Voucher};

/// Create the proxy service and the proxy channel.
/// The service should be ran in the background and the channel can be used to communicate with it.
pub fn new(
    repository: Box<dyn Repository + Send + Sync>,
    dapp: Box<dyn DApp + Send + Sync>,
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
    pub async fn advance(&self, request: AdvanceRequest) {
        self.advance_tx
            .send(request)
            .await
            .expect("send should not fail")
    }

    /// Send an inspect request and wait for the report
    pub async fn inspect(&self, request: InspectRequest) -> Result<Vec<Report>, InspectError> {
        Self::make_sync_request(request, &self.inspect_tx).await
    }

    /// Send a voucher request
    pub async fn insert_voucher(&self, voucher: Voucher) -> Result<u64, InsertError> {
        Self::make_sync_request(voucher, &self.voucher_tx).await
    }

    /// Send a notice request
    pub async fn insert_notice(&self, notice: Notice) -> Result<u64, InsertError> {
        Self::make_sync_request(notice, &self.notice_tx).await
    }

    /// Send a report request
    pub async fn insert_report(&self, request: Report) {
        self.report_tx
            .send(request)
            .await
            .expect("send should not fail")
    }

    /// Send a accept request
    pub async fn accept(&self) {
        self.accept_tx.send(()).await.expect("send should not fail")
    }

    /// Send a reject request
    pub async fn reject(&self) {
        self.reject_tx.send(()).await.expect("send should not fail")
    }

    /// Send a shutdown request
    pub async fn shutdown(&self) {
        self.shutdown_tx
            .send(())
            .await
            .expect("send should not fail")
    }

    async fn make_sync_request<
        T: std::fmt::Debug + Send + Sync + 'static,
        U: std::fmt::Debug + Send + Sync + 'static,
    >(
        value: T,
        tx: &mpsc::Sender<SyncRequest<T, U>>,
    ) -> U {
        let (response_tx, response_rx) = oneshot::channel();
        tx.send(SyncRequest { value, response_tx })
            .await
            .expect("send should not fail");
        response_rx.await.expect("receive should not fail")
    }
}

#[derive(Debug)]
pub struct InsertError();
impl Error for InsertError {}
impl fmt::Display for InsertError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "request not accepted when state is idle")
    }
}

#[derive(Debug)]
pub struct InspectError();
impl Error for InspectError {}
impl fmt::Display for InspectError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "failed to send inspect request to dapp")
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
    pub async fn run(mut self) {
        loop {
            self.state = match self.state {
                State::Idle(idle) => {
                    tokio::select! {
                        biased;

                        Some(request) = self.voucher_rx.recv() => {
                            idle.insert_voucher(request)
                        }
                        Some(request) = self.notice_rx.recv() => {
                            idle.insert_notice(request)
                        }
                        Some(_) = self.report_rx.recv() => {
                            idle.insert_report()
                        }
                        Some(_) = self.accept_rx.recv() => {
                            idle.accept()
                        }
                        Some(_) = self.reject_rx.recv() => {
                            idle.reject()
                        }
                        Some(request) = self.inspect_rx.recv() => {
                            idle.inspect(request).await
                        }
                        Some(request) = self.advance_rx.recv() => {
                            idle.advance(request).await
                        }
                        Some(_) = self.shutdown_rx.recv() => {
                            return;
                        }
                    }
                }
                State::Advancing(advancing) => {
                    tokio::select! {
                        biased;

                        // Do not handle inspect and advance requests when advancing

                        Some(request) = self.voucher_rx.recv() => {
                            advancing.insert_voucher(request).await
                        }
                        Some(request) = self.notice_rx.recv() => {
                            advancing.insert_notice(request).await
                        }
                        Some(report) = self.report_rx.recv() => {
                            advancing.insert_report(report).await
                        }
                        Some(_) = self.accept_rx.recv() => {
                            advancing.accept().await
                        }
                        Some(_) = self.reject_rx.recv() => {
                            advancing.reject().await
                        }
                        Some(_) = self.shutdown_rx.recv() => {
                            return;
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

type SyncInspectRequest = SyncRequest<InspectRequest, Result<Vec<Report>, InspectError>>;
type SyncVoucherRequest = SyncRequest<Voucher, Result<u64, InsertError>>;
type SyncNoticeRequest = SyncRequest<Notice, Result<u64, InsertError>>;

enum State {
    Idle(IdleState),
    Advancing(AdvancingState),
}

struct IdleState {
    repository: Box<dyn Repository + Send + Sync>,
    dapp: Box<dyn DApp + Send + Sync>,
    id: u64,
}

impl IdleState {
    fn new(
        repository: Box<dyn Repository + Send + Sync>,
        dapp: Box<dyn DApp + Send + Sync>,
        id: u64,
    ) -> State {
        State::Idle(Self {
            repository,
            dapp,
            id,
        })
    }

    fn insert_voucher(self, request: SyncVoucherRequest) -> State {
        log::warn!("ignoring voucher when idle");
        Self::reject_request(request);
        State::Idle(self)
    }

    fn insert_notice(self, request: SyncNoticeRequest) -> State {
        log::warn!("ignoring notice when idle");
        Self::reject_request(request);
        State::Idle(self)
    }

    fn insert_report(self) -> State {
        log::warn!("ignoring report when idle");
        State::Idle(self)
    }

    fn accept(self) -> State {
        log::warn!("ignoring accept when idle");
        State::Idle(self)
    }

    fn reject(self) -> State {
        log::warn!("ignoring reject when idle");
        State::Idle(self)
    }

    async fn advance(self, request: AdvanceRequest) -> State {
        let metadata = request.metadata.clone();
        match self.dapp.advance(request).await {
            Ok(_) => {
                log::info!("setting state to advance");
                AdvancingState::new(self.repository, self.dapp, metadata, self.id)
            }
            Err(e) => {
                log::error!("failed to advance state with error: {}", e);
                State::Idle(self)
            }
        }
    }

    async fn inspect(self, request: SyncInspectRequest) -> State {
        log::info!("processing inspect request");
        let response = self.dapp.inspect(request.value).await.map_err(|e| {
            log::error!("failed to inspect with error: {}", e);
            InspectError()
        });
        request
            .response_tx
            .send(response)
            .expect("send should not fail");
        State::Idle(self)
    }

    fn reject_request<T: Send + Sync>(request: SyncRequest<T, Result<u64, InsertError>>) {
        request
            .response_tx
            .send(Err(InsertError()))
            .expect("send should not fail");
    }
}

struct AdvancingState {
    repository: Box<dyn Repository + Send + Sync>,
    dapp: Box<dyn DApp + Send + Sync>,
    metadata: AdvanceMetadata,
    previous_id: u64,
    current_id: u64,
    vouchers: Vec<Identified<Voucher>>,
    notices: Vec<Identified<Notice>>,
}

impl AdvancingState {
    fn new(
        repository: Box<dyn Repository + Send + Sync>,
        dapp: Box<dyn DApp + Send + Sync>,
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

    async fn insert_voucher(mut self, request: SyncVoucherRequest) -> State {
        log::info!("processing voucher request");
        Self::insert_identified(request, &mut self.vouchers, &mut self.current_id);
        State::Advancing(self)
    }

    async fn insert_notice(mut self, request: SyncNoticeRequest) -> State {
        log::info!("processing notice request");
        Self::insert_identified(request, &mut self.notices, &mut self.current_id);
        State::Advancing(self)
    }

    async fn insert_report(self, report: Report) -> State {
        log::info!("processing report request");
        if let Err(e) = self.repository.store_report(&self.metadata, report).await {
            log::error!("failed to store report with error: {}", e);
        }
        State::Advancing(self)
    }

    async fn accept(self) -> State {
        log::info!("processing accept request; setting state to idle");
        if !self.vouchers.is_empty() {
            if let Err(e) = self
                .repository
                .store_vouchers(&self.metadata, self.vouchers)
                .await
            {
                log::error!("failed to store vouchers with error: {}", e);
            }
        }
        if !self.notices.is_empty() {
            if let Err(e) = self
                .repository
                .store_notices(&self.metadata, self.notices)
                .await
            {
                log::error!("failed to store notices with error: {}", e);
            }
        }
        IdleState::new(self.repository, self.dapp, self.current_id)
    }

    async fn reject(self) -> State {
        log::info!("processing reject request; setting state to idle");
        IdleState::new(self.repository, self.dapp, self.previous_id)
    }

    fn insert_identified<T: Send + Sync>(
        request: SyncRequest<T, Result<u64, InsertError>>,
        vector: &mut Vec<Identified<T>>,
        id: &mut u64,
    ) {
        vector.push(Identified {
            id: *id,
            value: request.value,
        });
        request
            .response_tx
            .send(Ok(*id))
            .expect("send should not fail");
        *id = *id + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Notify;

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
        let (proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        proxy.advance(request).await;
        notify.notified().await;
        proxy.shutdown().await;
        service.await.unwrap();
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
        let (proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        let got = proxy.inspect(request).await.unwrap();
        assert!(got == expected);
        proxy.shutdown().await;
        service.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_processes_multiple_advance_requests() {
        let request = fake_advance_request();
        let notifies = vec![
            Arc::new(Notify::new()),
            Arc::new(Notify::new()),
            Arc::new(Notify::new()),
        ];
        let repository = Box::new(MockRepository::new());
        let mut dapp = Box::new(MockDApp::new());
        let mut sequence = Sequence::new();
        for notify in &notifies {
            let notify = notify.clone();
            dapp.expect_advance()
                .times(1)
                .in_sequence(&mut sequence)
                .with(eq(request.clone()))
                .returning(move |_| {
                    notify.notify_one();
                    Ok(())
                });
        }
        let (proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        for notify in notifies {
            proxy.advance(request.clone()).await;
            notify.notified().await;
            proxy.accept().await;
        }
        proxy.shutdown().await;
        service.await.unwrap();
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
        let (proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        proxy.advance(advance_request.clone()).await;
        advance_notify[0].notified().await;
        // Send both an advance and an inspect while processing the first advance
        proxy.advance(advance_request.clone()).await;
        let inspect_handle = {
            let proxy = proxy.clone();
            tokio::spawn(async move { proxy.inspect(inspect_request).await.unwrap() })
        };
        proxy.accept().await; // Accept the first advance
        inspect_handle.await.unwrap(); // Wait for the inspect to finish first
        advance_notify[1].notified().await; // Then wait for the second advance
        proxy.accept().await; // Finally, accept the second advance
        proxy.shutdown().await;
        service.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_rejects_vouchers_and_notices_when_idle() {
        let repository = Box::new(MockRepository::new());
        let dapp = Box::new(MockDApp::new());
        let (proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        let result = proxy
            .insert_notice(Notice {
                payload: String::from("notice 0"),
            })
            .await;
        assert!(matches!(result, Err(InsertError())));
        let result = proxy
            .insert_voucher(Voucher {
                address: String::from("0x0001"),
                payload: String::from("voucher 0"),
            })
            .await;
        assert!(matches!(result, Err(InsertError())));
        proxy.shutdown().await;
        service.await.unwrap();
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
        let (proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        proxy.advance(request).await;
        notify.notified().await;
        for voucher in vouchers {
            proxy.insert_voucher(voucher.value.clone()).await.unwrap();
        }
        proxy.accept().await;
        proxy.shutdown().await;
        service.await.unwrap();
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
        let (proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        proxy.advance(request.clone()).await;
        notify.notified().await;
        for notice in notices {
            proxy.insert_notice(notice.value.clone()).await.unwrap();
        }
        proxy.accept().await;
        proxy.shutdown().await;
        service.await.unwrap();
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
        let (proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        proxy.advance(request.clone()).await;
        notify.notified().await;
        proxy.insert_report(report).await;
        proxy.accept().await;
        proxy.shutdown().await;
        service.await.unwrap();
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
        // The test passes only because the repository mock expects nothing
        let repository = Box::new(MockRepository::new());
        let mut dapp = Box::new(MockDApp::new());
        {
            let notify = notify.clone();
            dapp.expect_advance().times(1).return_once(move |_| {
                notify.notify_one();
                Ok(())
            });
        }
        let (proxy, service) = new(repository, dapp);
        let service = tokio::spawn(service.run());

        proxy.advance(request).await;
        notify.notified().await;
        proxy.insert_voucher(voucher).await.unwrap();
        proxy.insert_notice(notice).await.unwrap();
        proxy.reject().await;
        proxy.shutdown().await;
        service.await.unwrap();
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
