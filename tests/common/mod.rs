// Copyright 2022 Cartesi Pte. Ltd.
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

// TODO remove?
#![allow(dead_code)]

pub mod config;
pub mod grpc_client;
pub mod http_client;
pub mod manager;

pub fn create_address() -> Vec<u8> {
    rand::random::<[u8; 20]>().into()
}

pub fn create_payload() -> Vec<u8> {
    rand::random::<[u8; 16]>().into()
}

pub async fn setup_advance_state(
    grpc_client: &mut grpc_client::ServerManagerClient,
    session_id: &str,
) {
    grpc_client
        .start_session(grpc_client::create_start_session_request(session_id))
        .await
        .unwrap();
    grpc_client
        .advance_state(grpc_client::create_advance_state_request(session_id, 0, 0))
        .await
        .unwrap();
    http_client::finish("accept".into()).await.unwrap();
}

pub async fn finish_advance_state(
    grpc_client: &mut grpc_client::ServerManagerClient,
    session_id: &str,
) -> Option<grpc_client::ProcessedInput> {
    tokio::spawn(http_client::finish("accept".into()));
    const RETRIES: i32 = 10;
    let mut processed = None;
    for _ in 0..RETRIES {
        processed = grpc_client
            .get_epoch_status(grpc_client::GetEpochStatusRequest {
                session_id: session_id.into(),
                epoch_index: 0,
            })
            .await
            .unwrap()
            .into_inner()
            .processed_inputs
            .pop();
        if processed.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    processed
}
