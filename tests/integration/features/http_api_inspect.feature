# Copyright 2021 Cartesi Pte. Ltd.
#
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use
# this file except in compliance with the License. You may obtain a copy of the
# License at http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed
# under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
# CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

Feature: Inspect HTTP API

@serial
Scenario: perform correct inspect request
    Given host server manager is up
    And dapp backend is up
    And host server manager has session with id `test_session`
    When client sends inspect request with the following parameters:
        | payload | <payload> |
    Then host server manager logs 'GET /inspect/<payload> HTTP/1.1'
    And dapp backend logs 'Got inspect request'
    And dapp backend receives the following inspect parameters:
        | session_payload | <payload> |
    And client gets http response code 200
    Examples:
        | payload                                    |
        | 0xffffffffffffffffffffffffffffffffffffffff |

@serial
Scenario: perform incorrect inspect request
    Given host server manager is up
    And dapp backend is up
    And host server manager has session with id `test_session`
    When client sends inspect request with the following parameters:
        | payload | <payload> |
    Then host server manager logs 'GET /inspect/<payload> HTTP/1.1'
    And client gets http response code 400
    Examples:
        | payload |
        | blabla  |
