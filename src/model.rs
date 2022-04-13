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

use crate::conversions;
use crate::driver::{compute_notice_hash, compute_voucher_hash};
use crate::hash::Hash;
use crate::merkle_tree::proof::Proof;
use crate::proofs::Proofable;

const ADDRESS_SIZE: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvanceStateRequest {
    pub metadata: AdvanceMetadata,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvanceMetadata {
    pub msg_sender: [u8; ADDRESS_SIZE],
    pub epoch_index: u64,
    pub input_index: u64,
    pub block_number: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvanceResult {
    pub status: FinishStatus,
    pub vouchers: Vec<Voucher>,
    pub notices: Vec<Notice>,
    pub reports: Vec<Report>,
    pub voucher_hashes_in_epoch: Option<Proof>,
    pub voucher_root: Option<Hash>,
    pub notice_hashes_in_epoch: Option<Proof>,
    pub notice_root: Option<Hash>,
}

impl AdvanceResult {
    pub fn new(
        status: FinishStatus,
        vouchers: Vec<Voucher>,
        notices: Vec<Notice>,
        reports: Vec<Report>,
    ) -> Self {
        Self {
            status,
            vouchers,
            notices,
            reports,
            voucher_hashes_in_epoch: None,
            voucher_root: None,
            notice_hashes_in_epoch: None,
            notice_root: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectStateRequest {
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectResult {
    pub reports: Vec<Report>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishStatus {
    Accept,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollupRequest {
    AdvanceState(AdvanceStateRequest),
    InspectState(InspectStateRequest),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voucher {
    pub address: [u8; ADDRESS_SIZE],
    pub payload: Vec<u8>,
    pub keccak: Hash,
    pub keccak_in_voucher_hashes: Option<Proof>,
}

impl Voucher {
    pub fn new(address: [u8; ADDRESS_SIZE], payload: Vec<u8>) -> Self {
        let keccak = compute_voucher_hash(&address, &payload);
        Self {
            address,
            payload,
            keccak,
            keccak_in_voucher_hashes: None,
        }
    }
}

impl Proofable for Voucher {
    fn get_hash(&self) -> &Hash {
        &self.keccak
    }

    fn set_proof(&mut self, proof: Proof) {
        self.keccak_in_voucher_hashes = Some(proof);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub payload: Vec<u8>,
    pub keccak: Hash,
    pub keccak_in_notice_hashes: Option<Proof>,
}

impl Notice {
    pub fn new(payload: Vec<u8>) -> Self {
        let keccak = compute_notice_hash(&payload);
        Self {
            payload,
            keccak,
            keccak_in_notice_hashes: None,
        }
    }
}

impl Proofable for Notice {
    fn get_hash(&self) -> &Hash {
        &self.keccak
    }

    fn set_proof(&mut self, proof: Proof) {
        self.keccak_in_notice_hashes = Some(proof);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollupException {
    pub payload: Vec<u8>,
}

impl std::fmt::Display for RollupException {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "rollup exception ({})",
            conversions::encode_ethereum_binary(&self.payload)
        )
    }
}
