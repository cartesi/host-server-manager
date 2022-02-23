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
async fn test_it_finish_existing_epoch() {
    let _manager = manager::Wrapper::new().await;
    let mut grpc_client = grpc_client::connect().await;
    grpc_client
        .start_session(grpc_client::create_start_session_request("rollup session"))
        .await
        .unwrap();
    let result = grpc_client
        .finish_epoch(grpc_client::FinishEpochRequest {
            session_id: "rollup session".into(),
            active_epoch_index: 0,
            processed_input_count: 0,
            storage_directory: "".into(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(result, grpc_client::Void {});
    let response = grpc_client
        .get_epoch_status(grpc_client::GetEpochStatusRequest {
            session_id: "rollup session".into(),
            epoch_index: 0,
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(response.state, grpc_client::EpochState::Finished as i32);
}

#[tokio::test]
#[serial_test::serial]
async fn test_it_fails_to_finish_unexistent_epoch() {
    let _manager = manager::Wrapper::new().await;
    let mut grpc_client = grpc_client::connect().await;
    grpc_client
        .start_session(grpc_client::create_start_session_request("rollup session"))
        .await
        .unwrap();
    let err = grpc_client
        .finish_epoch(grpc_client::FinishEpochRequest {
            session_id: "rollup session".into(),
            active_epoch_index: 10,
            processed_input_count: 0,
            storage_directory: "".into(),
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}
