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

use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::controller::Controller;
use crate::hash::Hash;
use crate::merkle_tree::{complete::Tree, proof::Proof, Error as MerkleTreeError};
use crate::model::{
    AdvanceMetadata, AdvanceResult, AdvanceStateRequest, CompletionStatus, Notice, Report, Voucher,
};
use crate::proofs::compute_proofs;

use super::proto::cartesi_machine::{
    Hash as GrpcHash, MerkleTreeProof as GrpcMerkleTreeProof, Void,
};
use super::proto::server_manager::{
    processed_input::ProcessedInputOneOf, server_manager_server::ServerManager, AcceptedData,
    Address, AdvanceStateRequest as GrpcAdvanceStateRequest,
    CompletionStatus as GrpcCompletionStatus, EndSessionRequest, EpochState, FinishEpochRequest,
    GetEpochStatusRequest, GetEpochStatusResponse, GetSessionStatusRequest,
    GetSessionStatusResponse, GetStatusResponse, InspectStateRequest, InspectStateResponse,
    Notice as GrpcNotice, ProcessedInput, Report as GrpcReport, StartSessionRequest,
    StartSessionResponse, TaintStatus, Voucher as GrpcVoucher,
};
use super::proto::versioning::{GetVersionResponse, SemanticVersion};

pub struct ServerManagerService {
    controller: Controller,
    sessions: SessionManager,
}

impl ServerManagerService {
    pub fn new(controller: Controller) -> Self {
        Self {
            controller,
            sessions: SessionManager::new(),
        }
    }
}

#[tonic::async_trait]
impl ServerManager for ServerManagerService {
    async fn get_version(&self, _: Request<Void>) -> Result<Response<GetVersionResponse>, Status> {
        log::info!("received get_version");
        let response = GetVersionResponse {
            version: Some(SemanticVersion {
                major: 0,
                minor: 2,
                patch: 0,
                pre_release: String::from(""),
                build: String::from("host-server-manager"),
            }),
        };
        Ok(Response::new(response))
    }

