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
use tokio::sync::{mpsc, oneshot};

use crate::config::Config;
use crate::controller::DApp;
use crate::conversions;
use crate::http::model::{HttpAdvanceRequest, HttpInspectResponse};
use crate::model::{AdvanceRequest, InspectRequest, Report};
use crate::sync_request::SyncRequest;

pub type Controller = crate::controller::Controller<DAppClient>;

const BUFFER_SIZE: usize = 1000;

/// Send HTTP requests to the DApp backend.
/// This struct manipulates the data inside an independent thread that can only communicate through
/// channels.
#[derive(Debug)]
pub struct DAppClient {
    advance_tx: mpsc::Sender<SyncAdvanceRequest>,
    inspect_tx: mpsc::Sender<SyncInspectRequest>,
    shutdown_tx: mpsc::Sender<SyncShutdownRequest>,
}

impl DAppClient {
    pub fn new(config: &Config) -> Self {
        let (advance_tx, advance_rx) = mpsc::channel(BUFFER_SIZE);
        let (inspect_tx, inspect_rx) = mpsc::channel(BUFFER_SIZE);
        let (shutdown_tx, shutdown_rx) = mpsc::channel(BUFFER_SIZE);
        let service = Service::new(advance_rx, inspect_rx, shutdown_rx, config);
        tokio::spawn(service.run());
        Self {
            advance_tx,
            inspect_tx,
            shutdown_tx,
        }
    }
}

#[async_trait]
impl DApp for DAppClient {
    type Error = DAppError;

    async fn advance(&self, request: AdvanceRequest) -> oneshot::Receiver<Result<(), DAppError>> {
        SyncRequest::send(&self.advance_tx, request).await
    }

    async fn inspect(
        &self,
        request: InspectRequest,
    ) -> oneshot::Receiver<Result<Vec<Report>, DAppError>> {
        SyncRequest::send(&self.inspect_tx, request).await
    }

    async fn shutdown(&self) -> oneshot::Receiver<()> {
        SyncRequest::send(&self.shutdown_tx, ()).await
    }
}

#[derive(Debug, Snafu)]
pub enum DAppError {
    #[snafu(display(
        "wrong status in HTTP call (expected {} but got {})",
        expected,
        obtained
    ))]
    UnexpectedStatusError {
        expected: StatusCode,
        obtained: StatusCode,
    },
    #[snafu(display("HTTP request error ({})", e))]
    ReqwestError { e: reqwest::Error },
    #[snafu(display("Hex decode error ({})", e))]
    HexDecodeError { e: conversions::DecodeError },
}

type SyncAdvanceRequest = SyncRequest<AdvanceRequest, Result<(), DAppError>>;
type SyncInspectRequest = SyncRequest<InspectRequest, Result<Vec<Report>, DAppError>>;
type SyncShutdownRequest = SyncRequest<(), ()>;

struct Service {
    advance_rx: mpsc::Receiver<SyncAdvanceRequest>,
    inspect_rx: mpsc::Receiver<SyncInspectRequest>,
    shutdown_rx: mpsc::Receiver<SyncShutdownRequest>,
    address: String,
    port: u16,
    client: Client,
}

impl Service {
    fn new(
        advance_rx: mpsc::Receiver<SyncAdvanceRequest>,
        inspect_rx: mpsc::Receiver<SyncInspectRequest>,
        shutdown_rx: mpsc::Receiver<SyncShutdownRequest>,
        config: &Config,
    ) -> Self {
        Self {
            advance_rx,
            inspect_rx,
            shutdown_rx,
            address: config.dapp_http_address.clone(),
            port: config.dapp_http_port,
            client: Client::new(),
        }
    }

    async fn run(mut self) {
        loop {
            tokio::select! {
                Some(request) = self.advance_rx.recv() => {
                    request.process_async(|request| self.advance(request)).await;
                }
                Some(request) = self.inspect_rx.recv() => {
                    request.process_async(|request| self.inspect(request)).await;
                }
                Some(request) = self.shutdown_rx.recv() => {
                    log::info!("dapp service terminated successfully");
                    request.process(|_| ());
                    return;
                }
            }
        }
    }

    async fn advance(&self, request: AdvanceRequest) -> Result<(), DAppError> {
        let url = format!("http://{}:{}/advance", self.address, self.port);
        log::info!("making request to {}", url);
        let request = HttpAdvanceRequest::from(request);
        let response = self.client.post(url).json(&request).send().await?;
        check_status(&response, StatusCode::ACCEPTED)
    }

    async fn inspect(&self, request: InspectRequest) -> Result<Vec<Report>, DAppError> {
        let payload = conversions::encode_ethereum_binary(&request.payload);
        let url = format!("http://{}:{}/inspect/{}", self.address, self.port, payload);
        log::info!("making request to {}", url);
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

impl From<reqwest::Error> for DAppError {
    fn from(e: reqwest::Error) -> DAppError {
        DAppError::ReqwestError { e }
    }
}

impl From<conversions::DecodeError> for DAppError {
    fn from(e: conversions::DecodeError) -> DAppError {
        DAppError::HexDecodeError { e }
    }
}

fn check_status(response: &Response, expected: StatusCode) -> Result<(), DAppError> {
    if response.status() != expected {
        Err(DAppError::UnexpectedStatusError {
            expected,
            obtained: response.status(),
        })
    } else {
        Ok(())
    }
}
