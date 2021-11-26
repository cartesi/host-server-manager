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
use serde::Deserialize;

use crate::TestWorld;
use host_server_manager_tests::utils::wait_process_output;

#[derive(Deserialize)]
struct IndexResponse {
    pub index: u64,
}

#[given(
    regex = r"client sends (voucher|notice|report|finish) request with the following parameters:"
)]
#[when(
    regex = r"client sends (voucher|notice|report|finish) request with the following parameters:"
)]
async fn send_post_http_request(world: &mut TestWorld, state: String, step: &gherkin_rust::Step) {
    let request_data = &step.table.as_ref().unwrap().rows;
    let request_body = match &state[..] {
        "voucher" => format!(
            "{{\"address\":\"{}\",\"payload\":\"{}\"}}",
            request_data[0][1], request_data[1][1]
        ),
        "notice" => format!("{{\"payload\":\"{}\"}}", request_data[0][1]),
        "report" => format!("{{\"payload\":\"{}\"}}", request_data[0][1]),
        "finish" => format!("{{\"status\":\"{}\"}}", request_data[0][1]),
        _ => panic!("Unknown send request type specified in the feature file"),
    };
    let response = reqwest::Client::new()
        .post(format!(
            "http://127.0.0.1:{}/{}",
            world.http_target_proxy_port, state
        ))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(request_body)
        .send()
        .await;
    assert!(response.is_ok());
    world.insert_http_response(response.unwrap());
}

#[when(regex = r"client sends inspect request with the following parameters:")]
async fn send_get_http_request(world: &mut TestWorld, step: &gherkin_rust::Step) {
    let request_data = &step.table.as_ref().unwrap().rows;
    let response = reqwest::Client::new()
        .get(format!(
            "http://127.0.0.1:{}/inspect/{}",
            world.http_inspect_port, request_data[0][1]
        ))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .send()
        .await;
    assert!(response.is_ok());
    world.insert_http_response(response.unwrap());
}

#[given(regex = r"client gets http response code (\d+)$")]
#[then(regex = r"client gets http response code (\d+)$")]
async fn process_response_code(world: &mut TestWorld, status: u16) {
    let response = world.get_http_response();
    assert_eq!(response.status().as_u16(), status);
    world.insert_http_response(*response);
}

#[then(regex = r"client gets (voucher|notice) response with index (\d+)$")]
async fn process_id_response(world: &mut TestWorld, _state: String, index: u64) {
    let response = world.get_http_response();
    assert_eq!(response.json::<IndexResponse>().await.unwrap().index, index);
}

#[given(regex = r"host server manager logs '(.+)'$")]
#[then(regex = r"host server manager logs '(.+)'$")]
async fn check_manager_log(world: &mut TestWorld, state: String) {
    assert!(
        wait_process_output(world.manager_receiver.as_ref().unwrap(), vec!((state, 1))).is_ok()
    );
}
