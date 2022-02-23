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
async fn test_it_performs_inspect_request() {
    let _manager = manager::Wrapper::new().await;
    // Perform the inspect request in another thread because it is blocking
    let inspect_handle = tokio::spawn(http_client::inspect(http_client::create_payload()));
    // Perform finish call to go to the inspect state
    http_client::finish("accept".into()).await.unwrap();
    // Send a report
    let payload = http_client::create_payload();
    http_client::insert_report(payload.clone()).await.unwrap();
    // Perform final finish call
    tokio::spawn(http_client::finish("accept".into()));
    // Obtain the inspect result
    let result = inspect_handle.await.unwrap().unwrap();
    assert_eq!(result.reports, vec![http_client::Report { payload }]);
}

#[tokio::test]
#[serial_test::serial]
async fn test_it_fails_to_perform_inspect_request_with_wrong_payload() {
    let _manager = manager::Wrapper::new().await;
    // Perform the inspect request in another thread because it is blocking
    let payload = String::from("deadbeef");
    let response = http_client::inspect(payload).await;
    assert_eq!(
        response,
        Err(http_client::HttpError {
            status: 400,
            message: "Failed to decode ethereum binary string deadbeef (expected 0x prefix)".into(),
        })
    );
}
