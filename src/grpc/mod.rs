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

mod proto;
mod server_manager;

use futures_util::FutureExt;
use std::future::Future;
use tonic::transport::Server;

use proto::server_manager::server_manager_server::ServerManagerServer;
use server_manager::ServerManagerService;

use crate::config::Config;
use crate::dapp_client::Controller;

pub async fn start_service<F: Future<Output = ()>>(
    config: &Config,
    controller: Controller,
    signal: F,
) -> Result<(), tonic::transport::Error> {
    let addr = format!(
        "{}:{}",
        config.grpc_server_manager_address, config.grpc_server_manager_port
    )
    .parse()
    .expect("invalid config");
    let service = ServerManagerService::new(controller);
    Server::builder()
        .add_service(ServerManagerServer::new(service))
        .serve_with_shutdown(addr, signal.map(|_| ()))
        .await
}
