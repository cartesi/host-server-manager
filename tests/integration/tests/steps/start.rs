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

use cucumber::given;
use std::{
    process::{Command, Stdio},
    sync::mpsc::channel,
    thread::sleep,
    time::Duration,
};

use crate::world::{
    TestWorld, DAPP_BACKEND_ADDRESS_ENV, DAPP_HTTP_PORT_VAR, DAPP_MANAGER_ADDRESS_ENV,
    GRPC_MACHINE_MANAGER_PORT_VAR, HTTP_INSPECT_PORT_VAR, HTTP_TARGET_PROXY_PORT_VAR,
};

use host_server_manager_tests::utils::{start_listener, wait_process_output};

#[given("host server manager is up")]
async fn start_manager(world: &mut TestWorld) {
    world.manager_handler = Some(
        Command::new(&world.manager_bin)
            .env(
                GRPC_MACHINE_MANAGER_PORT_VAR,
                world.grpc_machine_manager_port.to_string(),
            )
            .env(
                HTTP_TARGET_PROXY_PORT_VAR,
                world.http_target_proxy_port.to_string(),
            )
            .env(HTTP_INSPECT_PORT_VAR, world.http_inspect_port.to_string())
            .env(DAPP_HTTP_PORT_VAR, world.dapp_http_port.to_string())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start process"),
    );
    let (data_sender, data_receiver) = channel();
    let (cmd_sender, cmd_receiver) = channel();
    world.manager_sender = Some(cmd_sender);
    world.manager_receiver = Some(data_receiver);

    start_listener(
        data_sender,
        cmd_receiver,
        world
            .manager_handler
            .as_mut()
            .unwrap()
            .stderr
            .take()
            .unwrap(),
    );

    if let Err(e) = wait_process_output(
        world.manager_receiver.as_ref().unwrap(),
        vec![
            (
                "actix_server::server] Actix runtime found. Starting in Actix runtime".to_string(),
                2,
            ),
        ],
    ) {
        panic!("{}", e);
    }
    // After output is generated manager still needs some time to start listening for grpc
    // connections. There is not other way to know that grpc is ready so we are waiting here.
    sleep(Duration::new(1, 0));
}

#[given("dapp backend is up")]
async fn start_backend(world: &mut TestWorld) {
    world.dapp_handler = Some(
        Command::new(&world.dapp_bin)
            .env(
                DAPP_BACKEND_ADDRESS_ENV,
                format!("127.0.0.1:{}", world.dapp_http_port),
            )
            .env(
                DAPP_MANAGER_ADDRESS_ENV,
                format!("http://127.0.0.1:{}", world.http_target_proxy_port),
            )
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start process"),
    );
    let (data_sender, data_receiver) = channel();
    let (cmd_sender, cmd_receiver) = channel();
    world.dapp_sender = Some(cmd_sender);
    world.dapp_receiver = Some(data_receiver);

    start_listener(
        data_sender,
        cmd_receiver,
        world.dapp_handler.as_mut().unwrap().stderr.take().unwrap(),
    );

    if let Err(e) = wait_process_output(
        world.dapp_receiver.as_ref().unwrap(),
        vec![
            (
                "actix_server::server] Actix runtime found. Starting in Actix runtime".to_string(),
                1,
            ),
        ],
    ) {
        panic!("{}", e);
    }
}

#[given(
    regex = r"echo backend is up and will generate (\d+) vouchers, (\d+) notices and (\d+) reports"
)]
async fn start_echo(world: &mut TestWorld, vouchers: u32, notices: u32, reports: u32) {
    let args = [
        "--without-dispatcher",
        &format!("--vouchers={}", vouchers),
        &format!("--notices={}", notices),
        &format!("--reports={}", reports),
        &format!("--dispatcher=127.0.0.1:{}", world.http_target_proxy_port),
        &format!("--address=127.0.0.1:{}", world.dapp_http_port),
    ];
    world.dapp_handler = Some(
        Command::new(&world.echo_bin)
            .args(args)
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start process"),
    );
    let (data_sender, data_receiver) = channel();
    let (cmd_sender, cmd_receiver) = channel();
    world.dapp_sender = Some(cmd_sender);
    world.dapp_receiver = Some(data_receiver);

    start_listener(
        data_sender,
        cmd_receiver,
        world.dapp_handler.as_mut().unwrap().stderr.take().unwrap(),
    );

    if let Err(e) = wait_process_output(
        world.dapp_receiver.as_ref().unwrap(),
        vec![
            (
                "echo_dapp] starting dapp without http dispatcher".to_string(),
                1,
            ),
            (
                "actix_server::server] Actix runtime found; starting in Actix runtime".to_string(),
                1,
            ),
        ],
    ) {
        panic!("{}", e);
    }
}
