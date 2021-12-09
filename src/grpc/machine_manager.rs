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

use crate::controller::{AdvanceError, AdvanceFinisher, Controller};
use crate::conversions;
use crate::model::{
    AdvanceMetadata, AdvanceRequest, AdvanceResult, FinishStatus, Notice, Report, Voucher,
};

use super::proto::cartesi_machine::Void;
use super::proto::rollup_machine_manager::rollup_machine_manager_server::RollupMachineManager;
use super::proto::rollup_machine_manager::{
    processed_input::ProcessedOneof, Address, AdvanceStateRequest, CompletionStatus,
    EndSessionRequest, EpochState, FinishEpochRequest, GetEpochStatusRequest,
    GetEpochStatusResponse, GetSessionStatusRequest, GetSessionStatusResponse, GetStatusResponse,
    InputResult, InspectStateRequest, InspectStateResponse, Notice as GrpcNotice, ProcessedInput,
    Report as GrpcReport, StartSessionRequest, TaintStatus, Voucher as GrpcVoucher,
};
use super::proto::versioning::{GetVersionResponse, SemanticVersion};

pub struct RollupMachineManagerService {
    controller: Controller,
    sessions: SessionManager,
}

impl RollupMachineManagerService {
    pub fn new(controller: Controller) -> Self {
        Self {
            controller,
            sessions: SessionManager::new(),
        }
    }
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
        self.sessions
            .try_set_session(
                request.session_id,
                request.active_epoch_index,
                self.controller.clone(),
            )
            .await?;
        Ok(Response::new(Void {}))
    }

    async fn end_session(
        &self,
        request: Request<EndSessionRequest>,
    ) -> Result<Response<Void>, Status> {
        let request = request.into_inner();
        log::info!("received end_session with id={}", request.session_id);
        self.sessions.try_del_session(&request.session_id).await?;
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
        self.sessions
            .try_get_session(&request.session_id)
            .await?
            .try_lock()
            .or(Err(Status::aborted("concurrent call in session")))?
            .try_advance(
                request.active_epoch_index,
                request.current_input_index,
                advance_request,
            )
            .await?;
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
        self.sessions
            .try_get_session(&request.session_id)
            .await?
            .try_lock()
            .or(Err(Status::aborted("concurrent call in session")))?
            .try_finish_epoch(request.active_epoch_index, request.processed_input_count)
            .await?;
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
        let session_id = self.sessions.get_sessions().await;
        Ok(Response::new(GetStatusResponse { session_id }))
    }

    async fn get_session_status(
        &self,
        request: Request<GetSessionStatusRequest>,
    ) -> Result<Response<GetSessionStatusResponse>, Status> {
        let request = request.into_inner();
        log::info!("received get_session_status with id={}", request.session_id);
        let response = self
            .sessions
            .try_get_session(&request.session_id)
            .await?
            .try_lock()
            .or(Err(Status::aborted("concurrent call in session")))?
            .get_status(request.session_id)
            .await;
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
        let response = self
            .sessions
            .try_get_session(&request.session_id)
            .await?
            .try_lock()
            .or(Err(Status::aborted("concurrent call in session")))?
            .try_get_epoch_status(request.session_id, request.epoch_index)
            .await?;
        Ok(Response::new(response))
    }
}

struct SessionManager {
    entry: Mutex<Option<SessionEntry>>, // The mock supports only a single session
}

impl SessionManager {
    fn new() -> Self {
        Self {
            entry: Mutex::new(None),
        }
    }

    async fn try_set_session(
        &self,
        session_id: String,
        active_epoch_index: u64,
        controller: Controller,
    ) -> Result<(), Status> {
        if session_id == "" {
            return Err(Status::invalid_argument("session id is empty"));
        }
        let mut entry = self.entry.lock().await;
        match *entry {
            Some(_) => {
                log::warn!("the mock only supports a single session");
                Err(Status::already_exists("session id is taken"))
            }
            None => {
                *entry = Some(SessionEntry::new(
                    session_id,
                    active_epoch_index,
                    controller,
                ));
                Ok(())
            }
        }
    }

