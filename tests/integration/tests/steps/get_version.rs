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

use crate::world::TestWorld;
use host_server_manager_tests::grpc::proto::cartesi_machine::Void;
use host_server_manager_tests::grpc::proto::versioning::GetVersionResponse;

#[when("client asks host server manager for a version")]
async fn get_version(world: &mut TestWorld) {
    assert!(world.connect_grpc().await.is_ok());
    let response = world
        .grpc_client
        .as_mut()
        .unwrap()
        .get_version(Void {})
        .await;
    world.insert_response(response);
}

#[then(regex = r"^host server manager returns (\d+\.\d+\.\d+\.[a-zA-Z0-9-_]*\.[a-zA-Z0-9-_]*)$")]
async fn check_version(world: &mut TestWorld, state: String) {
    let response = world.get_response::<GetVersionResponse>();
    assert!(response.version.is_some());
    let semantic_version = response.version.as_ref().unwrap();
    let got_version = format!(
        "{}.{}.{}.{}.{}",
        semantic_version.major,
        semantic_version.minor,
        semantic_version.patch,
        semantic_version.pre_release,
        semantic_version.build
    );
    assert_eq!(got_version, state);
}
