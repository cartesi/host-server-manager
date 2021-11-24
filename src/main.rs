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
mod conversions;
mod dapp_client;
mod grpc_proto;
mod grpc_service;
mod http_service;
mod model;
mod proxy;

use config::Config;
use dapp_client::DAppClient;

#[actix_web::main]
async fn main() {
    let config = Config::new();
    let dapp_client = Box::new(DAppClient::new(&config));
    let (proxy_channel, proxy_service) = proxy::new(dapp_client);
    let proxy_service = proxy_service.run();
    let http_service = http_service::run(&config, proxy_channel.clone());
    let grpc_service = grpc_service::run(&config, proxy_channel);

    // Set the default log level to info
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tokio::select! {
        _ = proxy_service => {
            log::info!("proxy service terminated with success");
        }
        result = http_service => {
            match result {
                Ok(_) => log::info!("http service terminated successfully"),
                Err(e) => log::warn!("http service terminated with error: {}", e),
            }
        }
        result = grpc_service => {
            match result {
                Ok(_) => log::info!("grpc service terminated successfully"),
                Err(e) => log::warn!("grpc service terminated with error: {}", e),
            }
        }
    }
}
