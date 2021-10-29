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

use tokio::sync::mpsc;
use tonic::{transport::Server, Request, Response, Status};

use super::model::AdvanceRequest;

use cartesi_machine::Void;
use rollup_machine_manager::rollup_machine_manager_server::{
    RollupMachineManager, RollupMachineManagerServer,
};
use rollup_machine_manager::{
    EndSessionRequest, EnqueueInputRequest, FinishEpochRequest, GetEpochStatusRequest,
    GetEpochStatusResponse, GetSessionStatusRequest, GetSessionStatusResponse, GetStatusResponse,
    StartSessionRequest,
};
use versioning::{GetVersionResponse, SemanticVersion};

pub mod rollup_machine_manager {
    tonic::include_proto!("cartesi_rollup_machine_manager");
}

pub mod cartesi_machine {
    tonic::include_proto!("cartesi_machine");
}

pub mod versioning {
    tonic::include_proto!("versioning");
}

#[derive(Debug)]
struct RollupMachineManagerService {
    advance_state_tx: mpsc::Sender<AdvanceRequest>,
}

impl RollupMachineManagerService {
    fn new(advance_state_tx: mpsc::Sender<AdvanceRequest>) -> Self {
        Self { advance_state_tx }
    }
}

#[tonic::async_trait]
impl RollupMachineManager for RollupMachineManagerService {
    async fn get_version(
        &self,
        _request: Request<Void>,
    ) -> Result<Response<GetVersionResponse>, Status> {
        let response = GetVersionResponse {
            version: Some(SemanticVersion {
                major: 0,
                minor: 1,
                patch: 0,
                pre_release: String::from(""),
                build: String::from("mock-rollup-machine-manager"),
            }),
        };

        Ok(Response::new(response))
    }

    async fn start_session(
        &self,
        _request: Request<StartSessionRequest>,
    ) -> Result<Response<Void>, Status> {
        unimplemented!()
    }

    async fn enqueue_input(
        &self,
        _request: Request<EnqueueInputRequest>,
    ) -> Result<Response<Void>, Status> {
        unimplemented!()
    }

    async fn get_status(
        &self,
        _request: Request<Void>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        unimplemented!()
    }

    async fn get_session_status(
        &self,
        _request: Request<GetSessionStatusRequest>,
    ) -> Result<Response<GetSessionStatusResponse>, Status> {
        unimplemented!()
    }

    async fn get_epoch_status(
        &self,
        _request: Request<GetEpochStatusRequest>,
    ) -> Result<Response<GetEpochStatusResponse>, Status> {
        unimplemented!()
    }

    async fn finish_epoch(
        &self,
        _request: Request<FinishEpochRequest>,
    ) -> Result<Response<Void>, Status> {
        unimplemented!()
    }

    async fn end_session(
        &self,
        _request: Request<EndSessionRequest>,
    ) -> Result<Response<Void>, Status> {
        unimplemented!()
    }
}

pub async fn run(
    advance_state_tx: mpsc::Sender<AdvanceRequest>,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::]:50051".parse()?;
    let service = RollupMachineManagerService::new(advance_state_tx);

    Server::builder()
        .add_service(RollupMachineManagerServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
