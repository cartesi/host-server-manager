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
use reqwest::{Client, Response, StatusCode};
use std::{error::Error, fmt};

use super::config::Config;
use super::model::{AdvanceRequest, InspectRequest, InspectResponse, Report};
use super::proxy::DApp;

/// HTTP client for the DApp backend
pub struct DAppClient {
    address: String,
    port: u16,
    client: Client,
}

impl DAppClient {
    pub fn new(config: &Config) -> Self {
        Self {
            address: config.dapp_http_address.clone(),
            port: config.dapp_http_port,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl DApp for DAppClient {
    async fn advance(&self, request: AdvanceRequest) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.client
            .post(format!("http://{}:{}/advance", self.address, self.port))
            .json(&request)
            .send()
            .await
            .map_err(|e| e.into())
            .and_then(check_status(StatusCode::ACCEPTED))
            .map(|_| ())
    }

    async fn inspect(
        &self,
        request: InspectRequest,
    ) -> Result<Vec<Report>, Box<dyn Error + Send + Sync>> {
        let response = self
            .client
            .get(format!(
                "http://{}:{}/inspect/{}",
                self.address, self.port, request.payload
            ))
            .send()
            .await
            .map_err(|e| e.into())
            .and_then(check_status(StatusCode::OK))?;
        response
            .json::<InspectResponse>()
            .await
            .map(|inspect_response| inspect_response.reports)
            .map_err(|e| e.into())
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
) -> impl FnOnce(Response) -> Result<Response, Box<dyn Error + Send + Sync>> {
    move |response| {
        if expected != response.status() {
            Err(Box::new(DAppError {
                expected,
                obtained: response.status(),
            }))
        } else {
            Ok(response)
        }
    }
}
