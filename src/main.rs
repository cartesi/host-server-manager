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

mod config;
mod conversions;
mod dapp_client;
mod grpc;
mod http;
mod model;
mod proxy;

use futures_util::FutureExt;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use tokio::sync::oneshot;

use config::Config;
use dapp_client::DAppClient;

#[actix_web::main]
async fn main() {
    // Set the default log level to info
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = Config::new();
    let dapp_client = Box::new(DAppClient::new(&config));
    let (proxy_channel, proxy_service) = proxy::new(dapp_client);
    let proxy_service = tokio::spawn(async {
        proxy_service.run().await;
        log::info!("proxy service terminated successfully");
    });
    let http_service_running = Arc::new(AtomicBool::new(true));
    let (grpc_shutdown_tx, grpc_shutdown_rx) = oneshot::channel::<()>();
    let grpc_service = {
        let proxy_channel = proxy_channel.clone();
        let config = config.clone();
        let shutdown = grpc_shutdown_rx.map(|_| ());
        let http_service_running = http_service_running.clone();
        tokio::spawn(async move {
            match grpc::start_service(&config, proxy_channel.clone(), shutdown).await {
                Ok(_) => log::info!("grpc service terminated successfully"),
                Err(e) => log::warn!("grpc service terminated with error: {}", e),
            }
            if http_service_running.load(Ordering::Relaxed) {
                panic!("gRPC service terminated before shutdown signal");
            }
        })
    };

    // We run the actix-web in the main thread because it handles the SIGINT
    match http::start_services(&config, proxy_channel.clone()).await {
        Ok(_) => log::info!("http service terminated successfully"),
        Err(e) => log::warn!("http service terminated with error: {}", e),
    }
    http_service_running.store(false, Ordering::Relaxed);

    // Shutdown the other services
    proxy_channel.shutdown().await;
    proxy_service
        .await
        .expect("failed to shutdown the proxy service");
    grpc_shutdown_tx
        .send(())
        .expect("failed to send shutdown signal to grpc");
    grpc_service
        .await
        .expect("failed to shutdown the grpc service");
}
