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

use actix_web::ResponseError;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub struct SyncRequest<T: Send + Sync, U: Send + Sync> {
    pub value: T,
    pub response_tx: oneshot::Sender<U>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvanceMetadata {
    pub msg_sender: String,
    pub epoch_index: u64,
    pub input_index: u64,
    pub block_number: u64,
    pub time_stamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvanceRequest {
    pub metadata: AdvanceMetadata,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectRequest {
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectReport {
    pub reports: Vec<InspectRequest>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AdvanceError {
    pub cause: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InspectError {
    pub cause: String,
}

pub type AdvanceResult = Result<(), AdvanceError>;
pub type InspectResult = Result<InspectReport, InspectError>;

pub type SyncInspectRequest = SyncRequest<InspectRequest, InspectResult>;

impl Error for AdvanceError {}
impl Error for InspectError {}
impl ResponseError for AdvanceError {}

impl fmt::Display for AdvanceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "failed to advance ({})", self.cause)
    }
}

impl fmt::Display for InspectError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "failed to inspect ({})", self.cause)
    }
}

impl From<mpsc::error::SendError<SyncInspectRequest>> for InspectError {
    fn from(error: mpsc::error::SendError<SyncInspectRequest>) -> Self {
        InspectError {
            cause: error.to_string(),
        }
    }
}

pub struct Model {}

impl Model {
    // DApp logic
    pub async fn advance(
        &self,
        request: AdvanceRequest,
        _finish_channel: mpsc::Sender<AdvanceResult>,
    ) {
        eprintln!("Got advance request");
        eprintln!(
            "msg_sender: {}",
            &request.metadata.msg_sender.to_lowercase()[2..]
        );
        eprintln!("block_number: {}", request.metadata.block_number);
        eprintln!("timestamp: {}", request.metadata.time_stamp);
        eprintln!("epoch_index: {}", request.metadata.epoch_index);
        eprintln!("input_index: {}", request.metadata.input_index);
        eprintln!("input_payload: {}", &request.payload.to_lowercase()[2..]);
    }
    pub async fn inspect(&self, request: InspectRequest) -> InspectResult {
        eprintln!("Got inspect request");
        eprintln!("session_payload: {}", request.payload.to_lowercase());
        Ok(InspectReport {
            reports: vec![InspectRequest {
                payload: request.payload.to_lowercase(),
            }],
        })
    }
}
