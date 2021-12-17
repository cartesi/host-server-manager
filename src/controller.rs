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
    AdvanceRequest, AdvanceResult, FinishStatus, InspectRequest, Notice, Report, Voucher,
};
use crate::sync_request::SyncRequest;

const BUFFER_SIZE: usize = 1000;

/// State-machine that controls whether the host server manager is advancing the epoch state or
/// it is idle.
/// When processing advancing the epoch state, the host server manager receives vouchers, notices,
/// and reports until it receives a finish request.
/// This struct manipulates the data inside an independent thread that can only communicate through
/// channels.
#[derive(Debug)]
pub struct Controller<D: DApp> {
    advance_tx: mpsc::Sender<SyncAdvanceRequest<D::Error>>,
    inspect_tx: mpsc::Sender<SyncInspectRequest<D::Error>>,
    voucher_tx: mpsc::Sender<SyncVoucherRequest>,
    notice_tx: mpsc::Sender<SyncNoticeRequest>,
    report_tx: mpsc::Sender<Report>,
    finish_tx: mpsc::Sender<FinishStatus>,
    shutdown_tx: mpsc::Sender<SyncShutdownRequest>,
}

impl<D: DApp> Controller<D> {
    pub fn new(dapp: D) -> Self {
        let (advance_tx, advance_rx) = mpsc::channel(BUFFER_SIZE);
        let (inspect_tx, inspect_rx) = mpsc::channel(BUFFER_SIZE);
        let (voucher_tx, voucher_rx) = mpsc::channel(BUFFER_SIZE);
        let (notice_tx, notice_rx) = mpsc::channel(BUFFER_SIZE);
        let (report_tx, report_rx) = mpsc::channel(BUFFER_SIZE);
        let (finish_tx, finish_rx) = mpsc::channel(BUFFER_SIZE);
        let (shutdown_tx, shutdown_rx) = mpsc::channel(BUFFER_SIZE);
        let service = Service::new(
            dapp,
            advance_rx,
            inspect_rx,
            voucher_rx,
            notice_rx,
            report_rx,
            finish_rx,
            shutdown_rx,
        );
        tokio::spawn(service.run());
        Self {
            advance_tx,
            inspect_tx,
            voucher_tx,
            notice_tx,
            report_tx,
            finish_tx,
            shutdown_tx,
        }
    }

    pub async fn advance(
        &self,
        request: AdvanceRequest,
    ) -> oneshot::Receiver<Result<AdvanceResult, D::Error>> {
        SyncRequest::send(&self.advance_tx, request).await
    }

    pub async fn inspect(
        &self,
        request: InspectRequest,
    ) -> oneshot::Receiver<Result<Vec<Report>, D::Error>> {
        SyncRequest::send(&self.inspect_tx, request).await
    }

    pub async fn insert_voucher(
        &self,
        voucher: Voucher,
    ) -> oneshot::Receiver<Result<u64, InsertError>> {
        SyncRequest::send(&self.voucher_tx, voucher).await
    }

    pub async fn insert_notice(
        &self,
        notice: Notice,
    ) -> oneshot::Receiver<Result<u64, InsertError>> {
        SyncRequest::send(&self.notice_tx, notice).await
    }

    pub async fn insert_report(&self, request: Report) {
        if let Err(e) = self.report_tx.send(request).await {
            log::error!("failed to send insert report request ({})", e);
        }
    }

    pub async fn finish(&self, status: FinishStatus) {
        if let Err(e) = self.finish_tx.send(status).await {
            log::error!("failed to send finish request ({})", e);
        }
    }

    pub async fn shutdown(&self) -> oneshot::Receiver<()> {
        SyncRequest::send(&self.shutdown_tx, ()).await
    }
}

// Auto derive clone doesn't work well with generics and we don't want to enforce clone for DApp
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

#[derive(Debug, Snafu, PartialEq)]
#[snafu(display("request not accepted when state is idle"))]
pub struct InsertError {}

