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

use cucumber::{given, then, when};
use hex::FromHex;
use serde::Serialize;

use crate::steps::start_session::do_start_valid_default_session;
use crate::world::TestWorld;
use host_server_manager_tests::grpc::proto::host_server_manager::{
    Address, AdvanceStateRequest, InputMetadata,
};
use host_server_manager_tests::utils::wait_process_output;

fn gherkin_table_to_input_metadata(request_data: &Vec<Vec<String>>) -> Option<InputMetadata> {
    if request_data[2][1] == "None"
        && request_data[3][1] == "None"
        && request_data[4][1] == "None"
        && request_data[5][1] == "None"
        && request_data[6][1] == "None"
    {
        return None;
    }

    let mut msg_sender: Option<Address> = None;
    if request_data[2][1] != "None" {
        msg_sender = Some(Address {
            data: Vec::from_hex(request_data[2][1].clone()).expect("Invalid hex test data"),
        });
    }

    Some(InputMetadata {
        msg_sender: msg_sender,
        block_number: request_data[3][1]
            .parse::<u64>()
            .expect("Invalid u64 test data"),
        time_stamp: request_data[4][1]
            .parse::<u64>()
            .expect("Invalid u64 test data"),
        epoch_index: request_data[5][1]
            .parse::<u64>()
            .expect("Invalid u64 test data"),
        input_index: request_data[6][1]
            .parse::<u64>()
            .expect("Invalid u64 test data"),
    })
}

#[given(regex = r"host server manager has session with id `([a-zA-Z0-9-_]+)`$")]
async fn check_session_exists(world: &mut TestWorld, state: String) {
    assert!(do_start_valid_default_session(world, state).await.is_ok());
}

async fn do_send_machine_manager_advance(request_data: &Vec<Vec<String>>, world: &mut TestWorld) {
    let request = AdvanceStateRequest {
        session_id: world.session_id.as_ref().unwrap().to_string(),
        active_epoch_index: request_data[0][1]
            .parse::<u64>()
            .expect("Invalid u64 test data"),
        current_input_index: request_data[1][1]
            .parse::<u64>()
            .expect("Invalid u64 test data"),
        input_metadata: gherkin_table_to_input_metadata(&request_data),
        input_payload: Vec::from_hex(request_data[7][1].clone()).expect("Invalid hex test data"),
    };
    let response = world
        .grpc_client
        .as_mut()
        .unwrap()
        .advance_state(request)
        .await;
    world.insert_response(response);
}

#[derive(Serialize)]
pub struct AdvanceHttpMetadata {
    pub msg_sender: String,
    pub epoch_index: u64,
    pub input_index: u64,
    pub block_number: u64,
    pub time_stamp: u64,
}

#[derive(Serialize)]
pub struct AdvanceHttpRequest {
    pub metadata: AdvanceHttpMetadata,
    pub payload: String,
}

async fn do_send_echo_dapp_advance(request_data: &Vec<Vec<String>>, world: &mut TestWorld) {
    let request = AdvanceHttpRequest {
        metadata: AdvanceHttpMetadata {
            msg_sender: request_data[0][1].clone(),
            block_number: request_data[1][1]
                .parse::<u64>()
                .expect("Invalid u64 test data"),
            time_stamp: request_data[2][1]
                .parse::<u64>()
                .expect("Invalid u64 test data"),
            epoch_index: request_data[3][1]
                .parse::<u64>()
                .expect("Invalid u64 test data"),
            input_index: request_data[4][1]
                .parse::<u64>()
                .expect("Invalid u64 test data"),
        },
        payload: request_data[5][1].clone(),
    };
    let request_body =
        serde_json::to_string(&request).expect("Unable to serialize AdvanceHttpRequest");
    let response = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{}/advance", world.dapp_http_port))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(request_body)
        .send()
        .await;
    assert!(response.is_ok());
    world.insert_http_response(response.unwrap());
}

#[given(
    regex = r"client asks (host server manager|echo dapp) to advance with the following parameters:"
)]
#[when(
    regex = r"client asks (host server manager|echo dapp) to advance with the following parameters:"
)]
async fn send_advance_request(world: &mut TestWorld, recipient: String, step: &gherkin_rust::Step) {
    assert!(world.session_id.is_some());
    let request_data = &step.table.as_ref().unwrap().rows;

    match &recipient[..] {
        "host server manager" => do_send_machine_manager_advance(request_data, world).await,
        "echo dapp" => do_send_echo_dapp_advance(request_data, world).await,
        _ => panic!("Unknown recipient specified in the test scenario"),
    };
}

#[then(regex = r"^dapp backend logs '(.+)'$")]
async fn check_dapp_log(world: &mut TestWorld, state: String) {
    assert!(wait_process_output(world.dapp_receiver.as_ref().unwrap(), vec!((state, 1))).is_ok());
}

#[then(regex = r"dapp backend receives the following ([a-z]+) parameters:$")]
async fn check_dapp_advance_parameters(
    world: &mut TestWorld,
    _state: String,
    step: &gherkin_rust::Step,
) {
    let request_data = &step.table.as_ref().unwrap().rows;
    for i in 0..request_data.len() {
        assert!(wait_process_output(
            world.dapp_receiver.as_ref().unwrap(),
            vec!((format!("{}: {}", request_data[i][0], request_data[i][1]), 1))
        )
        .is_ok());
    }
}
