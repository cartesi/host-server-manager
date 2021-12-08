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

use structopt::StructOpt;

const DEFAULT_ADDRESS: &str = "0.0.0.0";

#[derive(StructOpt, Clone, Debug)]
#[structopt(name = "mock-rollups-machine-manager")]
pub struct Config {
    /// gRPC address of the Mock Rollups Machine Manager endpoint
    #[structopt(long, env, default_value = DEFAULT_ADDRESS)]
    pub grpc_machine_manager_address: String,

    /// gRPC port of the Mock Rollups Machine Manager endpoint
    #[structopt(long, env, default_value = "5000")]
    pub grpc_machine_manager_port: u16,

    /// HTTP address of the Target Proxy endpoint
    #[structopt(long, env, default_value = DEFAULT_ADDRESS)]
    pub http_target_proxy_address: String,

    /// HTTP port of the Target Proxy endpoint
    #[structopt(long, env, default_value = "5001")]
    pub http_target_proxy_port: u16,

    /// HTTP address of the Inspect endpoint
    #[structopt(long, env, default_value = DEFAULT_ADDRESS)]
    pub http_inspect_address: String,

    /// HTTP port of the Inspect endpoint
    #[structopt(long, env, default_value = "5002")]
    pub http_inspect_port: u16,

    /// HTTP address of the DApp backend
    #[structopt(long, env, default_value = DEFAULT_ADDRESS)]
    pub dapp_http_address: String,

    /// HTTP port of the DApp backend
    #[structopt(long, env, default_value = "5003")]
    pub dapp_http_port: u16,
}
