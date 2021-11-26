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

use cucumber::{then, when};

use crate::TestWorld;
use host_server_manager_tests::grpc::proto::host_server_manager::{
    GetEpochStatusRequest, GetEpochStatusResponse,
};

#[when("client asks host server manager epoch status for with the following data:")]
async fn send_epoch_status_request(world: &mut TestWorld, step: &gherkin_rust::Step) {
    assert!(world.session_id.is_some());
    let request_data = &step.table.as_ref().unwrap().rows;
    let response = world
        .grpc_client
        .as_mut()
        .unwrap()
        .get_epoch_status(GetEpochStatusRequest {
            session_id: request_data[1][0].clone(),
            epoch_index: request_data[1][1]
                .parse::<u64>()
                .expect("Invalid u64 test data"),
        })
        .await;
    world.insert_response(response);
}

#[then("host server manager returns the following epoch status data:")]
async fn check_epoch_status_response(world: &mut TestWorld, step: &gherkin_rust::Step) {
    let request_data = &step.table.as_ref().unwrap().rows;
    let origin_response = world.get_response::<GetEpochStatusResponse>();
    assert_eq!(origin_response.session_id, request_data[1][0]);
    assert_eq!(
        origin_response.epoch_index,
        request_data[1][1]
            .parse::<u64>()
            .expect("Invalid u64 test data")
    );
    assert_eq!(
        origin_response.state,
        request_data[1][2]
            .parse::<i32>()
            .expect("Invalid u64 test data")
    );
}