/// Service used to communicate with the DApp
#[cfg_attr(test, automock(type Error=tests::MockDAppError;))]
#[async_trait]
pub trait DApp: std::fmt::Debug + Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync;

    /// Send an advance request to the client.
    /// The request should be processed in another thread and the response should be sent through
    /// an oneshot channel.
    async fn advance(&self, request: AdvanceRequest) -> oneshot::Receiver<Result<(), Self::Error>>;

    /// Send an inspect request to the client.
    async fn inspect(
        &self,
        request: InspectRequest,
    ) -> oneshot::Receiver<Result<Vec<Report>, Self::Error>>;

    /// Shutdown the DApp service.
    async fn shutdown(&self) -> oneshot::Receiver<()>;
}

type SyncAdvanceRequest<E> = SyncRequest<AdvanceRequest, Result<AdvanceResult, E>>;
type SyncInspectRequest<E> = SyncRequest<InspectRequest, Result<Vec<Report>, E>>;
type SyncVoucherRequest = SyncRequest<Voucher, Result<u64, InsertError>>;
type SyncNoticeRequest = SyncRequest<Notice, Result<u64, InsertError>>;
type SyncShutdownRequest = SyncRequest<(), ()>;

struct Service {
    state: Box<dyn State>,
}

impl Service {
    fn new<D: DApp>(
        dapp: D,
        advance_rx: mpsc::Receiver<SyncAdvanceRequest<D::Error>>,
        inspect_rx: mpsc::Receiver<SyncInspectRequest<D::Error>>,
        voucher_rx: mpsc::Receiver<SyncVoucherRequest>,
        notice_rx: mpsc::Receiver<SyncNoticeRequest>,
        report_rx: mpsc::Receiver<Report>,
        finish_rx: mpsc::Receiver<FinishStatus>,
        shutdown_rx: mpsc::Receiver<SyncShutdownRequest>,
    ) -> Self {
        let channels = Channels {
            advance_rx,
            inspect_rx,
            voucher_rx,
            notice_rx,
            report_rx,
            finish_rx,
            shutdown_rx,
        };
        Self {
            state: Idle::new(dapp, channels),
        }
    }

    async fn run(mut self) {
        loop {
            if let Some(state) = self.state.process().await {
                self.state = state;
            } else {
                log::info!("controller service terminated successfully");
                break;
            }
        }
    }

    async fn shutdown<D>(request: SyncShutdownRequest, dapp: &D) -> Option<Box<dyn State>>
    where
        D: DApp,
    {
        log::info!("processing shutdown request");
        let rx = dapp.shutdown().await;
        if let Err(_) = rx.await {
            log::error!("failed to send shutdown to dapp (channel dropped)");
        }
        request.process(|_| ());
        None
    }
}

struct Channels<D: DApp> {
    advance_rx: mpsc::Receiver<SyncAdvanceRequest<D::Error>>,
    inspect_rx: mpsc::Receiver<SyncInspectRequest<D::Error>>,
    voucher_rx: mpsc::Receiver<SyncVoucherRequest>,
    notice_rx: mpsc::Receiver<SyncNoticeRequest>,
    report_rx: mpsc::Receiver<Report>,
    finish_rx: mpsc::Receiver<FinishStatus>,
    shutdown_rx: mpsc::Receiver<SyncShutdownRequest>,
}

/// OOP state design-pattern
#[async_trait]
trait State: Send + Sync {
    async fn process(self: Box<Self>) -> Option<Box<dyn State>>;
}

/// In idle state, the controller processes inspect requests and waits for advance requests.
struct Idle<D: DApp> {
    dapp: D,
    channels: Channels<D>,
}

impl<D: DApp> Idle<D> {
    fn new(dapp: D, channels: Channels<D>) -> Box<dyn State> {
        log::info!("setting state to idle");
        Box::new(Self { dapp, channels })
    }

    async fn inspect(&self, request: InspectRequest) -> Option<Result<Vec<Report>, D::Error>> {
        let rx = self.dapp.inspect(request).await;
        match rx.await {
            Ok(result) => Some(result.map_err(|e| {
                log::error!("failed to send inspect request ({})", e);
                e
            })),
            Err(e) => {
                log::error!("failed to receive inspect response ({})", e);
                None
            }
        }
    }
}

