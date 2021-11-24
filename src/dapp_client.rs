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

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use std::{error::Error, fmt};

use super::config::Config;
use super::model::{AdvanceRequest, InspectRequest, Report};
use super::proxy::DApp;

/// HTTP client for the DApp backend
pub struct DAppClient {
    url: String,
    client: Client,
}

impl DAppClient {
    pub fn new(config: &Config) -> Self {
        Self {
            url: format!(
                "http://{}:{}",
                config.dapp_http_address, config.dapp_http_address
            ),
            client: Client::new(),
        }
    }
}

#[async_trait]
impl DApp for DAppClient {
    async fn advance(&self, request: AdvanceRequest) -> Result<(), Box<dyn Error + Send + Sync>> {
        let response = self.client.post(&self.url).json(&request).send().await?;
        check_status(StatusCode::ACCEPTED, response.status())
    }

    async fn inspect(
        &self,
        request: InspectRequest,
    ) -> Result<Vec<Report>, Box<dyn Error + Send + Sync>> {
        let response = self.client.get(&self.url).json(&request).send().await?;
        check_status(StatusCode::OK, response.status())?;
        Ok(response.json().await?)
    }
}

#[derive(Debug)]
pub struct DAppError {
    expected: StatusCode,
    obtained: StatusCode,
}

impl Error for DAppError {}

impl fmt::Display for DAppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "wrong status in http call; expected {} but got {}",
            self.expected, self.obtained
        )
    }
}

fn check_status(
    expected: StatusCode,
    obtained: StatusCode,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if expected != obtained {
        Err(Box::new(DAppError { expected, obtained }))
    } else {
        Ok(())
    }
}
