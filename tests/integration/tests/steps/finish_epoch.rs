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

use cucumber::when;

use crate::TestWorld;
use host_server_manager_tests::grpc::proto::host_server_manager::FinishEpochRequest;

#[when("client asks host server manager to finish epoch with the following data:")]
async fn send_epoch_status_request(world: &mut TestWorld, step: &gherkin_rust::Step) {
    assert!(world.session_id.is_some());
    let request_data = &step.table.as_ref().unwrap().rows;
    let response = world
        .grpc_client
        .as_mut()
        .unwrap()
        .finish_epoch(FinishEpochRequest {
            session_id: request_data[0][1].clone(),
            active_epoch_index: request_data[1][1]
                .parse::<u64>()
                .expect("Invalid u64 test data"),
            processed_input_count: request_data[1][2]
                .parse::<u64>()
                .expect("Invalid u64 test data"),
            storage_directory: request_data[1][3].clone(),
        })
        .await;
    world.insert_response(response);
}
