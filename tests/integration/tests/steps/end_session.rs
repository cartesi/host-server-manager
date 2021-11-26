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
use host_server_manager_tests::grpc::proto::host_server_manager::EndSessionRequest;

#[when(regex = r"client asks host server manager to end session with id `([a-zA-Z0-9-_]+)`$")]
async fn end_session_request(world: &mut TestWorld, state: String) {
    assert!(world.session_id.is_some());
    let response = world
        .grpc_client
        .as_mut()
        .unwrap()
        .end_session(EndSessionRequest { session_id: state })
        .await;
    world.insert_response(response);
}
