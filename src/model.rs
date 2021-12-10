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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvanceMetadata {
    pub address: [u8; 20],
    pub epoch_number: u64,
    pub input_number: u64,
    pub block_number: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvanceRequest {
    pub metadata: AdvanceMetadata,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvanceResult {
    pub status: FinishStatus,
    pub vouchers: Vec<Identified<Voucher>>,
    pub notices: Vec<Identified<Notice>>,
    pub reports: Vec<Report>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectRequest {
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identified<T> {
    pub id: u64,
    pub value: T,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishStatus {
    Accept,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voucher {
    pub address: [u8; 20],
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub payload: Vec<u8>,
}
