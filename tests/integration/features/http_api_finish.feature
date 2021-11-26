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

Feature: Finish HTTP API

@serial
Scenario: perform finish request within an advance procedure
    Given host server manager is up
    And dapp backend is up
    And host server manager has session with id `test_session`
    And client asks host server manager to advance with the following parameters:
        | active_epoch_index  | 0                                        |
        | current_input_index | 0                                        |
        | msg_sender          | ffffffffffffffffffffffffffffffffffffffff |
        | block_number        | 123                                      |
        | timestamp           | 123124123                                |
        | epoch_index         | 124                                      |
        | input_index         | 125                                      |
        | input_payload       | deadbeef                                 |
    And host server manager logs 'setting state to advance'
    And client sends finish request with the following parameters:
        | status | <status> |
    When client sends notice request with the following parameters:
        | payload | 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF |
    Then host server manager logs 'ignoring notice when idle'
    And client gets http response code <code>
    Examples:
        | status | code |
        | accept | 400  |
        | reject | 400  |

@serial
Scenario: perform invalid finish request within an advance procedure
    Given host server manager is up
    And dapp backend is up
    And host server manager has session with id `test_session`
    And client asks host server manager to advance with the following parameters:
        | active_epoch_index  | 0                                        |
        | current_input_index | 0                                        |
        | msg_sender          | ffffffffffffffffffffffffffffffffffffffff |
        | block_number        | 123                                      |
        | timestamp           | 123124123                                |
        | epoch_index         | 124                                      |
        | input_index         | 125                                      |
        | input_payload       | deadbeef                                 |
    And host server manager logs 'setting state to advance'
    And client sends finish request with the following parameters:
        | status | blabla |
    And client gets http response code 422
    When client sends notice request with the following parameters:
        | payload | 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF |
    Then client gets http response code 201

@serial
Scenario: perform finish request without an advance procedure
    Given host server manager is up
    And dapp backend is up
    And host server manager has session with id `test_session`
    When client sends finish request with the following parameters:
        | status | <status> |
    Then client gets http response code <code>
    Examples:
        | status | code |
        | accept | 202  |
        | reject | 202  |
        | blabla | 422  |
