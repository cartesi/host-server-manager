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

use crate::common::*;

#[tokio::test]
#[serial_test::serial]
async fn test_it_finishes_after_advance_request() {
    let _manager = manager::Wrapper::new().await;
    let mut grpc_client = grpc_client::connect().await;
    grpc_client
        .start_session(grpc_client::create_start_session_request("rollup session"))
        .await
        .unwrap();
    // Perform the advance request
    let advance_request = grpc_client::create_advance_state_request("rollup session", 0, 0);
    grpc_client
        .advance_state(advance_request.clone())
        .await
        .unwrap();
    // Then perform the finish request
    let response = http_client::finish("accept".into()).await.unwrap();
    // Then compare the received request with the expected one
    let expected_metadata = advance_request.input_metadata.unwrap();
    let expected_sender = expected_metadata.msg_sender.unwrap().data;
    assert_eq!(
        response,
        http_client::RollupHttpRequest::Advance {
            data: http_client::AdvanceRequest {
                metadata: http_client::AdvanceMetadata {
                    msg_sender: String::from("0x") + &hex::encode(&expected_sender),
                    epoch_index: expected_metadata.epoch_index,
                    input_index: expected_metadata.input_index,
                    block_number: expected_metadata.block_number,
                    timestamp: expected_metadata.timestamp,
                },
                payload: String::from("0x") + &hex::encode(&advance_request.input_payload),
            }
        }
    );
}

#[tokio::test]
#[serial_test::serial]
async fn test_it_finishes_after_inspect_request() {
    let _manager = manager::Wrapper::new().await;
    let mut grpc_client = grpc_client::connect().await;
    grpc_client
        .start_session(grpc_client::create_start_session_request("rollup session"))
        .await
        .unwrap();
    let payload = String::from("0xdeafbeef");
    // Perform the inspect request in another thread because it is blocking
    tokio::spawn(http_client::inspect(payload.clone()));
    // Then perform the finish request
    let response = http_client::finish("accept".into()).await.unwrap();
    // Then compare the received request with the expected one
    assert_eq!(
        response,
        http_client::RollupHttpRequest::Inspect {
            data: http_client::InspectRequest { payload }
        }
    );
}

#[tokio::test]
#[serial_test::serial]
async fn test_it_finishes_before_advance_request() {
    let _manager = manager::Wrapper::new().await;
    let mut grpc_client = grpc_client::connect().await;
    grpc_client
        .start_session(grpc_client::create_start_session_request("rollup session"))
        .await
        .unwrap();
    let finish_handler = tokio::spawn(http_client::finish("accept".into()));
    // Wait for a bit before sending advance request
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    grpc_client
        .advance_state(grpc_client::create_advance_state_request(
            "rollup session",
            0,
            0,
        ))
        .await
        .unwrap();
    // Then receive and compare finish response
    let response = finish_handler.await.unwrap();
    assert!(matches!(
        response,
        Ok(http_client::RollupHttpRequest::Advance { .. })
    ));
}

#[tokio::test]
#[serial_test::serial]
async fn test_it_finishes_current_advance_request() {
    let _manager = manager::Wrapper::new().await;
    let mut grpc_client = grpc_client::connect().await;
    setup_advance_state(&mut grpc_client, "rollup session").await;
    finish_advance_state(&mut grpc_client, "rollup session")
        .await
        .unwrap();
}

#[tokio::test]
#[serial_test::serial]
async fn test_it_fails_to_finish_while_waiting_for_rollup_request() {
    let _manager = manager::Wrapper::new().await;
    let mut grpc_client = grpc_client::connect().await;
    grpc_client
        .start_session(grpc_client::create_start_session_request("rollup session"))
        .await
        .unwrap();
    // Peform the first finish call in another thread
    tokio::spawn(http_client::finish("accept".into()));
    // Wait for a bit and perform another finish call before the previous one returned
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    let err = http_client::finish("accept".into()).await.unwrap_err();
    assert_eq!(
        err,
        http_client::HttpError {
            status: 400,
            message: String::from("invalid request finish in fetch request state"),
        }
    );
}

#[tokio::test]
#[serial_test::serial]
async fn test_it_times_out_finish_request() {
    let _manager = manager::Wrapper::new().await;
    let mut grpc_client = grpc_client::connect().await;
    grpc_client
        .start_session(grpc_client::create_start_session_request("rollup session"))
        .await
        .unwrap();
    // Perform the finish request and wait until it times out
    let err = http_client::finish("accept".into()).await.unwrap_err();
    assert_eq!(
        err,
        http_client::HttpError {
            status: 202,
            message: String::from("no rollup request available"),
        }
    );
}
