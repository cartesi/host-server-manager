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

use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::conversions;
use crate::model::{AdvanceMetadata, AdvanceRequest, FinishStatus, Input};
use crate::proxy::{AdvanceError, AdvanceFinisher, ProxyChannel};

use super::proto::cartesi_machine::Void;
use super::proto::rollup_machine_manager::rollup_machine_manager_server::RollupMachineManager;
use super::proto::rollup_machine_manager::{
    processed_input::ProcessedOneof, Address, AdvanceStateRequest, CompletionStatus,
    EndSessionRequest, EpochState, FinishEpochRequest, GetEpochStatusRequest,
    GetEpochStatusResponse, GetSessionStatusRequest, GetSessionStatusResponse, GetStatusResponse,
    InputResult, InspectStateRequest, InspectStateResponse, Notice, ProcessedInput, Report,
    StartSessionRequest, TaintStatus, Voucher,
};
use super::proto::versioning::{GetVersionResponse, SemanticVersion};

pub struct RollupMachineManagerService {
    proxy: ProxyChannel,
    session_cell: Mutex<Option<(String, Arc<Mutex<Session>>)>>, // it only supports one session
}

#[derive(Debug)]
struct Session {
    active_epoch_index: u64,
    epochs: HashMap<u64, Arc<Mutex<Epoch>>>,
    tainted: Arc<Mutex<Option<Status>>>,
}

#[derive(Debug)]
struct Epoch {
    state: EpochState,
    pending_inputs: u64,
    processed_inputs: Vec<Input>,
}

#[derive(Debug)]
struct Finisher {
    epoch: Arc<Mutex<Epoch>>,
    tainted: Arc<Mutex<Option<Status>>>,
}

#[tonic::async_trait]
impl RollupMachineManager for RollupMachineManagerService {
    async fn get_version(&self, _: Request<Void>) -> Result<Response<GetVersionResponse>, Status> {
        log::info!("received get_version");
        let response = GetVersionResponse {
            version: Some(SemanticVersion {
                major: 0,
                minor: 0,
                patch: 0,
                pre_release: String::from(""),
                build: String::from("mock-rollup-machine-manager"),
            }),
        };
        Ok(Response::new(response))
    }

    async fn start_session(
        &self,
        request: Request<StartSessionRequest>,
    ) -> Result<Response<Void>, Status> {
        let request = request.into_inner();
        log::info!("received start_session with id={}", request.session_id);
        if request.session_id == "" {
            Err(Status::invalid_argument("session id is empty"))
        } else {
            let mut session_cell = self.session_cell.lock().await;
            match *session_cell {
                Some(_) => {
                    log::warn!("the mock only supports a single session");
                    Err(Status::already_exists("session id is taken"))
                }
                None => {
                    *session_cell = Some((
                        request.session_id,
                        Arc::new(Mutex::new(Session::new(request.active_epoch_index))),
                    ));
                    Ok(Response::new(Void {}))
                }
            }
        }
    }

    async fn end_session(
        &self,
        request: Request<EndSessionRequest>,
    ) -> Result<Response<Void>, Status> {
        let request = request.into_inner();
        log::info!("received end_session with id={}", request.session_id);
        let session_mutex = self.get_session(&request.session_id).await?;
        let session = session_mutex
            .try_lock()
            .or(Err(Status::aborted("concurrent call in session")))?;
        // If the session is not tainted, we should check if the active epoch is empty
        if session.tainted.lock().await.is_none() {
            let active_epoch_index = session.active_epoch_index;
            session
                .get_active_epoch(active_epoch_index)?
                .lock()
                .await
                .check_pending_inputs()?
                .check_processed_inputs(0)?;
        }
        let mut session_cell = self.session_cell.lock().await;
        *session_cell = None;
        Ok(Response::new(Void {}))
    }

