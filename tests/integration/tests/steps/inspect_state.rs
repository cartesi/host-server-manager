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
use hex::FromHex;

use crate::TestWorld;
use host_server_manager_tests::grpc::proto::host_server_manager::{
    InspectStateRequest, InspectStateResponse,
};

#[when("client asks host server manager to inspect with the following parameters:")]
async fn send_inspect_request(world: &mut TestWorld, step: &gherkin_rust::Step) {
    assert!(world.session_id.is_some());
    let request_data = &step.table.as_ref().unwrap().rows;

    let request = InspectStateRequest {
        session_id: world.session_id.as_ref().unwrap().to_string(),
        active_epoch_index: request_data[0][1]
            .parse::<u64>()
            .expect("Invalid u64 test data"),
        query_payload: Vec::from_hex(request_data[1][1].clone()).expect("Invalid hex test data"),
    };

    let response = world
        .grpc_client
        .as_mut()
        .unwrap()
        .inspect_state(request)
        .await;
    world.insert_response(response);
}

#[then("host server manager responds with parameters:")]
async fn check_inspect_response_parameters(world: &mut TestWorld, step: &gherkin_rust::Step) {
    let cur_session_id = world.session_id.clone();
    let origin_response = world.get_response::<InspectStateResponse>();
    let request_data = &step.table.as_ref().unwrap().rows;

    assert_eq!(
        origin_response.session_id,
        cur_session_id.unwrap().to_string()
    );
    assert_eq!(
        origin_response.active_epoch_index,
        request_data[0][1]
            .parse::<u64>()
            .expect("Invalid u64 test data")
    );
    assert_eq!(
        origin_response.current_input_index,
        request_data[1][1]
            .parse::<u64>()
            .expect("Invalid u64 test data")
    );
    assert_eq!(
        origin_response.status,
        request_data[2][1]
            .parse::<i32>()
            .expect("Invalid i32 test data")
    );
}
