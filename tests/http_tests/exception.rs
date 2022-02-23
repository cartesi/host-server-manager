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

async fn get_taint_status(
    grpc_client: &mut grpc_client::ServerManagerClient,
    session_id: &str,
) -> Option<grpc_client::TaintStatus> {
    let mut taint_status = None;
    const RETRIES: usize = 10;
    for _ in 0..RETRIES {
        let request = grpc_client::GetSessionStatusRequest {
            session_id: session_id.into(),
        };
        taint_status = grpc_client
            .get_session_status(request)
            .await
            .unwrap()
            .into_inner()
            .taint_status;
        if taint_status.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    taint_status
}

#[tokio::test]
#[serial_test::serial]
async fn test_it_notifies_exception_during_advance_state() {
    let _manager = manager::Wrapper::new().await;
    let mut grpc_client = grpc_client::connect().await;
    setup_advance_state(&mut grpc_client, "rollup session").await;
    let payload = http_client::create_payload();
    http_client::notify_exception(payload.clone())
        .await
        .unwrap();
    let taint_status = get_taint_status(&mut grpc_client, "rollup session")
        .await
        .unwrap();
    assert_eq!(
        taint_status,
        grpc_client::TaintStatus {
            error_code: tonic::Code::Internal as i32,
            error_message: format!("rollup exception ({})", payload),
        }
    );
}

#[tokio::test]
#[serial_test::serial]
async fn test_it_notifies_exception_during_inspect_state() {
    let _manager = manager::Wrapper::new().await;
    let inspect_handle = tokio::spawn(http_client::inspect(http_client::create_payload()));
    http_client::finish("accept".into()).await.unwrap();
    let payload = http_client::create_payload();
    http_client::notify_exception(payload.clone())
        .await
        .unwrap();
    let response = inspect_handle.await.unwrap();
    assert_eq!(
        response,
        Err(http_client::HttpError {
            status: 500,
            message: format!("rollup exception ({})", payload),
        })
    );
}

#[tokio::test]
#[serial_test::serial]
async fn test_it_fails_to_notify_exception_with_incorrect_data() {
    let _manager = manager::Wrapper::new().await;
    let mut grpc_client = grpc_client::connect().await;
    setup_advance_state(&mut grpc_client, "rollup session").await;
    let response = http_client::notify_exception("deadbeef".into()).await;
    assert_eq!(
        response,
        Err(http_client::HttpError {
            status: 400,
            message: "Failed to decode ethereum binary string deadbeef (expected 0x prefix)".into(),
        })
    );
}

#[tokio::test]
#[serial_test::serial]
async fn test_it_fails_to_notify_exception_during_idle_state() {
    let _manager = manager::Wrapper::new().await;
    let response = http_client::notify_exception(http_client::create_payload()).await;
    assert_eq!(
        response,
        Err(http_client::HttpError {
            status: 400,
            message: "invalid request exception in idle state".into(),
        })
    );
}
