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

use async_trait::async_trait;
use cucumber::{World, WorldInit};
use std::{
    any::Any,
    boxed::Box,
    convert::Infallible,
    env,
    process::Child,
    sync::mpsc::{Receiver, Sender},
};

use host_server_manager_tests::grpc::proto::host_server_manager::*;

pub const GRPC_MACHINE_MANAGER_PORT_VAR: &'static str = "GRPC_SERVER_MANAGER_PORT";
pub const HTTP_TARGET_PROXY_PORT_VAR: &'static str = "HTTP_DISPATCHER_PORT";
pub const HTTP_INSPECT_PORT_VAR: &'static str = "HTTP_INSPECT_PORT";
pub const DAPP_HTTP_PORT_VAR: &'static str = "DAPP_HTTP_PORT";
pub const MANAGER_BIN_VAR: &'static str = "CARTESI_HOST_SERVER_MANAGER_BIN";
pub const BACKEND_BIN_VAR: &'static str = "CARTESI_DAPP_BACKEND_BIN";
pub const ECHO_BIN_VAR: &'static str = "CARTESI_ECHO_DAPP_BACKEND_BIN";
pub const CARTESI_MACHINE_FILES_ENV: &'static str = "CARTESI_MACHINE_FILES";

// these variables to be passed to the test dapp backend
pub const DAPP_BACKEND_ADDRESS_ENV: &'static str = "CARTESI_DAPP_BACKEND_ADDRESS";
pub const DAPP_MANAGER_ADDRESS_ENV: &'static str = "CARTESI_MANAGER_MOCK_ADDRESS";

#[derive(Debug, Default, WorldInit)]
pub struct TestWorld {
    pub manager_bin: String,
    pub dapp_bin: String,
    pub echo_bin: String,
    pub grpc_machine_manager_port: u16,
    pub http_target_proxy_port: u16,
    pub http_inspect_port: u16,
    pub dapp_http_port: u16,
    pub manager_receiver: Option<Receiver<String>>,
    pub dapp_receiver: Option<Receiver<String>>,
    pub manager_sender: Option<Sender<()>>,
    pub dapp_sender: Option<Sender<()>>,
    pub manager_handler: Option<Child>,
    pub dapp_handler: Option<Child>,
    pub response: Option<Box<dyn Any>>,
    pub grpc_client: Option<server_manager_client::ServerManagerClient<tonic::transport::Channel>>,
    pub session_id: Option<String>,
}

impl TestWorld {
    pub fn get_var(var: &str) -> String {
        env::var(var).expect(&format!("You need to set `{}` environment variable", var))
    }

    pub async fn connect_grpc(&mut self) -> Result<(), tonic::transport::Error> {
        let server_address = format!("http://127.0.0.1:{}", self.grpc_machine_manager_port);
        self.grpc_client =
            Some(server_manager_client::ServerManagerClient::connect(server_address).await?);
        Ok(())
    }

    pub fn insert_response<T: 'static>(
        &mut self,
        response: Result<tonic::Response<T>, tonic::Status>,
    ) {
        match response {
            Ok(val) => self.response = Some(Box::new(val.into_inner())),
            Err(e) => self.response = Some(Box::new(e)),
        };
    }

    pub fn insert_http_response(&mut self, response: reqwest::Response) {
        self.response = Some(Box::new(response));
    }

    pub fn get_http_response(&mut self) -> Box<reqwest::Response> {
        let response_option = std::mem::replace(&mut self.response, None);
        response_option
            .and_then(|x| x.downcast::<reqwest::Response>().ok())
            .unwrap()
    }

    pub fn get_response<ExpectedT: 'static>(&mut self) -> &ExpectedT {
        match self
            .response
            .as_ref()
            .and_then(|x| x.downcast_ref::<ExpectedT>())
        {
            Some(val) => val,
            None => {
                let error_msg = self
                    .response
                    .as_ref()
                    .and_then(|x| x.downcast_ref::<tonic::Status>())
                    .expect("No result after executing request")
                    .to_string();
                panic!("Request returned an error: {}", error_msg);
            }
        }
    }

    pub fn get_error_code(&mut self) -> Option<tonic::Code> {
        match self
            .response
            .as_ref()
            .and_then(|x| x.downcast_ref::<tonic::Status>())
        {
            Some(val) => Some(val.code()),
            None => None,
        }
    }
}

#[async_trait(?Send)]
impl World for TestWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        let mut world = TestWorld::default();
        world.manager_bin = TestWorld::get_var(MANAGER_BIN_VAR);
        world.dapp_bin = TestWorld::get_var(BACKEND_BIN_VAR);
        world.echo_bin = TestWorld::get_var(ECHO_BIN_VAR);

        world.grpc_machine_manager_port = 50501;
        world.http_target_proxy_port = 50502;
        world.http_inspect_port = 50503;
        world.dapp_http_port = 50504;

        Ok(world)
    }
}

impl Drop for TestWorld {
    fn drop(&mut self) {
        if self.manager_handler.is_some() {
            self.manager_sender.as_mut().unwrap().send(()).ok();
            self.manager_handler.as_mut().unwrap().kill().ok();
        }
        if self.dapp_handler.is_some() {
            self.dapp_sender.as_mut().unwrap().send(()).ok();
            self.dapp_handler.as_mut().unwrap().kill().ok();
        }
    }
}