#[async_trait]
impl<D: DApp> State for Idle<D> {
    async fn process(mut self: Box<Self>) -> Option<Box<dyn State>> {
        tokio::select! {
            biased;
            Some(request) = self.channels.voucher_rx.recv() => {
                log::warn!("ignoring voucher when idle");
                request.process(|_| Err(InsertError {}));
                Some(self)
            }
            Some(request) = self.channels.notice_rx.recv() => {
                log::warn!("ignoring notice when idle");
                request.process(|_| Err(InsertError {}));
                Some(self)
            }
            Some(_) = self.channels.report_rx.recv() => {
                log::warn!("ignoring report when idle");
                Some(self)
            }
            Some(_) = self.channels.finish_rx.recv() => {
                log::warn!("ignoring finish when idle");
                Some(self)
            }
            Some(request) = self.channels.inspect_rx.recv() => {
                log::info!("processing inspect request");
                request.try_process_async(|request| self.inspect(request)).await;
                Some(self)
            }
            Some(request) = self.channels.advance_rx.recv() => {
                log::info!("processing advance request");
                Some(Advance::new(self.dapp, self.channels, request).await)
            }
            Some(request) = self.channels.shutdown_rx.recv() => {
                Service::shutdown(request, &self.dapp).await
            }
        }
    }
}

/// When advancing, the controller receives vouchers, notices, reports and the finish request.
/// The advance is only finished when it receives both a finish request and the DApp advance
/// response.
/// There are two distinct advance states.
/// The first state waits for the DApp response and listen to requests that require responses, such
/// as insert voucher and insert notice.
/// The second advance state listen to all advance-related requests and waits until finish is
/// called.
/// Notice that the advance states do no handle the advance and inspect requests.
/// Those are ketp in the queue until the controller goes back to the idle state.
struct Advance<D: DApp> {
    dapp: D,
    channels: Channels<D>,
    advance_tx: oneshot::Sender<Result<AdvanceResult, D::Error>>,
    dapp_advance_rx: oneshot::Receiver<Result<(), D::Error>>,
    vouchers: Vec<Voucher>,
    notices: Vec<Notice>,
}

impl<D: DApp> Advance<D> {
    async fn new(
        dapp: D,
        channels: Channels<D>,
        request: SyncAdvanceRequest<D::Error>,
    ) -> Box<dyn State> {
        log::info!("setting state to advance");
        let (request, advance_tx) = request.into_inner();
        let dapp_advance_rx = dapp.advance(request).await;
        Box::new(Self {
            dapp,
            channels,
            advance_tx,
            dapp_advance_rx,
            vouchers: vec![],
            notices: vec![],
        })
    }
}

#[async_trait]
impl<D: DApp> State for Advance<D> {
    async fn process(mut self: Box<Self>) -> Option<Box<dyn State>> {
        tokio::select! {
            biased;
            result = &mut self.dapp_advance_rx => {
                log::info!("processing dapp advance response");
                match result {
                    Ok(result) => match result {
                        Ok(_) => Some(AdvanceUntilFinish::from(self)),
                        Err(e) => {
                            log::error!("failed to send advance request to dapp ({})", e);
                            if let Err(_) = self.advance_tx.send(Err(e)) {
                                log::error!("failed to send advance response (channel dropped)");
                            }
                            Some(Idle::new(self.dapp, self.channels))
                        }
                    }
                    Err(e) => {
                        log::error!("failed to receive dapp advance response ({})", e);
                        Some(Idle::new(self.dapp, self.channels))
                    }
                }
            }
            Some(request) = self.channels.voucher_rx.recv() => {
                log::info!("processing insert voucher request");
                process_insert_request(request, &mut self.vouchers);
                Some(self)
            }
            Some(request) = self.channels.notice_rx.recv() => {
                log::info!("processing insert notice request");
                process_insert_request(request, &mut self.notices);
                Some(self)
            }
            Some(request) = self.channels.shutdown_rx.recv() => {
                Service::shutdown(request, &self.dapp).await
            }
        }
    }
}

struct AdvanceUntilFinish<D: DApp> {
    dapp: D,
    channels: Channels<D>,
    advance_tx: oneshot::Sender<Result<AdvanceResult, D::Error>>,
    vouchers: Vec<Voucher>,
    notices: Vec<Notice>,
    reports: Vec<Report>,
}

impl<D: DApp> AdvanceUntilFinish<D> {
    fn from(advance: Box<Advance<D>>) -> Box<dyn State> {
        log::info!("setting state to advance until finish");
        Box::new(Self {
            dapp: advance.dapp,
            channels: advance.channels,
            advance_tx: advance.advance_tx,
            vouchers: advance.vouchers,
            notices: advance.notices,
            reports: vec![],
        })
    }
}

