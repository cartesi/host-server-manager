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
mod controller;
mod http_service;
mod model;

use crate::config::Config;
use controller::new_controller;
use model::Model;

#[actix_web::main]
async fn main() {
    // Set the default log level to debug
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let config = Config::create();
    let model = Box::new(Model {});
    let (channel, service) = new_controller(model, config.clone());

    tokio::spawn(async {
        service.run().await;
        log::info!("proxy service terminated successfully");
    });

    match http_service::run(&config, channel).await {
        Ok(_) => log::info!("http service terminated successfully"),
        Err(e) => log::warn!("http service terminated with error: {}", e),
    }
}
