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

use crate::world::{TestWorld, CARTESI_MACHINE_FILES_ENV};
use host_server_manager_tests::generate_default_start_session_request;
use host_server_manager_tests::grpc::proto::host_server_manager::StartSessionResponse;
use host_server_manager_tests::utils::error_name_to_code;

pub async fn do_start_valid_default_session(
    world: &mut TestWorld,
    session_id: String,
) -> Result<tonic::Response<StartSessionResponse>, tonic::Status> {
    if world.grpc_client.is_none() {
        if let Err(e) = world.connect_grpc().await {
            panic!("{:?}", e);
        }
    }
    let files_dir = TestWorld::get_var(CARTESI_MACHINE_FILES_ENV);
    let request = generate_default_start_session_request(&files_dir, &session_id);
    let response = world
        .grpc_client
        .as_mut()
        .unwrap()
        .start_session(request)
        .await;
    world.session_id = Some(session_id);
    return response;
}

#[when(regex = r"^client asks host server manager to start session with id `([a-zA-Z0-9-_]+)`$")]
async fn start_valid_default_session(world: &mut TestWorld, state: String) {
    let response = do_start_valid_default_session(world, state).await;
    world.insert_response(response);
}

#[then(regex = r"host server manager reports ([a-zA-Z]+) error$")]
async fn check_no_error_reported(world: &mut TestWorld, state: String) {
    let error = world.get_error_code();
    if state == "no" {
        assert!(error.is_none());
    } else {
        assert_eq!(error.unwrap(), error_name_to_code(&state));
    }
}