    async fn try_get_session(&self, request_id: &String) -> Result<Arc<Mutex<Session>>, Status> {
        self.entry
            .lock()
            .await
            .as_ref()
            .and_then(|entry| entry.get_session(request_id))
            .ok_or(Status::invalid_argument("session id not found"))
    }

    async fn try_del_session(&self, request_id: &String) -> Result<(), Status> {
        self.try_get_session(&request_id)
            .await?
            .try_lock()
            .or(Err(Status::aborted("concurrent call in session")))?
            .check_endable()
            .await?;
        let mut entry = self.entry.lock().await;
        *entry = None;
        Ok(())
    }

    async fn get_sessions(&self) -> Vec<String> {
        let mut sessions = Vec::new();
        if let Some(entry) = self.entry.lock().await.as_ref() {
            sessions.push(entry.get_id());
        }
        sessions
    }
}

struct SessionEntry {
    id: String,
    session: Arc<Mutex<Session>>,
}

impl SessionEntry {
    fn new(id: String, active_epoch_index: u64, controller: Controller) -> Self {
        Self {
            id,
            session: Arc::new(Mutex::new(Session::new(active_epoch_index, controller))),
        }
    }

    fn get_session(&self, request_id: &String) -> Option<Arc<Mutex<Session>>> {
        if &self.id == request_id {
            Some(self.session.clone())
        } else {
            None
        }
    }

    fn get_id(&self) -> String {
        self.id.clone()
    }
}

struct Session {
    controller: Controller,
    active_epoch_index: u64,
    epochs: HashMap<u64, Arc<Mutex<Epoch>>>,
    tainted: Arc<Mutex<Option<Status>>>,
}

impl Session {
    fn new(active_epoch_index: u64, controller: Controller) -> Self {
        Self {
            controller,
            active_epoch_index,
            epochs: HashMap::from([(active_epoch_index, Arc::new(Mutex::new(Epoch::new())))]),
            tainted: Arc::new(Mutex::new(None)),
        }
    }

    async fn try_advance(
        &mut self,
        active_epoch_index: u64,
        current_input_index: u64,
        advance_request: AdvanceRequest,
    ) -> Result<(), Status> {
        self.check_epoch_index_overflow()?;
        self.check_tainted().await?;
        self.check_active_epoch(active_epoch_index)?;
        let epoch = self.try_get_epoch(active_epoch_index)?;
        epoch
            .lock()
            .await
            .try_add_pending_input(current_input_index)?;
        let finisher = Finisher::new(epoch.clone(), self.tainted.clone());
        self.controller
            .advance(advance_request, Box::new(finisher))
            .await;
        Ok(())
    }

    async fn try_finish_epoch(
        &mut self,
        active_epoch_index: u64,
        processed_input_count: u64,
    ) -> Result<(), Status> {
        self.check_epoch_index_overflow()?;
        self.check_tainted().await?;
        self.check_active_epoch(active_epoch_index)?;
        self.try_get_epoch(active_epoch_index)?
            .lock()
            .await
            .try_finish(processed_input_count)?;
        self.active_epoch_index += 1;
        self.epochs
            .insert(self.active_epoch_index, Arc::new(Mutex::new(Epoch::new())));
        Ok(())
    }

