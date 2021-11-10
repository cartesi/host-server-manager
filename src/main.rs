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

// TODO remove the followin line
#![allow(dead_code)]

mod config;
mod dapp_client;
mod grpc_proto;
mod grpc_service;
mod http_service;
mod model;
mod proxy;
mod repository;

use std::error::Error;
use tokio;

use config::Config;
use dapp_client::DAppClient;
use repository::MemRepository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = Config::new();
    let dapp_client = Box::new(DAppClient::new());
    let repository = Box::new(MemRepository::new());
    let (proxy_channel, proxy_service) = proxy::new(repository, dapp_client);
    let proxy_service = proxy_service.run();
    let http_service = http_service::run(&config, proxy_channel.clone());
    let grpc_service = grpc_service::run(proxy_channel);
    tokio::try_join!(proxy_service, http_service, grpc_service)?;
    Ok(())
}
