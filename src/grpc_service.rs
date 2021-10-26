// Copyright 2021 Cartesi Pte. Ltd.
//
// This file is part of the machine-emulator. The machine-emulator is free
// software: you can redistribute it and/or modify it under the terms of the GNU
// Lesser General Public License as published by the Free Software Foundation,
// either version 3 of the License, or (at your option) any later version.
//
// The machine-emulator is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE. See the GNU Lesser General Public License
// for more details.
//
// You should have received a copy of the GNU Lesser General Public License
// along with the machine-emulator. If not, see http://www.gnu.org/licenses/.

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