    async fn advance_state(
        &self,
        request: Request<AdvanceStateRequest>,
    ) -> Result<Response<Void>, Status> {
        let request = request.into_inner();
        log::info!("received advance_state with id={}", request.session_id);
        let metadata = request
            .input_metadata
            .ok_or(Status::invalid_argument("missing metadata from request"))?;
        let sender = metadata
            .msg_sender
            .ok_or(Status::invalid_argument("missing msg_sender from metadata"))?;
        let advance_request = AdvanceRequest {
            metadata: AdvanceMetadata {
                address: conversions::encode_ethereum_binary(&sender.data),
                epoch_number: metadata.epoch_index,
                input_number: metadata.input_index,
                block_number: metadata.block_number,
                timestamp: metadata.time_stamp,
            },
            payload: conversions::encode_ethereum_binary(&request.input_payload),
        };
        let session_mutex = self.get_session(&request.session_id).await?;
        let session = session_mutex
            .try_lock()
            .or(Err(Status::aborted("concurrent call in session")))?;
        let epoch_mutex = session
            .check_status()
            .await?
            .get_active_epoch(request.active_epoch_index)?;
        {
            let mut epoch = epoch_mutex.lock().await;
            epoch
                .check_active()?
                .check_current_input_index(request.current_input_index)?;
            epoch.add_pending_input();
        }
        let finisher = Box::new(Finisher::new(epoch_mutex.clone(), session.tainted.clone()));
        self.proxy.advance(advance_request, finisher).await;
        Ok(Response::new(Void {}))
    }

    async fn finish_epoch(
        &self,
        request: Request<FinishEpochRequest>,
    ) -> Result<Response<Void>, Status> {
        let request = request.into_inner();
        log::info!("received finish_epoch with id={}", request.session_id);
        if request.storage_directory != "" {
            log::warn!("ignoring storage_directory parameter");
        }
        let session_mutex = self.get_session(&request.session_id).await?;
        let mut session = session_mutex
            .try_lock()
            .or(Err(Status::aborted("concurrent call in session")))?;
        let epoch_mutex = session
            .check_status()
            .await?
            .get_active_epoch(request.active_epoch_index)?;
        {
            let mut epoch = epoch_mutex.lock().await;
            epoch
                .check_active()?
                .check_pending_inputs()?
                .check_processed_inputs(request.processed_input_count)?;
            epoch.finish();
        }
        session.start_new_epoch();
        Ok(Response::new(Void {}))
    }

    async fn inspect_state(
        &self,
        _: Request<InspectStateRequest>,
    ) -> Result<tonic::Response<InspectStateResponse>, Status> {
        log::warn!("received inspect_state (not implemented)");
        Err(Status::unimplemented(
            "the inspect_state should be called from the dapp-reader-server HTTP API",
        ))
    }

    async fn get_status(&self, _: Request<Void>) -> Result<Response<GetStatusResponse>, Status> {
        log::info!("received get_status");
        let mut response = GetStatusResponse { session_id: vec![] };
        if let Some((id, _)) = self.session_cell.lock().await.as_ref() {
            response.session_id.push(id.clone());
        }
        Ok(Response::new(response))
    }

    async fn get_session_status(
        &self,
        request: Request<GetSessionStatusRequest>,
    ) -> Result<Response<GetSessionStatusResponse>, Status> {
        let request = request.into_inner();
        log::info!("received get_session_status with id={}", request.session_id);
        let session_mutex = self.get_session(&request.session_id).await?;
        let session = session_mutex
            .try_lock()
            .or(Err(Status::aborted("concurrent call in session")))?;
        let response = GetSessionStatusResponse {
            session_id: request.session_id,
            active_epoch_index: session.active_epoch_index,
            epoch_index: session.epochs.keys().cloned().collect(),
            taint_status: session.get_taint_status().await,
        };
        Ok(Response::new(response))
    }

