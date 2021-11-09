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
use std::sync::Arc;
use tokio::sync::Mutex;

use super::model::{AdvanceMetadata as Metadata, Notice, Report, Voucher};
use super::proxy::{Identified, Repository};

/// Store the data in the RAM
pub struct MemRepository {
    data: Arc<Mutex<SharedData>>,
}

impl MemRepository {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(SharedData::new())),
        }
    }
}

#[async_trait]
impl Repository for MemRepository {
    async fn store_vouchers(
        &self,
        metadata: &Metadata,
        vouchers: Vec<Identified<Voucher>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut data = self.data.lock().await;
        for voucher in vouchers {
            data.vouchers.push(Entry::new(metadata.clone(), voucher));
        }
        Ok(())
    }

    async fn store_notices(
        &self,
        metadata: &Metadata,
        notices: Vec<Identified<Notice>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut data = self.data.lock().await;
        for notice in notices {
            data.notices.push(Entry::new(metadata.clone(), notice));
        }
        Ok(())
    }

    async fn store_report(
        &self,
        metadata: &Metadata,
        report: Report,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut data = self.data.lock().await;
        data.reports.push(Entry::new(metadata.clone(), report));
        Ok(())
    }
}

struct SharedData {
    vouchers: Vec<Entry<Identified<Voucher>>>,
    notices: Vec<Entry<Identified<Notice>>>,
    reports: Vec<Entry<Report>>,
}

impl SharedData {
    fn new() -> Self {
        Self {
            vouchers: vec![],
            notices: vec![],
            reports: vec![],
        }
    }
}

struct Entry<T> {
    metadata: Metadata,
    value: T,
}

impl<T> Entry<T> {
    fn new(metadata: Metadata, value: T) -> Self {
        Self { metadata, value }
    }
}
