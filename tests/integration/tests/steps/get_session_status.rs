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
    GetSessionStatusRequest, GetSessionStatusResponse,
};

#[when(regex = r"client asks host server manager session `([a-zA-Z0-9-_]*)` status$")]
async fn send_session_status_request(world: &mut TestWorld, state: String) {
    assert!(world.session_id.is_some());
    let response = world
        .grpc_client
        .as_mut()
        .unwrap()
        .get_session_status(GetSessionStatusRequest { session_id: state })
        .await;
    world.insert_response(response);
}

#[then("host server manager returns the following session status data:")]
async fn check_session_status_response(world: &mut TestWorld, step: &gherkin_rust::Step) {
    let request_data = &step.table.as_ref().unwrap().rows;
    let origin_response = world.get_response::<GetSessionStatusResponse>();
    assert_eq!(origin_response.session_id, request_data[1][0]);
    assert_eq!(
        origin_response.active_epoch_index,
        request_data[1][1]
            .parse::<u64>()
            .expect("Invalid u64 test data")
    );

    let epoch_index_data: Vec<u64> = (request_data[1][2])[1..request_data[1][2].len() - 1]
        .split(",")
        .collect::<Vec<&str>>()
        .iter()
        .map(|e| e.parse::<u64>().expect("Invalid u64 test data"))
        .collect();
    assert_eq!(origin_response.epoch_index.len(), epoch_index_data.len());
    let eq = origin_response
        .epoch_index
        .iter()
        .zip(epoch_index_data.iter())
        .filter(|&(a, b)| a == b)
        .count();
    assert_eq!(origin_response.epoch_index.len(), eq);
}
