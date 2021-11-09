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
use std::error::Error;

use super::model::{AdvanceRequest, InspectRequest, Report};
use super::proxy::DApp;

pub struct DAppClient {}

impl DAppClient {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl DApp for DAppClient {
    async fn advance(&self, _request: AdvanceRequest) -> Result<(), Box<dyn Error + Send + Sync>> {
        // unimplemented!()
        Ok(())
    }

    async fn inspect(
        &self,
        _request: InspectRequest,
    ) -> Result<Vec<Report>, Box<dyn Error + Send + Sync>> {
        // unimplemented!()
        Ok(vec![Report {
            payload: String::from("<placeholder report>"),
        }])
    }
}
