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
use tokio::sync::{mpsc, oneshot};

use crate::model::{
    AdvanceRequest, AdvanceResult, FinishStatus, Identified, InspectRequest, Notice, Report,
    Voucher,
};

const BUFFER_SIZE: usize = 1000;

/// Create the controller and its background service
pub fn new<D: DApp>(dapp: D) -> (Controller<D>, ControllerService<D>) {
    let (advance_tx, advance_rx) = mpsc::channel::<AdvanceRequestWrapper<D>>(BUFFER_SIZE);
    let (inspect_tx, inspect_rx) = mpsc::channel::<SyncInspectRequest<D::Error>>(BUFFER_SIZE);
    let (voucher_tx, voucher_rx) = mpsc::channel::<SyncVoucherRequest>(BUFFER_SIZE);
    let (notice_tx, notice_rx) = mpsc::channel::<SyncNoticeRequest>(BUFFER_SIZE);
    let (report_tx, report_rx) = mpsc::channel::<Report>(BUFFER_SIZE);
    let (finish_tx, finish_rx) = mpsc::channel::<FinishStatus>(BUFFER_SIZE);
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(BUFFER_SIZE);

    let controller = Controller {
        advance_tx,
        inspect_tx,
        voucher_tx,
        notice_tx,
        report_tx,
        finish_tx,
        shutdown_tx,
    };

    let service = ControllerService {
        state: IdleState::new(dapp, 0),
        advance_rx,
        inspect_rx,
        voucher_rx,
        notice_rx,
        report_rx,
        finish_rx,
        shutdown_rx,
    };

    (controller, service)
}

/// Channel used to communicate with the DApp
#[cfg_attr(test, automock(type Error=MockDAppError;))]
#[async_trait]
pub trait DApp: std::fmt::Debug + Send + Sync {
    type Error: std::error::Error + Send + Sync;

    /// Send an advance request to the client
    async fn advance(&self, request: AdvanceRequest) -> Result<(), Self::Error>;

    /// Send an inspect request to the client
    async fn inspect(&self, request: InspectRequest) -> Result<Vec<Report>, Self::Error>;
}

#[cfg(test)]
#[derive(Debug, Snafu)]
#[snafu(display("mock dapp error"))]
pub struct MockDAppError {}

#[derive(Debug, Snafu)]
#[snafu(display("request not accepted when state is idle"))]
pub struct InsertError {}

/// State-machine that controls whether the Mock is processing an input (advancing the rollups
/// epoch state) or idle.
/// When processing an input, the Mock receives vouchers, notices, and reports until it receives a
/// finish request.
pub struct Controller<D: DApp> {
    advance_tx: mpsc::Sender<AdvanceRequestWrapper<D>>,
    inspect_tx: mpsc::Sender<SyncInspectRequest<D::Error>>,
    voucher_tx: mpsc::Sender<SyncVoucherRequest>,
    notice_tx: mpsc::Sender<SyncNoticeRequest>,
    report_tx: mpsc::Sender<Report>,
    finish_tx: mpsc::Sender<FinishStatus>,
    shutdown_tx: mpsc::Sender<()>,
}

// Auto derive clone doesn't work well with generics
impl<D: DApp> Clone for Controller<D> {
    fn clone(&self) -> Self {
        Self {
            advance_tx: self.advance_tx.clone(),
            inspect_tx: self.inspect_tx.clone(),
            voucher_tx: self.voucher_tx.clone(),
            notice_tx: self.notice_tx.clone(),
            report_tx: self.report_tx.clone(),
            finish_tx: self.finish_tx.clone(),
            shutdown_tx: self.shutdown_tx.clone(),
        }
    }
}

/// Callback object that will be called when advance if finished
/// This could be callback, but async closures are unstable at the current version of Rust
/// https://github.com/rust-lang/rust/issues/62290
#[cfg_attr(test, automock)]
#[async_trait]
pub trait AdvanceFinisher<D: DApp>: std::fmt::Debug + Send + Sync {
    async fn handle(&self, result: Result<AdvanceResult, D::Error>);
}

impl<D: DApp> Controller<D> {
    /// Send an advance request
    /// The request will run in the background and the callback will be called when it is finished.
    pub async fn advance(&self, request: AdvanceRequest, finisher: Box<dyn AdvanceFinisher<D>>) {
        self.advance_tx
            .send(AdvanceRequestWrapper { request, finisher })
            .await
            .expect("send should not fail");
    }

