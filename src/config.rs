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

pub struct Config {
    pub proxy_http_address: &'static str,
    pub proxy_http_port: u16,
    pub dapp_http_address: &'static str,
    pub dapp_http_port: u16,
}

impl Config {
    pub fn new() -> Self {
        Self {
            proxy_http_address: "127.0.0.1",
            proxy_http_port: 5555,
            dapp_http_address: "127.0.0.1",
            dapp_http_port: 6666,
        }
    }
}
