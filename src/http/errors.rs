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

use actix_web::{error, error::Error};

use crate::controller::{InsertError, InspectError};
use crate::conversions::DecodeError;

use super::model::{DecodeStatusError, VoucherDecodeError};

impl From<InspectError> for Error {
    fn from(e: InspectError) -> Error {
        error::ErrorInternalServerError(e.to_string())
    }
}

impl From<InsertError> for Error {
    fn from(e: InsertError) -> Error {
        error::ErrorBadRequest(e.to_string())
    }
}

impl From<DecodeError> for Error {
    fn from(e: DecodeError) -> Error {
        error::ErrorBadRequest(e.to_string())
    }
}

impl From<VoucherDecodeError> for Error {
    fn from(e: VoucherDecodeError) -> Error {
        error::ErrorBadRequest(e.to_string())
    }
}

impl From<DecodeStatusError> for Error {
    fn from(e: DecodeStatusError) -> Error {
        error::ErrorUnprocessableEntity(e.to_string())
    }
}
