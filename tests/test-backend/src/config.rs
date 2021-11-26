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

const BACKEND_ADDRESS_ENV: &'static str = "CARTESI_DAPP_BACKEND_ADDRESS";
const MANAGER_ADDRESS_ENV: &'static str = "CARTESI_MANAGER_MOCK_ADDRESS";

const BACKEND_ADDRESS_DEFAULT: &'static str = "127.0.0.1:27017";
const MANAGER_ADDRESS_DEFAULT: &'static str = "127.0.0.1:27018";

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub backend_address: String,
    pub manager_address: String,
}

impl Config {
    fn fetch_env_var(env_var: &str, default_value: &str) -> String {
        env::var(env_var).unwrap_or_else(|e| {
            log::info!(
                "Error fetching `{}`: {}. Defaulting to {}",
                env_var,
                e,
                default_value
            );
            env_var.to_string()
        })
    }
    pub fn create() -> Self {
        let backend_address = Config::fetch_env_var(BACKEND_ADDRESS_ENV, BACKEND_ADDRESS_DEFAULT);
        let manager_address = Config::fetch_env_var(MANAGER_ADDRESS_ENV, MANAGER_ADDRESS_DEFAULT);

        Self {
            backend_address,
            manager_address,
        }
    }
}