#[async_trait]
impl<D: DApp> State for AdvanceUntilFinish<D> {
    async fn process(mut self: Box<Self>) -> Option<Box<dyn State>> {
        tokio::select! {
            biased;
            Some(request) = self.channels.voucher_rx.recv() => {
                log::info!("processing insert voucher request");
                process_insert_request(request, &mut self.vouchers);
                Some(self)
            }
            Some(request) = self.channels.notice_rx.recv() => {
                log::info!("processing insert notice request");
                process_insert_request(request, &mut self.notices);
                Some(self)
            }
            Some(report) = self.channels.report_rx.recv() => {
                log::info!("processing insert report request");
                self.reports.push(report);
                Some(self)
            }
            Some(status) = self.channels.finish_rx.recv() => {
                log::info!("processing finish request");
                let result = AdvanceResult {
                    status,
                    vouchers: self.vouchers,
                    notices: self.notices,
                    reports: self.reports,
                };
                if let Err(_) = self.advance_tx.send(Ok(result)) {
                    log::error!("failed to send advance result (channel dropped)");
                }
                Some(Idle::new(self.dapp, self.channels))
            }
            Some(request) = self.channels.shutdown_rx.recv() => {
                Service::shutdown(request, &self.dapp).await
            }
        }
    }
}

fn process_insert_request<T, E>(request: SyncRequest<T, Result<u64, E>>, vector: &mut Vec<T>)
where
    T: Send + Sync + std::fmt::Debug,
    E: Send + Sync + std::fmt::Debug,
{
    request.process(|item| {
        vector.push(item);
        Ok(vector.len() as u64 - 1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AdvanceMetadata;
    use std::sync::Arc;
    use tokio::sync::Notify;

    fn setup() -> (MockDApp, Sequence) {
        // enable logs
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Info)
            .is_test(true)
            .try_init();

        // setup mock dapp
        let mut dapp = MockDApp::new();
        let sequence = Sequence::new();
        dapp.expect_shutdown().times(1).return_once(|| {
            let (response_tx, response_rx) = oneshot::channel();
            response_tx.send(()).unwrap();
            response_rx
        });
        (dapp, sequence)
    }

    #[tokio::test]
    async fn test_it_sends_an_advance_request_and_dapp_returns_ok() {
        let (mut dapp, mut sequence) = setup();
        let advance = expect_advance_request(&mut dapp, &mut sequence).await;
        let controller = Controller::new(dapp);
        let advance_rx = controller.advance(advance.request).await;
        advance.received.notified().await;
        advance.response_tx.send(Ok(())).unwrap();
        controller.finish(FinishStatus::Accept).await;
        let result = advance_rx.await.unwrap().unwrap();
        assert_eq!(result, mock_result(FinishStatus::Accept));
        controller.shutdown().await.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_sends_an_advance_request_and_dapp_returns_error() {
        let (mut dapp, mut sequence) = setup();
        let advance = expect_advance_request(&mut dapp, &mut sequence).await;
        let controller = Controller::new(dapp);
        let advance_rx = controller.advance(advance.request).await;
        advance.received.notified().await;
        advance.response_tx.send(Err(MockDAppError {})).unwrap();
        controller.finish(FinishStatus::Accept).await;
        let err = advance_rx.await.unwrap().unwrap_err();
        assert_eq!(err, MockDAppError {});
        controller.shutdown().await.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_sends_an_inspect_request_and_dapp_returns_ok() {
        let (mut dapp, mut sequence) = setup();
        let inspect = expect_inspect_request(&mut dapp, &mut sequence);
        let controller = Controller::new(dapp);
        let inspect_rx = controller.inspect(inspect.request).await;
        let expected_reports = vec![mock_report()];
        inspect
            .response_tx
            .send(Ok(expected_reports.clone()))
            .unwrap();
        let reports = inspect_rx.await.unwrap().unwrap();
        assert!(reports == expected_reports);
        controller.shutdown().await.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_sends_an_inspect_request_and_dapp_returns_error() {
        let (mut dapp, mut sequence) = setup();
        let inspect = expect_inspect_request(&mut dapp, &mut sequence);
        let controller = Controller::new(dapp);
        let inspect_rx = controller.inspect(inspect.request).await;
        inspect.response_tx.send(Err(MockDAppError {})).unwrap();
        let err = inspect_rx.await.unwrap().unwrap_err();
        assert_eq!(err, MockDAppError {});
        controller.shutdown().await.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_processes_multiple_advance_requests() {
        let (mut dapp, mut sequence) = setup();
        let mut advances: Vec<MockAdvance> = Vec::new();
        for _ in 0..3 {
            advances.push(expect_advance_request(&mut dapp, &mut sequence).await);
        }
        let controller = Controller::new(dapp);
        for advance in advances {
            let advance_rx = controller.advance(advance.request).await;
            advance.received.notified().await;
            advance.response_tx.send(Ok(())).unwrap();
            controller.finish(FinishStatus::Accept).await;
            let result = advance_rx.await.unwrap().unwrap();
            assert_eq!(result, mock_result(FinishStatus::Accept));
        }
        controller.shutdown().await.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_prioritizes_inspect_requests_over_advance_requests() {
        let (mut dapp, mut sequence) = setup();
        let first_advance = expect_advance_request(&mut dapp, &mut sequence).await;
        let inspect = expect_inspect_request(&mut dapp, &mut sequence);
        let second_advance = expect_advance_request(&mut dapp, &mut sequence).await;
        let controller = Controller::new(dapp);
        // Wait for the first advance to start
        let first_advance_rx = controller.advance(first_advance.request).await;
        first_advance.received.notified().await;
        first_advance.response_tx.send(Ok(())).unwrap();
        // Send both an advance and an inspect while processing the first advance
        let second_advance_rx = controller.advance(second_advance.request).await;
        let inspect_rx = controller.inspect(inspect.request).await;
        // Finish the first advance
        controller.finish(FinishStatus::Accept).await;
        let result = first_advance_rx.await.unwrap().unwrap();
        assert_eq!(result, mock_result(FinishStatus::Accept));
        // Wait for the inspect to finish before the second advance
        let expected_reports = vec![mock_report()];
        inspect
            .response_tx
            .send(Ok(expected_reports.clone()))
            .unwrap();
        let reports = inspect_rx.await.unwrap().unwrap();
        assert!(reports == expected_reports);
        // Then wait for the second advance and finish it
        second_advance.received.notified().await;
        second_advance.response_tx.send(Ok(())).unwrap();
        controller.finish(FinishStatus::Accept).await;
        let result = second_advance_rx.await.unwrap().unwrap();
        assert_eq!(result, mock_result(FinishStatus::Accept));
        controller.shutdown().await.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_rejects_vouchers_and_notices_when_idle() {
        let (dapp, _) = setup();
        let controller = Controller::new(dapp);
        let insert_rx = controller.insert_voucher(mock_voucher()).await;
        let err = insert_rx.await.unwrap().unwrap_err();
        assert_eq!(err, InsertError {});
        let insert_rx = controller.insert_notice(mock_notice()).await;
        let err = insert_rx.await.unwrap().unwrap_err();
        assert_eq!(err, InsertError {});
        controller.shutdown().await.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_receives_vouchers_notices_and_reports() {
        let (mut dapp, mut sequence) = setup();
        let advance = expect_advance_request(&mut dapp, &mut sequence).await;
        let controller = Controller::new(dapp);
        let expected_result = AdvanceResult {
            status: FinishStatus::Reject,
            vouchers: vec![mock_voucher(), mock_voucher()],
            notices: vec![mock_notice(), mock_notice()],
            reports: vec![mock_report(), mock_report()],
        };
        let advance_rx = controller.advance(advance.request).await;
        advance.received.notified().await;
        advance.response_tx.send(Ok(())).unwrap();
        for (expected_id, voucher) in expected_result.vouchers.iter().enumerate() {
            let insert_rx = controller.insert_voucher(voucher.clone()).await;
            let obtained_id = insert_rx.await.unwrap().unwrap();
            assert_eq!(obtained_id, expected_id as u64);
        }
        for (expected_id, notice) in expected_result.notices.iter().enumerate() {
            let insert_rx = controller.insert_notice(notice.clone()).await;
            let obtained_id = insert_rx.await.unwrap().unwrap();
            assert_eq!(obtained_id, expected_id as u64);
        }
        for report in expected_result.reports.iter() {
            controller.insert_report(report.clone()).await;
        }
        controller.finish(FinishStatus::Reject).await;
        let obtained_result = advance_rx.await.unwrap().unwrap();
        assert_eq!(obtained_result, expected_result);
        controller.shutdown().await.await.unwrap();
    }

    #[tokio::test]
    async fn test_it_accepts_vouchers_and_notices_before_advance_response() {
        let (mut dapp, mut sequence) = setup();
        let advance = expect_advance_request(&mut dapp, &mut sequence).await;
        let controller = Controller::new(dapp);
        let expected_result = AdvanceResult {
            status: FinishStatus::Reject,
            vouchers: vec![mock_voucher(), mock_voucher()],
            notices: vec![mock_notice(), mock_notice()],
            reports: vec![mock_report(), mock_report()],
        };
        let advance_rx = controller.advance(advance.request).await;
        advance.received.notified().await;
        for (expected_id, voucher) in expected_result.vouchers.iter().enumerate() {
            let insert_rx = controller.insert_voucher(voucher.clone()).await;
            let obtained_id = insert_rx.await.unwrap().unwrap();
            assert_eq!(obtained_id, expected_id as u64);
        }
        for (expected_id, notice) in expected_result.notices.iter().enumerate() {
            let insert_rx = controller.insert_notice(notice.clone()).await;
            let obtained_id = insert_rx.await.unwrap().unwrap();
            assert_eq!(obtained_id, expected_id as u64);
        }
        for report in expected_result.reports.iter() {
            controller.insert_report(report.clone()).await;
        }
        controller.finish(FinishStatus::Reject).await;
        // Only respond to advance after sending a finish response
        advance.response_tx.send(Ok(())).unwrap();
        let obtained_result = advance_rx.await.unwrap().unwrap();
        assert_eq!(obtained_result, expected_result);
        controller.shutdown().await.await.unwrap();
    }

    fn mock_advance_request() -> AdvanceRequest {
        AdvanceRequest {
            metadata: AdvanceMetadata {
                address: rand::random(),
                epoch_number: rand::random(),
                input_number: rand::random(),
                block_number: rand::random(),
                timestamp: rand::random(),
            },
            payload: rand::random::<[u8; 32]>().into(),
        }
    }

    fn mock_voucher() -> Voucher {
        Voucher {
            address: rand::random(),
            payload: rand::random::<[u8; 32]>().into(),
        }
    }

    fn mock_notice() -> Notice {
        Notice {
            payload: rand::random::<[u8; 32]>().into(),
        }
    }

    fn mock_report() -> Report {
        Report {
            payload: rand::random::<[u8; 32]>().into(),
        }
    }

    fn mock_result(status: FinishStatus) -> AdvanceResult {
        AdvanceResult {
            status,
            vouchers: vec![],
            notices: vec![],
            reports: vec![],
        }
    }

    struct MockAdvance {
        request: AdvanceRequest,
        received: Arc<Notify>,
        response_tx: oneshot::Sender<Result<(), MockDAppError>>,
    }

    async fn expect_advance_request(dapp: &mut MockDApp, sequence: &mut Sequence) -> MockAdvance {
        let request = mock_advance_request();
        let received = Arc::new(Notify::new());
        let notify = received.clone();
        let (response_tx, response_rx) = oneshot::channel();
        dapp.expect_advance()
            .times(1)
            .with(eq(request.clone()))
            .in_sequence(sequence)
            .return_once(move |_| {
                notify.notify_one();
                response_rx
            });
        MockAdvance {
            request,
            received,
            response_tx,
        }
    }

    struct MockInspect {
        request: InspectRequest,
        response_tx: oneshot::Sender<Result<Vec<Report>, MockDAppError>>,
    }

    fn expect_inspect_request(dapp: &mut MockDApp, sequence: &mut Sequence) -> MockInspect {
        let request = InspectRequest {
            payload: rand::random::<[u8; 32]>().into(),
        };
        let (response_tx, response_rx) = oneshot::channel();
        dapp.expect_inspect()
            .times(1)
            .with(eq(request.clone()))
            .in_sequence(sequence)
            .return_once(move |_| response_rx);
        MockInspect {
            request,
            response_tx,
        }
    }

    #[derive(Debug, Snafu, PartialEq)]
    #[snafu(display("mock dapp error"))]
    pub struct MockDAppError {}
}
