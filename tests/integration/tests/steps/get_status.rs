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
use host_server_manager_tests::grpc::proto::cartesi_machine::Void;
use host_server_manager_tests::grpc::proto::host_server_manager::GetStatusResponse;

use crate::TestWorld;

#[when("client asks host server manager status")]
async fn send_status_request(world: &mut TestWorld) {
    if let Err(e) = world.connect_grpc().await {
        panic!("{:?}", e);
    }
    let response = world
        .grpc_client
        .as_mut()
        .unwrap()
        .get_status(Void {})
        .await;
    world.insert_response(response);
}

#[then(regex = r"host server manager returns `([a-zA-Z0-9-_]*)`$")]
async fn check_status_response(world: &mut TestWorld, state: String) {
    let response = &world.get_response::<GetStatusResponse>().session_id;
    if state == "" {
        assert_eq!(response.len(), 0);
    } else {
        assert_eq!(response.len(), 1);
        assert_eq!(response[0], state);
    }
}