    async fn get_epoch_status(
        &self,
        request: Request<GetEpochStatusRequest>,
    ) -> Result<Response<GetEpochStatusResponse>, Status> {
        let request = request.into_inner();
        log::info!(
            "received get_epoch_status with id={} and epoch_index={}",
            request.session_id,
            request.epoch_index
        );
        let session_mutex = self.get_session(&request.session_id).await?;
        let session = session_mutex
            .try_lock()
            .or(Err(Status::aborted("concurrent call in session")))?;
        let epoch_mutex = session.get_epoch(request.epoch_index)?;
        let epoch = epoch_mutex.lock().await;
        let mut processed_inputs: Vec<ProcessedInput> = Vec::new();
        for (i, input) in epoch.processed_inputs.iter().enumerate() {
            let processed_oneof = Some(match input.status {
                FinishStatus::Accept => ProcessedOneof::Result(InputResult {
                    voucher_hashes_in_machine: None,
                    vouchers: Self::convert_vouchers(&input)?,
                    notice_hashes_in_machine: None,
                    notices: Self::convert_notices(&input)?,
                }),
                FinishStatus::Reject => {
                    ProcessedOneof::SkipReason(CompletionStatus::RejectedByMachine as i32)
                }
            });
            processed_inputs.push(ProcessedInput {
                input_index: i as u64,
                most_recent_machine_hash: None,
                voucher_hashes_in_epoch: None,
                notice_hashes_in_epoch: None,
                reports: Self::convert_reports(&input),
                processed_oneof,
            });
        }
        let response = GetEpochStatusResponse {
            session_id: request.session_id,
            epoch_index: request.epoch_index,
            state: epoch.state as i32,
            most_recent_machine_hash: None,
            most_recent_vouchers_epoch_root_hash: None,
            most_recent_notices_epoch_root_hash: None,
            processed_inputs,
            pending_input_count: epoch.pending_inputs,
            taint_status: session.get_taint_status().await,
        };
        Ok(Response::new(response))
    }
}

impl RollupMachineManagerService {
    pub fn new(proxy: ProxyChannel) -> Self {
        Self {
            proxy,
            session_cell: Mutex::new(None),
        }
    }

    async fn get_session(&self, request_id: &String) -> Result<Arc<Mutex<Session>>, Status> {
        let session_cell = self.session_cell.lock().await;
        if let Some((id, session)) = session_cell.as_ref() {
            if id == request_id {
                return Ok(session.clone());
            }
        }
        Err(Status::invalid_argument("session id not found"))
    }

    fn decode_ethereum_binary(s: &String) -> Result<Vec<u8>, Status> {
        conversions::decode_ethereum_binary(s).map_err(|e| {
            log::warn!("failed to convert Eth binary string from DApp ({})", e);
            Status::aborted("failed to convert Eth binary string from DApp")
        })
    }

    fn convert_vouchers(input: &Input) -> Result<Vec<Voucher>, Status> {
        let mut vouchers: Vec<Voucher> = Vec::new();
        for voucher in &input.vouchers {
            vouchers.push(Voucher {
                keccak: None,
                address: Some(Address {
                    data: Self::decode_ethereum_binary(&voucher.value.address)?,
                }),
                payload: Self::decode_ethereum_binary(&voucher.value.payload)?,
                keccak_in_voucher_hashes: None,
            });
        }
        Ok(vouchers)
    }

    fn convert_notices(input: &Input) -> Result<Vec<Notice>, Status> {
        let mut notices: Vec<Notice> = Vec::new();
        for notice in &input.notices {
            notices.push(Notice {
                keccak: None,
                payload: Self::decode_ethereum_binary(&notice.value.payload)?,
                keccak_in_notice_hashes: None,
            });
        }
        Ok(notices)
    }

    fn convert_reports(input: &Input) -> Vec<Report> {
        let mut reports: Vec<Report> = Vec::new();
        for report in &input.reports {
            reports.push(Report {
                payload: report.payload.as_bytes().iter().copied().collect(),
            });
        }
        reports
    }
}

impl Session {
    fn new(active_epoch_index: u64) -> Self {
        Self {
            active_epoch_index,
            epochs: HashMap::from([(active_epoch_index, Arc::new(Mutex::new(Epoch::new())))]),
            tainted: Arc::new(Mutex::new(None)),
        }
    }

