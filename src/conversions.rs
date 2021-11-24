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

use std::{fmt::Write, num::ParseIntError};

#[derive(Debug)]
pub struct ParseError();

impl std::error::Error for ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "error when parsing string")
    }
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
pub fn decode_ethereum_binary(s: &str) -> Result<Vec<u8>, ParseError> {
    if !s.starts_with("0x") {
        Err(ParseError())
    } else {
        (2..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect::<Result<Vec<u8>, ParseIntError>>()
            .map_err(|_| {
                log::error!("error when parsing Ethereum string ({})", s);
                ParseError()
            })
    }
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
        assert!(matches!(
            decode_ethereum_binary("0X0120FF00"),
            Err(ParseError())
        ));
    }

    #[test]
    fn test_decode_with_invalid_number() {
        assert!(matches!(decode_ethereum_binary("0xZZ"), Err(ParseError())));
    }
}