    async fn get_status(&self, session_id: String) -> GetSessionStatusResponse {
        GetSessionStatusResponse {
            session_id,
            active_epoch_index: self.active_epoch_index,
            epoch_index: self.epochs.keys().cloned().collect(),
            taint_status: self.get_taint_status().await,
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

    async fn try_get_epoch_status(
        &self,
        session_id: String,
        epoch_index: u64,
    ) -> Result<GetEpochStatusResponse, Status> {
        self.try_get_epoch(epoch_index)?
            .lock()
            .await
            .try_get_status(session_id, epoch_index, self.get_taint_status().await)
            .await
    }

    fn try_get_epoch(&self, epoch_index: u64) -> Result<&Arc<Mutex<Epoch>>, Status> {
        self.epochs
            .get(&epoch_index)
            .ok_or(Status::invalid_argument("unknown epoch index"))
    }

    async fn check_endable(&self) -> Result<(), Status> {
        if self.tainted.lock().await.is_none() {
            self.try_get_epoch(self.active_epoch_index)?
                .lock()
                .await
                .check_endable()?;
        }
        Ok(())
    }

    async fn check_tainted(&self) -> Result<(), Status> {
        if self.tainted.lock().await.is_some() {
            Err(Status::data_loss("session is tainted"))
        } else {
            Ok(())
        }
    }

    fn check_epoch_index_overflow(&self) -> Result<(), Status> {
        if self.active_epoch_index == std::u64::MAX {
            Err(Status::out_of_range("active epoch index will overflow"))
        } else {
            Ok(())
        }
    }

    fn check_active_epoch(&self, active_epoch_index: u64) -> Result<(), Status> {
        if self.active_epoch_index != active_epoch_index {
            Err(Status::invalid_argument(format!(
                "incorrect active epoch index (expected {}, got {})",
                self.active_epoch_index, active_epoch_index
            )))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
struct Epoch {
    state: EpochState,
    pending_inputs: u64,
    processed_inputs: Vec<AdvanceResult>,
}

impl Epoch {
    fn new() -> Self {
        Self {
            state: EpochState::Active,
            pending_inputs: 0,
            processed_inputs: vec![],
        }
    }

    fn try_add_pending_input(&mut self, current_input_index: u64) -> Result<(), Status> {
        self.check_active()?;
        self.check_current_input_index(current_input_index)?;
        self.pending_inputs += 1;
        Ok(())
    }

    fn add_processed_input(&mut self, input: AdvanceResult) {
        self.pending_inputs -= 1;
        self.processed_inputs.push(input);
    }

    fn try_finish(&mut self, processed_input_count: u64) -> Result<(), Status> {
        self.check_active()?;
        self.check_pending_inputs()?;
        self.check_processed_inputs(processed_input_count)?;
        self.state = EpochState::Finished;
        Ok(())
    }

    async fn try_get_status(
        &self,
        session_id: String,
        epoch_index: u64,
        taint_status: Option<TaintStatus>,
    ) -> Result<GetEpochStatusResponse, Status> {
        let mut processed_inputs: Vec<ProcessedInput> = Vec::new();
        for (i, result) in self.processed_inputs.iter().enumerate() {
            processed_inputs.push(result.to_grpc(i as u64)?);
        }
        Ok(GetEpochStatusResponse {
            session_id,
            epoch_index,
            state: self.state as i32,
            most_recent_machine_hash: None,
            most_recent_vouchers_epoch_root_hash: None,
            most_recent_notices_epoch_root_hash: None,
            processed_inputs,
            pending_input_count: self.pending_inputs,
            taint_status,
        })
    }

    fn get_num_processed_inputs(&self) -> u64 {
        self.processed_inputs.len() as u64
    }

    fn check_endable(&self) -> Result<(), Status> {
        self.check_pending_inputs()?;
        self.check_no_processed_inputs()?;
        Ok(())
    }

    fn check_active(&self) -> Result<(), Status> {
        if self.state != EpochState::Active {
            Err(Status::invalid_argument("epoch is finished"))
        } else {
            Ok(())
        }
    }

    fn check_current_input_index(&self, current_input_index: u64) -> Result<(), Status> {
        let epoch_current_input_index = self.pending_inputs + self.get_num_processed_inputs();
        if epoch_current_input_index != current_input_index {
            Err(Status::invalid_argument(format!(
                "incorrect current input index (expected {}, got {})",
                epoch_current_input_index, current_input_index
            )))
        } else {
            Ok(())
        }
    }

    fn check_pending_inputs(&self) -> Result<(), Status> {
        if self.pending_inputs != 0 {
            Err(Status::invalid_argument("epoch still has pending inputs"))
        } else {
            Ok(())
        }
    }

    fn check_processed_inputs(&self, processed_input_count: u64) -> Result<(), Status> {
        if self.get_num_processed_inputs() != processed_input_count {
            Err(Status::invalid_argument(format!(
                "incorrect processed input count (expected {}, got {})",
                self.get_num_processed_inputs(),
                processed_input_count
            )))
        } else {
            Ok(())
        }
    }

    fn check_no_processed_inputs(&self) -> Result<(), Status> {
        if self.get_num_processed_inputs() != 0 {
            Err(Status::invalid_argument("epoch still has processed inputs"))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
struct Finisher {
    epoch: Arc<Mutex<Epoch>>,
    tainted: Arc<Mutex<Option<Status>>>,
}

impl Finisher {
    fn new(epoch: Arc<Mutex<Epoch>>, tainted: Arc<Mutex<Option<Status>>>) -> Self {
        Self { epoch, tainted }
    }
}

#[async_trait]
impl AdvanceFinisher for Finisher {
    async fn handle(&self, result: Result<AdvanceResult, AdvanceError>) {
        match result {
            Ok(result) => {
                self.epoch.lock().await.add_processed_input(result);
            }
            Err(e) => {
                log::error!("something went wrong; tainting session");
                *self.tainted.lock().await = Some(Status::internal(e.to_string()));
            }
        }
    }
}

impl AdvanceResult {
    fn to_grpc(&self, input_index: u64) -> Result<ProcessedInput, Status> {
        let mut vouchers: Vec<GrpcVoucher> = Vec::new();
        for voucher in self.vouchers.iter() {
            vouchers.push(voucher.value.to_grpc()?);
        }
        let mut notices: Vec<GrpcNotice> = Vec::new();
        for notice in self.notices.iter() {
            notices.push(notice.value.to_grpc()?);
        }
        let mut reports: Vec<GrpcReport> = Vec::new();
        for report in self.reports.iter() {
            reports.push(report.to_grpc());
        }
        let processed_oneof = Some(match self.status {
            FinishStatus::Accept => ProcessedOneof::Result(InputResult {
                voucher_hashes_in_machine: None,
                vouchers,
                notice_hashes_in_machine: None,
                notices,
            }),
            FinishStatus::Reject => {
                ProcessedOneof::SkipReason(CompletionStatus::RejectedByMachine as i32)
            }
        });
        Ok(ProcessedInput {
            input_index,
            most_recent_machine_hash: None,
            voucher_hashes_in_epoch: None,
            notice_hashes_in_epoch: None,
            reports,
            processed_oneof,
        })
    }
}

impl Voucher {
    fn to_grpc(&self) -> Result<GrpcVoucher, Status> {
        Ok(GrpcVoucher {
            keccak: None,
            address: Some(Address {
                data: decode_ethereum_binary(&self.address)?,
            }),
            payload: decode_ethereum_binary(&self.payload)?,
            keccak_in_voucher_hashes: None,
        })
    }
}

impl Notice {
    fn to_grpc(&self) -> Result<GrpcNotice, Status> {
        Ok(GrpcNotice {
            keccak: None,
            payload: decode_ethereum_binary(&self.payload)?,
            keccak_in_notice_hashes: None,
        })
    }
}

impl Report {
    fn to_grpc(&self) -> GrpcReport {
        GrpcReport {
            payload: self.payload.as_bytes().iter().copied().collect(),
        }
    }
}

fn decode_ethereum_binary(s: &str) -> Result<Vec<u8>, Status> {
    conversions::decode_ethereum_binary(s).map_err(|e| {
        log::warn!("failed to convert Eth binary string from DApp ({})", e);
        Status::aborted("failed to convert Eth binary string from DApp")
    })
}
