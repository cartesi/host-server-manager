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
use snafu::Snafu;
use std::{error::Error, fmt};
use tokio::sync::{mpsc, oneshot};

use crate::model::{
    AdvanceRequest, FinishStatus, Identified, Input, InspectRequest, Notice, Report, Voucher,
};

/// Create the proxy service and the proxy channel.
/// The service should be ran in the background and the channel can be used to communicate with it.
pub fn new(dapp: Box<dyn DApp>) -> (ProxyChannel, ProxyService) {
    let (advance_tx, advance_rx) = mpsc::channel::<AdvanceRequestWrapper>(1000);
    let (inspect_tx, inspect_rx) = mpsc::channel::<SyncInspectRequest>(1000);
    let (voucher_tx, voucher_rx) = mpsc::channel::<SyncVoucherRequest>(1000);
    let (notice_tx, notice_rx) = mpsc::channel::<SyncNoticeRequest>(1000);
    let (report_tx, report_rx) = mpsc::channel::<Report>(1000);
    let (finish_tx, finish_rx) = mpsc::channel::<FinishStatus>(1000);
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1000);

    let channel = ProxyChannel {
        advance_tx,
        inspect_tx,
        voucher_tx,
        notice_tx,
        report_tx,
        finish_tx,
        shutdown_tx,
    };

    let service = ProxyService {
        state: IdleState::new(dapp, 0),
        advance_rx,
        inspect_rx,
        voucher_rx,
        notice_rx,
        report_rx,
        finish_rx,
        shutdown_rx,
    };

    (channel, service)
}

/// Channel used to communicate with the DApp
#[cfg_attr(test, automock)]
#[async_trait]
pub trait DApp: Send + Sync {
    /// Send an advance request to the client
    async fn advance(&self, request: AdvanceRequest) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Send an inspect request to the client
    async fn inspect(
        &self,
        request: InspectRequest,
    ) -> Result<Vec<Report>, Box<dyn Error + Send + Sync>>;
}

#[derive(Debug, Snafu)]
#[snafu(display("request not accepted when state is idle"))]
pub struct InsertError {}

#[derive(Debug, Snafu)]
#[snafu(display("failed to send inspect request to dapp: {}", e))]
pub struct InspectError {
    e: Box<dyn Error + Send + Sync>,
}

#[derive(Debug, Snafu)]
#[snafu(display("failed to send advance request to dapp: {}", e))]
pub struct AdvanceError {
    e: Box<dyn Error + Send + Sync>,
}

/// Channel used to communicate with the Proxy
#[derive(Clone)]
pub struct ProxyChannel {
    advance_tx: mpsc::Sender<AdvanceRequestWrapper>,
    inspect_tx: mpsc::Sender<SyncInspectRequest>,
    voucher_tx: mpsc::Sender<SyncVoucherRequest>,
    notice_tx: mpsc::Sender<SyncNoticeRequest>,
    report_tx: mpsc::Sender<Report>,
    finish_tx: mpsc::Sender<FinishStatus>,
    shutdown_tx: mpsc::Sender<()>,
}

/// Callback object that will be called when advance if finished
/// This could be callback, but async closures are unstable at the current version of Rust
/// https://github.com/rust-lang/rust/issues/62290
#[cfg_attr(test, automock)]
#[async_trait]
pub trait AdvanceFinisher: fmt::Debug + Send + Sync {
    async fn handle(&self, result: Result<Input, AdvanceError>);
}

impl ProxyChannel {
    /// Send an advance request
    /// The request will run in the background and the callback will be called when it is finished.
    pub async fn advance(&self, request: AdvanceRequest, finisher: Box<dyn AdvanceFinisher>) {
        self.advance_tx
            .send(AdvanceRequestWrapper { request, finisher })
            .await
            .expect("send should not fail");
    }

    /// Send an inspect request and wait for the report
    /// The function only returns when the request is complete.
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

