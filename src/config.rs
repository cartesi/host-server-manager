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

use std::env;

#[derive(Clone)]
pub struct Config {
    pub grpc_machine_manager_address: String,
    pub grpc_machine_manager_port: u16,
    pub http_target_proxy_address: String,
    pub http_target_proxy_port: u16,
    pub http_inspect_address: String,
    pub http_inspect_port: u16,
    pub dapp_http_address: String,
    pub dapp_http_port: u16,
}

impl Config {
    pub fn new() -> Self {
        Self {
            grpc_machine_manager_address: get_address("CARTESI_GRPC_MACHINE_MANAGER_ADDRESS"),
            grpc_machine_manager_port: get_port("CARTESI_GRPC_MACHINE_MANAGER_PORT", 5000),
            http_target_proxy_address: get_address("CARTESI_HTTP_TARGET_PROXY_ADDRESS"),
            http_target_proxy_port: get_port("CARTESI_HTTP_TARGET_PROXY_PORT", 5001),
            http_inspect_address: get_address("CARTESI_HTTP_INSPECT_ADDRESS"),
            http_inspect_port: get_port("CARTESI_HTTP_INSPECT_PORT", 5002),
            dapp_http_address: get_address("CARTESI_DAPP_HTTP_ADDRESS"),
            dapp_http_port: get_port("CARTESI_DAPP_HTTP_PORT", 5003),
        }
    }
}

fn get_address(var: &str) -> String {
    env::var(var).unwrap_or(String::from("127.0.0.1"))
}

fn get_port(var: &str, default: u16) -> u16 {
    env::var(var)
        .map(|s| {
            s.parse::<u16>()
                .expect(format!("failed to parse variable {}", var).as_str())
        })
        .unwrap_or(default)
}