    async fn check_status(&self) -> Result<&Self, Status> {
        if self.active_epoch_index == std::u64::MAX {
            Err(Status::out_of_range("active epoch index will overflow"))
        } else if self.tainted.lock().await.is_some() {
            Err(Status::data_loss("session is tainted"))
        } else {
            Ok(self)
        }
    }

    async fn get_taint_status(&self) -> Option<TaintStatus> {
        self.tainted
            .lock()
            .await
            .as_ref()
            .map(|status| TaintStatus {
                error_code: status.code() as i32,
                error_message: String::from(status.message()),
            })
    }

    fn get_epoch(&self, epoch_index: u64) -> Result<&Arc<Mutex<Epoch>>, Status> {
        self.epochs
            .get(&epoch_index)
            .ok_or(Status::invalid_argument("unknown epoch index"))
    }

    fn get_active_epoch(&self, active_epoch_index: u64) -> Result<&Arc<Mutex<Epoch>>, Status> {
        if self.active_epoch_index != active_epoch_index {
            Err(Status::invalid_argument(format!(
                "incorrect active epoch index (expected {}, got {})",
                self.active_epoch_index, active_epoch_index
            )))
        } else {
            self.epochs
                .get(&active_epoch_index)
                .ok_or(Status::internal("active epoch not found"))
        }
    }

    fn start_new_epoch(&mut self) {
        self.active_epoch_index += 1;
        self.epochs
            .insert(self.active_epoch_index, Arc::new(Mutex::new(Epoch::new())));
    }
}

impl Epoch {
    fn new() -> Self {
        Self {
            state: EpochState::Active,
            pending_inputs: 0,
            processed_inputs: vec![],
        }
    }

    fn check_active(&self) -> Result<&Self, Status> {
        if self.state != EpochState::Active {
            Err(Status::invalid_argument("epoch is finished"))
        } else {
            Ok(self)
        }
    }

    fn check_pending_inputs(&self) -> Result<&Self, Status> {
        if self.pending_inputs != 0 {
            Err(Status::invalid_argument("epoch still has pending inputs"))
        } else {
            Ok(self)
        }
    }

    fn check_processed_inputs(&self, processed_input_count: u64) -> Result<&Self, Status> {
        if self.num_processed_inputs() != processed_input_count {
            Err(Status::invalid_argument(format!(
                "incorrect processed input count (expected {}, got {})",
                self.num_processed_inputs(),
                processed_input_count
            )))
        } else {
            Ok(self)
        }
    }

    fn check_current_input_index(&self, current_input_index: u64) -> Result<&Self, Status> {
        let epoch_current_input_index = self.pending_inputs + self.num_processed_inputs();
        if epoch_current_input_index != current_input_index {
            Err(Status::invalid_argument(format!(
                "incorrect current input index (expected {}, got {})",
                epoch_current_input_index, current_input_index
            )))
        } else {
            Ok(self)
        }
    }

    fn num_processed_inputs(&self) -> u64 {
        self.processed_inputs.len() as u64
    }

    fn add_pending_input(&mut self) {
        self.pending_inputs += 1;
    }

    fn add_processed_input(&mut self, input: Input) {
        self.pending_inputs -= 1;
        self.processed_inputs.push(input);
    }

    fn finish(&mut self) {
        self.state = EpochState::Finished;
    }
}

impl Finisher {
    fn new(epoch: Arc<Mutex<Epoch>>, tainted: Arc<Mutex<Option<Status>>>) -> Self {
        Self { epoch, tainted }
    }
}

#[async_trait]
impl AdvanceFinisher for Finisher {
    async fn handle(&self, result: Result<Input, AdvanceError>) {
        match result {
            Ok(input) => {
                self.epoch.lock().await.add_processed_input(input);
            }
            Err(e) => {
                log::error!("something went wrong; tainting session");
                *self.tainted.lock().await = Some(Status::internal(e.to_string()));
            }
        }
    }
}