    /// Send a finish request
    pub async fn finish(&self, status: FinishStatus) {
        self.finish_tx
            .send(status)
            .await
            .expect("send should not fail")
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

/// Service that processes the requests and should be ran in the background
pub struct ProxyService {
    state: State,
    advance_rx: mpsc::Receiver<AdvanceRequestWrapper>,
    inspect_rx: mpsc::Receiver<SyncInspectRequest>,
    voucher_rx: mpsc::Receiver<SyncVoucherRequest>,
    notice_rx: mpsc::Receiver<SyncNoticeRequest>,
    report_rx: mpsc::Receiver<Report>,
    finish_rx: mpsc::Receiver<FinishStatus>,
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
                        Some(_) = self.finish_rx.recv() => {
                            idle.finish()
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
                        Some(status) = self.finish_rx.recv() => {
                            advancing.finish(status).await
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
struct AdvanceRequestWrapper {
    request: AdvanceRequest,
    finisher: Box<dyn AdvanceFinisher>,
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
    dapp: Box<dyn DApp>,
    id: u64,
}

impl IdleState {
    fn new(dapp: Box<dyn DApp>, id: u64) -> State {
        State::Idle(Self { dapp, id })
    }

    fn insert_voucher(self, request: SyncVoucherRequest) -> State {
        log::warn!("ignoring voucher when idle");
        Self::reject_insert_request(request);
        State::Idle(self)
    }

    fn insert_notice(self, request: SyncNoticeRequest) -> State {
        log::warn!("ignoring notice when idle");
        Self::reject_insert_request(request);
        State::Idle(self)
    }

    fn insert_report(self) -> State {
        log::warn!("ignoring report when idle");
        State::Idle(self)
    }

    fn finish(self) -> State {
        log::warn!("ignoring finish when idle");
        State::Idle(self)
    }

    async fn advance(self, wrapper: AdvanceRequestWrapper) -> State {
        match self.dapp.advance(wrapper.request).await {
            Ok(_) => {
                log::info!("setting state to advance");
                AdvancingState::new(self.dapp, self.id, wrapper.finisher)
            }
            Err(e) => {
                log::error!("failed to advance state with error: {}", e);
                wrapper.finisher.handle(Err(AdvanceError { e })).await;
                State::Idle(self)
            }
        }
    }

    async fn inspect(self, request: SyncInspectRequest) -> State {
        log::info!("processing inspect request");
        let response = self.dapp.inspect(request.value).await.map_err(|e| {
            log::error!("failed to inspect with error: {}", e);
            InspectError { e }
        });
        request
            .response_tx
            .send(response)
            .expect("send should not fail");
        State::Idle(self)
    }

    fn reject_insert_request<T: Send + Sync>(request: SyncRequest<T, Result<u64, InsertError>>) {
        request
            .response_tx
            .send(Err(InsertError {}))
            .expect("send should not fail");
    }
}

struct AdvancingState {
    dapp: Box<dyn DApp>,
    id: u64,
    finisher: Box<dyn AdvanceFinisher>,
    vouchers: Vec<Identified<Voucher>>,
    notices: Vec<Identified<Notice>>,
    reports: Vec<Report>,
}

impl AdvancingState {
    fn new(dapp: Box<dyn DApp>, id: u64, finisher: Box<dyn AdvanceFinisher>) -> State {
        State::Advancing(Self {
            dapp,
            id,
            finisher,
            vouchers: vec![],
            notices: vec![],
            reports: vec![],
        })
    }

    async fn insert_voucher(mut self, request: SyncVoucherRequest) -> State {
        log::info!("processing voucher request");
        Self::insert_identified(request, &mut self.vouchers, &mut self.id);
        State::Advancing(self)
    }

    async fn insert_notice(mut self, request: SyncNoticeRequest) -> State {
        log::info!("processing notice request");
        Self::insert_identified(request, &mut self.notices, &mut self.id);
        State::Advancing(self)
    }

    async fn insert_report(mut self, report: Report) -> State {
        log::info!("processing report request");
        self.reports.push(report);
        State::Advancing(self)
    }

    async fn finish(self, status: FinishStatus) -> State {
        log::info!("processing finish request; setting state to idle");
        let input = Input {
            status,
            vouchers: self.vouchers,
            notices: self.notices,
            reports: self.reports,
        };
        self.finisher.handle(Ok(input)).await;
        IdleState::new(self.dapp, self.id)
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
    use crate::model::AdvanceMetadata;
    use std::sync::Arc;
    use tokio::sync::Notify;

    #[tokio::test]
    async fn test_it_sends_an_advance_request() {
        let notify = Arc::new(Notify::new());
        let request = mock_advance_request();
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
        let (proxy, service) = new(dapp);
        let service = tokio::spawn(service.run());

        proxy
            .advance(request, mock_finisher(FinishStatus::Accept))
            .await;
        notify.notified().await;
        proxy.finish(FinishStatus::Accept).await;
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
        let mut dapp = Box::new(MockDApp::new());
        {
            let response = expected.clone();
            dapp.expect_inspect()
                .times(1)
                .with(eq(request.clone()))
                .return_once(move |_| Ok(response));
        }
        let (proxy, service) = new(dapp);
        let service = tokio::spawn(service.run());

        let got = proxy.inspect(request).await.unwrap();
        assert!(got == expected);
        proxy.shutdown().await;
        service.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_processes_multiple_advance_requests() {
        let request = mock_advance_request();
        let notifies = vec![
            Arc::new(Notify::new()),
            Arc::new(Notify::new()),
            Arc::new(Notify::new()),
        ];
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
        let (proxy, service) = new(dapp);
        let service = tokio::spawn(service.run());

        for notify in notifies {
            proxy
                .advance(request.clone(), mock_finisher(FinishStatus::Accept))
                .await;
            notify.notified().await;
            proxy.finish(FinishStatus::Accept).await;
        }
        proxy.shutdown().await;
        service.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_prioritizes_inspect_requests_over_advance_requests() {
        let advance_request = mock_advance_request();
        let inspect_request = InspectRequest {
            payload: String::from("0xdeadbeef"),
        };
        let advance_notify = &[Arc::new(Notify::new()), Arc::new(Notify::new())];
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
        let (proxy, service) = new(dapp);
        let service = tokio::spawn(service.run());

        proxy
            .advance(advance_request.clone(), mock_finisher(FinishStatus::Accept))
            .await;
        advance_notify[0].notified().await;
        // Send both an advance and an inspect while processing the first advance
        proxy
            .advance(advance_request, mock_finisher(FinishStatus::Accept))
            .await;
        let inspect_handle = {
            let proxy = proxy.clone();
            tokio::spawn(async move { proxy.inspect(inspect_request).await.unwrap() })
        };
        proxy.finish(FinishStatus::Accept).await; // Accept the first advance
        inspect_handle.await.unwrap(); // Wait for the inspect to finish first
        advance_notify[1].notified().await; // Then wait for the second advance
        proxy.finish(FinishStatus::Accept).await; // Finally, accept the second advance
        proxy.shutdown().await;
        service.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_rejects_vouchers_and_notices_when_idle() {
        let dapp = Box::new(MockDApp::new());
        let (proxy, service) = new(dapp);
        let service = tokio::spawn(service.run());

        let result = proxy
            .insert_notice(Notice {
                payload: String::from("notice 0"),
            })
            .await;
        assert!(matches!(result, Err(InsertError {})));
        let result = proxy
            .insert_voucher(Voucher {
                address: String::from("0x0001"),
                payload: String::from("voucher 0"),
            })
            .await;
        assert!(matches!(result, Err(InsertError {})));
        proxy.shutdown().await;
        service.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_sends_vouchers_notices_and_reports_to_finisher() {
        let request = mock_advance_request();
        let notify = Arc::new(Notify::new());
        let input = Input {
            status: FinishStatus::Reject,
            vouchers: vec![
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
            ],
            notices: vec![
                Identified {
                    id: 2,
                    value: Notice {
                        payload: String::from("notice 2"),
                    },
                },
                Identified {
                    id: 3,
                    value: Notice {
                        payload: String::from("notice 3"),
                    },
                },
            ],
            reports: vec![Report {
                payload: String::from("0xdeadbeef"),
            }],
        };
        let mut finisher = Box::new(MockAdvanceFinisher::new());
        finisher
            .expect_handle()
            .times(1)
            .with(eq(Ok(input.clone())))
            .return_once(|_| ());
        let mut dapp = Box::new(MockDApp::new());
        {
            let notify = notify.clone();
            dapp.expect_advance().times(1).return_once(move |_| {
                notify.notify_one();
                Ok(())
            });
        }
        let (proxy, service) = new(dapp);
        let service = tokio::spawn(service.run());

        proxy.advance(request, finisher).await;
        notify.notified().await;
        for voucher in input.vouchers {
            proxy.insert_voucher(voucher.value.clone()).await.unwrap();
        }
        for notice in input.notices {
            proxy.insert_notice(notice.value.clone()).await.unwrap();
        }
        for report in input.reports {
            proxy.insert_report(report).await;
        }
        proxy.finish(FinishStatus::Reject).await;
        proxy.shutdown().await;
        service.await.unwrap();
    }

    fn mock_input(status: FinishStatus) -> Input {
        Input {
            status,
            vouchers: vec![],
            notices: vec![],
            reports: vec![],
        }
    }

    fn mock_finisher(status: FinishStatus) -> Box<dyn AdvanceFinisher> {
        let mut finisher = Box::new(MockAdvanceFinisher::new());
        finisher
            .expect_handle()
            .times(1)
            .with(eq(Ok(mock_input(status))))
            .return_once(|_| ());
        finisher
    }

    fn mock_advance_request() -> AdvanceRequest {
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

    // This is required to use the eq predicate in the finisher mock
    impl PartialEq for AdvanceError {
        fn eq(&self, rhs: &Self) -> bool {
            self.e.to_string() == rhs.e.to_string()
        }
    }
}
