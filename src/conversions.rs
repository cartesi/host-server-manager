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

use snafu::Snafu;
use std::{fmt::Write, num::ParseIntError};

#[derive(Debug, Snafu)]
pub enum DecodeError {
    #[snafu(display("Failed to decode ethereum binary string {} (expected 0x prefix)", s))]
    InvalidPrefix { s: String },
    #[snafu(display(
        "Failed to decode ethereum binary string {} (expected even number of chars)",
        s
    ))]
    OddChars { s: String },
    #[snafu(display("Failed to decode ethereum binary string {} ({})", s, e))]
    ParseInt { s: String, e: ParseIntError },
}

/// Convert binary array to Ethereum binary format
pub fn encode_ethereum_binary(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(2 + bytes.len() * 2);
    write!(&mut s, "0x").expect("impossible to happen");
    for byte in bytes {
        write!(&mut s, "{:02X}", byte).expect("impossible to happen");
    }
    s
}

/// Convert string in Ethereum binary format to binary array
pub fn decode_ethereum_binary(s: &str) -> Result<Vec<u8>, DecodeError> {
    snafu::ensure!(s.starts_with("0x"), InvalidPrefix { s });
    snafu::ensure!(s.len() % 2 == 0, OddChars { s });
    (2..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<Vec<u8>, ParseIntError>>()
        .map_err(|e| {
            log::error!("error when parsing Ethereum string ({})", s);
            DecodeError::ParseInt {
                s: String::from(s),
                e,
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() {
        assert_eq!(
            encode_ethereum_binary(&[0x01, 0x20, 0xFF, 0x00]).as_str(),
            "0x0120FF00"
        );
    }

    #[test]
    fn test_encode_with_empty() {
        assert_eq!(encode_ethereum_binary(&[]).as_str(), "0x");
    }

    #[test]
    fn test_decode_with_uppercase() {
        assert_eq!(
            decode_ethereum_binary("0x0120FF00").unwrap(),
            vec![0x01, 0x20, 0xFF, 0x00]
        );
    }

    #[test]
    fn test_decode_with_lowercase() {
        assert_eq!(decode_ethereum_binary("0xff").unwrap(), vec![0xFF]);
    }

    #[test]
    fn test_decode_with_invalid_prefix() {
        let err = decode_ethereum_binary("0X0120FF00").unwrap_err();
        assert_eq!(
            err.to_string().as_str(),
            "Failed to decode ethereum binary string 0X0120FF00 (expected 0x prefix)",
        );
    }

    #[test]
    fn test_decode_with_invalid_number() {
        let err = decode_ethereum_binary("0xZZ").unwrap_err();
        assert_eq!(
            err.to_string().as_str(),
            "Failed to decode ethereum binary string 0xZZ (invalid digit found in string)"
        );
    }

    #[test]
    fn test_decode_with_odd_number_of_chars() {
        let err = decode_ethereum_binary("0xA").unwrap_err();
        assert_eq!(
            err.to_string().as_str(),
            "Failed to decode ethereum binary string 0xA (expected even number of chars)"
        );
    }
}