    async fn start_session(
        &self,
        request: Request<StartSessionRequest>,
    ) -> Result<Response<StartSessionResponse>, Status> {
        let request = request.into_inner();
        log::info!("received start_session with id={}", request.session_id);
        self.sessions
            .try_set_session(
                request.session_id,
                request.active_epoch_index,
                self.controller.clone(),
            )
            .await?;
        let response = StartSessionResponse { config: None };
        Ok(Response::new(response))
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
        request: Request<GrpcAdvanceStateRequest>,
    ) -> Result<Response<Void>, Status> {
        let request = request.into_inner();
        log::info!("received advance_state with id={}", request.session_id);
        let metadata = request
            .input_metadata
            .ok_or(Status::invalid_argument("missing metadata from request"))?;
        let msg_sender = metadata
            .msg_sender
            .ok_or(Status::invalid_argument("missing msg_sender from metadata"))?
            .data
            .try_into()
            .or(Err(Status::invalid_argument("invalid address")))?;
        if metadata.epoch_index != request.active_epoch_index {
            return Err(Status::invalid_argument("metadata epoch index mismatch"));
        }
        if metadata.input_index != request.current_input_index {
            return Err(Status::invalid_argument("metadata input index mismatch"));
        }
        let advance_request = AdvanceStateRequest {
            metadata: AdvanceMetadata {
                msg_sender,
                epoch_index: metadata.epoch_index,
                input_index: metadata.input_index,
                block_number: metadata.block_number,
                timestamp: metadata.timestamp,
            },
            payload: request.input_payload,
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
            "the inspect state should be called from the HTTP API",
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
    entry: Mutex<Option<SessionEntry>>,
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
                log::warn!("the host-server-manager only supports a single session");
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
    active_epoch_index: u64,
    controller: Controller,
    epochs: HashMap<u64, Arc<Mutex<Epoch>>>,
    tainted: Arc<Mutex<Option<Status>>>,
}

impl Session {
    fn new(active_epoch_index: u64, controller: Controller) -> Self {
        let epoch = Arc::new(Mutex::new(Epoch::new()));
        let mut epochs = HashMap::new();
        epochs.insert(active_epoch_index, epoch);
        Self {
            active_epoch_index,
            controller,
            epochs,
            tainted: Arc::new(Mutex::new(None)),
        }
    }

    async fn try_advance(
        &mut self,
        active_epoch_index: u64,
        current_input_index: u64,
        advance_request: AdvanceStateRequest,
    ) -> Result<(), Status> {
        self.check_epoch_index_overflow()?;
        self.check_tainted().await?;
        self.check_active_epoch(active_epoch_index)?;
        let epoch = self.try_get_epoch(active_epoch_index)?;
        epoch
            .lock()
            .await
            .try_add_pending_input(current_input_index)?;
        let rx = self.controller.advance(advance_request).await;
        let epoch = epoch.clone();
        let tainted = self.tainted.clone();
        // Handle the advance response in another thread
        tokio::spawn(async move {
            match rx.await {
                Ok(result) => {
                    if let Err(e) = epoch.lock().await.add_processed_input(result) {
                        log::error!("failed to add processed input; tainting session");
                        *tainted.lock().await = Some(e);
                    }
                }
                Err(_) => {
                    log::error!("sender dropped the channel");
                }
            }
        });
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
        let epoch = Arc::new(Mutex::new(Epoch::new()));
        self.epochs.insert(self.active_epoch_index, epoch);
        Ok(())
    }

    async fn get_status(&self, session_id: String) -> GetSessionStatusResponse {
        let mut epoch_index: Vec<u64> = self.epochs.keys().cloned().collect();
        epoch_index.sort();
        GetSessionStatusResponse {
            session_id,
            active_epoch_index: self.active_epoch_index,
            epoch_index,
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
        let taint_status = self.get_taint_status().await;
        let response = self.try_get_epoch(epoch_index)?.lock().await.get_status(
            session_id,
            epoch_index,
            taint_status,
        );
        Ok(response)
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

/// The keccak output has 32 bytes
const LOG2_KECCAK_SIZE: usize = 5;

/// The epoch tree has 2^32 leafs
const LOG2_ROOT_SIZE: usize = 32 + LOG2_KECCAK_SIZE;

/// The max number of inputs in an epoch is limited by the size of the merkle tree
const MAX_INPUTS_IN_EPOCH: usize = 1 << (LOG2_ROOT_SIZE - LOG2_KECCAK_SIZE);

#[derive(Debug)]
struct Epoch {
    state: EpochState,
    pending_inputs: u64,
    processed_inputs: Vec<AdvanceResult>,
    vouchers_tree: Tree,
    notices_tree: Tree,
}

impl Epoch {
    fn new() -> Self {
        Self {
            state: EpochState::Active,
            pending_inputs: 0,
            processed_inputs: vec![],
            vouchers_tree: Tree::new(LOG2_ROOT_SIZE, LOG2_KECCAK_SIZE, LOG2_KECCAK_SIZE)
                .expect("cannot fail"),
            notices_tree: Tree::new(LOG2_ROOT_SIZE, LOG2_KECCAK_SIZE, LOG2_KECCAK_SIZE)
                .expect("cannot fail"),
        }
    }

    fn try_add_pending_input(&mut self, current_input_index: u64) -> Result<(), Status> {
        self.check_active()?;
        self.check_current_input_index(current_input_index)?;
        self.check_input_limit()?;
        self.pending_inputs += 1;
        Ok(())
    }

    fn add_processed_input(&mut self, mut result: AdvanceResult) -> Result<(), Status> {
        // Compute proofs and update vouchers and notices trees
        if let CompletionStatus::Accepted { vouchers, notices } = &mut result.status {
            let voucher_root = compute_proofs(vouchers)?;
            result.voucher_root = Some(voucher_root.clone());
            self.vouchers_tree.push(voucher_root)?;
            let notice_root = compute_proofs(notices)?;
            result.notice_root = Some(notice_root.clone());
            self.notices_tree.push(notice_root)?;
        } else {
            self.vouchers_tree.push(Hash::default())?;
            self.notices_tree.push(Hash::default())?;
        }
        // Setup proofs for the current result
        let address = (self.vouchers_tree.len() - 1) << LOG2_KECCAK_SIZE;
        result.voucher_hashes_in_epoch =
            Some(self.vouchers_tree.get_proof(address, LOG2_KECCAK_SIZE)?);
        result.notice_hashes_in_epoch =
            Some(self.notices_tree.get_proof(address, LOG2_KECCAK_SIZE)?);
        // Add result to processed inputs
        self.pending_inputs -= 1;
        self.processed_inputs.push(result);
        Ok(())
    }

    fn try_finish(&mut self, processed_input_count: u64) -> Result<(), Status> {
        self.check_active()?;
        self.check_pending_inputs()?;
        self.check_processed_inputs(processed_input_count)?;
        self.state = EpochState::Finished;
        // Re-generate all proofs for processed inputs
        for (i, result) in self.processed_inputs.iter_mut().enumerate() {
            let address = i << LOG2_KECCAK_SIZE;
            result.voucher_hashes_in_epoch =
                Some(self.vouchers_tree.get_proof(address, LOG2_KECCAK_SIZE)?);
            result.notice_hashes_in_epoch =
                Some(self.notices_tree.get_proof(address, LOG2_KECCAK_SIZE)?);
        }
        Ok(())
    }

    fn get_status(
        &self,
        session_id: String,
        epoch_index: u64,
        taint_status: Option<TaintStatus>,
    ) -> GetEpochStatusResponse {
        GetEpochStatusResponse {
            session_id,
            epoch_index,
            state: self.state as i32,
            most_recent_machine_hash: None,
            most_recent_vouchers_epoch_root_hash: Some(
                self.vouchers_tree.get_root_hash().clone().into(),
            ),
            most_recent_notices_epoch_root_hash: Some(
                self.notices_tree.get_root_hash().clone().into(),
            ),
            processed_inputs: self
                .processed_inputs
                .iter()
                .cloned()
                .enumerate()
                .map(|item| item.into())
                .collect(),
            pending_input_count: self.pending_inputs,
            taint_status,
        }
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

    fn check_input_limit(&self) -> Result<(), Status> {
        if self.pending_inputs + self.get_num_processed_inputs() + 1 >= MAX_INPUTS_IN_EPOCH as u64 {
            Err(Status::invalid_argument(
                "reached max number of inputs per epoch",
            ))
        } else {
            Ok(())
        }
    }
}

impl From<(usize, AdvanceResult)> for ProcessedInput {
    fn from((index, result): (usize, AdvanceResult)) -> ProcessedInput {
        ProcessedInput {
            input_index: index as u64,
            most_recent_machine_hash: None,
            voucher_hashes_in_epoch: result
                .voucher_hashes_in_epoch
                .map(GrpcMerkleTreeProof::from),
            notice_hashes_in_epoch: result.notice_hashes_in_epoch.map(GrpcMerkleTreeProof::from),
            status: (&result.status).into(),
            processed_input_one_of: result.status.into(),
            reports: result.reports.into_iter().map(GrpcReport::from).collect(),
        }
    }
}

impl From<&CompletionStatus> for i32 {
    fn from(status: &CompletionStatus) -> i32 {
        let status = match status {
            CompletionStatus::Accepted { .. } => GrpcCompletionStatus::Accepted,
            CompletionStatus::Rejected => GrpcCompletionStatus::Rejected,
            CompletionStatus::Exception { .. } => GrpcCompletionStatus::Exception,
        };
        status as i32
    }
}

impl From<CompletionStatus> for Option<ProcessedInputOneOf> {
    fn from(status: CompletionStatus) -> Option<ProcessedInputOneOf> {
        match status {
            CompletionStatus::Accepted { vouchers, notices } => {
                Some(ProcessedInputOneOf::AcceptedData(AcceptedData {
                    voucher_hashes_in_machine: None,
                    vouchers: vouchers.into_iter().map(GrpcVoucher::from).collect(),
                    notice_hashes_in_machine: None,
                    notices: notices.into_iter().map(GrpcNotice::from).collect(),
                }))
            }
            CompletionStatus::Rejected => None,
            CompletionStatus::Exception { exception } => {
                Some(ProcessedInputOneOf::ExceptionData(exception.payload))
            }
        }
    }
}

impl From<Voucher> for GrpcVoucher {
    fn from(voucher: Voucher) -> GrpcVoucher {
        GrpcVoucher {
            keccak: Some(voucher.keccak.into()),
            address: Some(Address {
                data: voucher.address.into(),
            }),
            payload: voucher.payload,
            keccak_in_voucher_hashes: voucher
                .keccak_in_voucher_hashes
                .map(GrpcMerkleTreeProof::from),
        }
    }
}

impl From<Notice> for GrpcNotice {
    fn from(notice: Notice) -> GrpcNotice {
        GrpcNotice {
            keccak: Some(notice.keccak.into()),
            payload: notice.payload,
            keccak_in_notice_hashes: notice
                .keccak_in_notice_hashes
                .map(GrpcMerkleTreeProof::from),
        }
    }
}

impl From<Report> for GrpcReport {
    fn from(report: Report) -> GrpcReport {
        GrpcReport {
            payload: report.payload,
        }
    }
}

impl From<Hash> for GrpcHash {
    fn from(hash: Hash) -> GrpcHash {
        GrpcHash { data: hash.into() }
    }
}

impl From<Proof> for GrpcMerkleTreeProof {
    fn from(proof: Proof) -> GrpcMerkleTreeProof {
        GrpcMerkleTreeProof {
            target_address: proof.target_address as u64,
            log2_target_size: proof.log2_target_size as u64,
            target_hash: Some(proof.target_hash.into()),
            log2_root_size: proof.log2_root_size as u64,
            root_hash: Some(proof.root_hash.into()),
            sibling_hashes: proof
                .sibling_hashes
                .into_iter()
                .map(GrpcHash::from)
                .collect(),
        }
    }
}

impl From<MerkleTreeError> for Status {
    fn from(e: MerkleTreeError) -> Status {
        Status::internal(format!(
            "unexpected error when updating merkle tree ({})",
            e
        ))
    }
}