    /// Send an inspect request and wait for the report
    /// The function only returns when the request is complete.
    pub async fn inspect(&self, request: InspectRequest) -> Result<Vec<Report>, D::Error> {
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
        T: std::fmt::Debug + Send + Sync,
        U: std::fmt::Debug + Send + Sync,
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
pub struct ControllerService<D: DApp> {
    state: State<D>,
    advance_rx: mpsc::Receiver<AdvanceRequestWrapper<D>>,
    inspect_rx: mpsc::Receiver<SyncInspectRequest<D::Error>>,
    voucher_rx: mpsc::Receiver<SyncVoucherRequest>,
    notice_rx: mpsc::Receiver<SyncNoticeRequest>,
    report_rx: mpsc::Receiver<Report>,
    finish_rx: mpsc::Receiver<FinishStatus>,
    shutdown_rx: mpsc::Receiver<()>,
}

impl<D: DApp> ControllerService<D> {
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
struct AdvanceRequestWrapper<D: DApp> {
    request: AdvanceRequest,
    finisher: Box<dyn AdvanceFinisher<D>>,
}

#[derive(Debug)]
struct SyncRequest<T: Send + Sync, U: Send + Sync> {
    value: T,
    response_tx: oneshot::Sender<U>,
}

type SyncInspectRequest<E> = SyncRequest<InspectRequest, Result<Vec<Report>, E>>;
type SyncVoucherRequest = SyncRequest<Voucher, Result<u64, InsertError>>;
type SyncNoticeRequest = SyncRequest<Notice, Result<u64, InsertError>>;

enum State<D: DApp> {
    Idle(IdleState<D>),
    Advancing(AdvancingState<D>),
}

struct IdleState<D: DApp> {
    dapp: D,
    id: u64,
}

impl<D: DApp> IdleState<D> {
    fn new(dapp: D, id: u64) -> State<D> {
        State::Idle(Self { dapp, id })
    }

    fn insert_voucher(self, request: SyncVoucherRequest) -> State<D> {
        log::warn!("ignoring voucher when idle");
        Self::reject_insert_request(request);
        State::Idle(self)
    }

    fn insert_notice(self, request: SyncNoticeRequest) -> State<D> {
        log::warn!("ignoring notice when idle");
        Self::reject_insert_request(request);
        State::Idle(self)
    }

    fn insert_report(self) -> State<D> {
        log::warn!("ignoring report when idle");
        State::Idle(self)
    }

    fn finish(self) -> State<D> {
        log::warn!("ignoring finish when idle");
        State::Idle(self)
    }

    async fn advance(self, wrapper: AdvanceRequestWrapper<D>) -> State<D> {
        match self.dapp.advance(wrapper.request).await {
            Ok(_) => {
                log::info!("setting state to advance");
                AdvancingState::new(self.dapp, self.id, wrapper.finisher)
            }
            Err(e) => {
                log::error!("failed to advance state with error: {}", e);
                wrapper.finisher.handle(Err(e)).await;
                State::Idle(self)
            }
        }
    }

    async fn inspect(self, request: SyncInspectRequest<D::Error>) -> State<D> {
        log::info!("processing inspect request");
        let response = self.dapp.inspect(request.value).await.map_err(|e| {
            log::error!("failed to inspect with error: {}", e);
            e
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

struct AdvancingState<D: DApp> {
    dapp: D,
    id: u64,
    finisher: Box<dyn AdvanceFinisher<D>>,
    vouchers: Vec<Identified<Voucher>>,
    notices: Vec<Identified<Notice>>,
    reports: Vec<Report>,
}

impl<D: DApp> AdvancingState<D> {
    fn new(dapp: D, id: u64, finisher: Box<dyn AdvanceFinisher<D>>) -> State<D> {
        State::Advancing(Self {
            dapp,
            id,
            finisher,
            vouchers: vec![],
            notices: vec![],
            reports: vec![],
        })
    }

    async fn insert_voucher(mut self, request: SyncVoucherRequest) -> State<D> {
        log::info!("processing voucher request");
        Self::insert_identified(request, &mut self.vouchers, &mut self.id);
        State::Advancing(self)
    }

    async fn insert_notice(mut self, request: SyncNoticeRequest) -> State<D> {
        log::info!("processing notice request");
        Self::insert_identified(request, &mut self.notices, &mut self.id);
        State::Advancing(self)
    }

    async fn insert_report(mut self, report: Report) -> State<D> {
        log::info!("processing report request");
        self.reports.push(report);
        State::Advancing(self)
    }

    async fn finish(self, status: FinishStatus) -> State<D> {
        log::info!("processing finish request; setting state to idle");
        let input = AdvanceResult {
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
        let mut dapp = MockDApp::new();
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
        let (controller, service) = new(dapp);
        let service = tokio::spawn(service.run());

        controller
            .advance(request, mock_finisher(FinishStatus::Accept))
            .await;
        notify.notified().await;
        controller.finish(FinishStatus::Accept).await;
        controller.shutdown().await;
        service.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_sends_an_inspect_request() {
        let request = InspectRequest {
            payload: vec![1, 2, 3],
        };
        let expected = vec![
            Report {
                payload: vec![4, 5, 6],
            },
            Report {
                payload: vec![7, 8, 9],
            },
        ];
        let mut dapp = MockDApp::new();
        {
            let response = expected.clone();
            dapp.expect_inspect()
                .times(1)
                .with(eq(request.clone()))
                .return_once(move |_| Ok(response));
        }
        let (controller, service) = new(dapp);
        let service = tokio::spawn(service.run());

        let got = controller.inspect(request).await.unwrap();
        assert!(got == expected);
        controller.shutdown().await;
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
        let mut dapp = MockDApp::new();
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
        let (controller, service) = new(dapp);
        let service = tokio::spawn(service.run());

        for notify in notifies {
            controller
                .advance(request.clone(), mock_finisher(FinishStatus::Accept))
                .await;
            notify.notified().await;
            controller.finish(FinishStatus::Accept).await;
        }
        controller.shutdown().await;
        service.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_prioritizes_inspect_requests_over_advance_requests() {
        let advance_request = mock_advance_request();
        let inspect_request = InspectRequest {
            payload: vec![1, 2, 3],
        };
        let advance_notify = &[Arc::new(Notify::new()), Arc::new(Notify::new())];
        let mut dapp = MockDApp::new();
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
        let (controller, service) = new(dapp);
        let service = tokio::spawn(service.run());

        controller
            .advance(advance_request.clone(), mock_finisher(FinishStatus::Accept))
            .await;
        advance_notify[0].notified().await;
        // Send both an advance and an inspect while processing the first advance
        controller
            .advance(advance_request, mock_finisher(FinishStatus::Accept))
            .await;
        let inspect_handle = {
            let controller = controller.clone();
            tokio::spawn(async move { controller.inspect(inspect_request).await.unwrap() })
        };
        controller.finish(FinishStatus::Accept).await; // Accept the first advance
        inspect_handle.await.unwrap(); // Wait for the inspect to finish first
        advance_notify[1].notified().await; // Then wait for the second advance
        controller.finish(FinishStatus::Accept).await; // Finally, accept the second advance
        controller.shutdown().await;
        service.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_rejects_vouchers_and_notices_when_idle() {
        let dapp = MockDApp::new();
        let (controller, service) = new(dapp);
        let service = tokio::spawn(service.run());

        let result = controller
            .insert_notice(Notice {
                payload: vec![1, 2, 3],
            })
            .await;
        assert!(matches!(result, Err(InsertError {})));
        let result = controller
            .insert_voucher(Voucher {
                address: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
                payload: vec![4, 5, 6],
            })
            .await;
        assert!(matches!(result, Err(InsertError {})));
        controller.shutdown().await;
        service.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_sends_vouchers_notices_and_reports_to_finisher() {
        let request = mock_advance_request();
        let notify = Arc::new(Notify::new());
        let input = AdvanceResult {
            status: FinishStatus::Reject,
            vouchers: vec![
                Identified {
                    id: 0,
                    value: Voucher {
                        address: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
                        payload: vec![1, 2, 3],
                    },
                },
                Identified {
                    id: 1,
                    value: Voucher {
                        address: [9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8],
                        payload: vec![4, 5, 6],
                    },
                },
            ],
            notices: vec![
                Identified {
                    id: 2,
                    value: Notice {
                        payload: vec![7, 8, 9],
                    },
                },
                Identified {
                    id: 3,
                    value: Notice {
                        payload: vec![0, 1, 2],
                    },
                },
            ],
            reports: vec![Report {
                payload: vec![3, 4, 5],
            }],
        };
        let mut finisher = Box::new(MockAdvanceFinisher::new());
        finisher
            .expect_handle()
            .times(1)
            .with(eq(Ok(input.clone())))
            .return_once(|_| ());
        let mut dapp = MockDApp::new();
        {
            let notify = notify.clone();
            dapp.expect_advance().times(1).return_once(move |_| {
                notify.notify_one();
                Ok(())
            });
        }
        let (controller, service) = new(dapp);
        let service = tokio::spawn(service.run());

        controller.advance(request, finisher).await;
        notify.notified().await;
        for voucher in input.vouchers {
            controller
                .insert_voucher(voucher.value.clone())
                .await
                .unwrap();
        }
        for notice in input.notices {
            controller
                .insert_notice(notice.value.clone())
                .await
                .unwrap();
        }
        for report in input.reports {
            controller.insert_report(report).await;
        }
        controller.finish(FinishStatus::Reject).await;
        controller.shutdown().await;
        service.await.unwrap();
    }

    fn mock_input(status: FinishStatus) -> AdvanceResult {
        AdvanceResult {
            status,
            vouchers: vec![],
            notices: vec![],
            reports: vec![],
        }
    }

    fn mock_finisher(status: FinishStatus) -> Box<dyn AdvanceFinisher<MockDApp>> {
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
                address: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
                epoch_number: 1,
                input_number: 2,
                block_number: 3,
                timestamp: 1635946561,
            },
            payload: vec![1, 2, 3],
        }
    }

    // This is required to use the eq predicate in the finisher mock
    impl PartialEq for MockDAppError {
        fn eq(&self, _: &Self) -> bool {
            true
        }
    }
}
