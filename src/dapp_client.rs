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
use snafu::Snafu;
use std::error::Error;

use crate::config::Config;
use crate::controller::DApp;
use crate::conversions;
use crate::http::model::{HttpAdvanceRequest, HttpInspectResponse};
use crate::model::{AdvanceRequest, InspectRequest, Report};

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
        let url = format!("http://{}:{}/advance", self.address, self.port);
        let request = HttpAdvanceRequest::from(request);
        let response = self.client.post(url).json(&request).send().await?;
        check_status(&response, StatusCode::ACCEPTED)
    }

    async fn inspect(
        &self,
        request: InspectRequest,
    ) -> Result<Vec<Report>, Box<dyn Error + Send + Sync>> {
        let payload = conversions::encode_ethereum_binary(&request.payload);
        let url = format!("http://{}:{}/inspect/{}", self.address, self.port, payload);
        let response = self.client.get(url).send().await?;
        check_status(&response, StatusCode::OK)?;
        let response = response.json::<HttpInspectResponse>().await?;
        let reports = response
            .reports
            .into_iter()
            .map(Report::try_from)
            .collect::<Result<Vec<Report>, _>>()?;
        Ok(reports)
    }
}

#[derive(Debug, Snafu)]
#[snafu(display(
    "wrong status in HTTP call (expected {} but got {})",
    expected,
    obtained
))]
pub struct UnexpectedStatusError {
    expected: StatusCode,
    obtained: StatusCode,
}

fn check_status(
    response: &Response,
    expected: StatusCode,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if response.status() != expected {
        Err(Box::new(UnexpectedStatusError {
            expected,
            obtained: response.status(),
        }))
    } else {
        Ok(())
    }
}
