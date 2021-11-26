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

Feature: AdvanceState rpc

# Default scenario for the advance request
@serial
Scenario: asking host server manager to advance state correctly within existent session
    Given host server manager is up
    And dapp backend is up
    And host server manager has session with id `test_session`
    When client asks host server manager to advance with the following parameters:
        | active_epoch_index  | <active_epoch>  |
        | current_input_index | <current_input> |
        | msg_sender          | <msg_sender>    |
        | block_number        | <block_number>  |
        | timestamp           | <timestamp>     |
        | epoch_index         | <epoch>         |
        | input_index         | <input>         |
        | input_payload       | <input_payload> |
    Then host server manager reports no error
    And host server manager logs 'setting state to advance'
    And dapp backend logs 'Got advance request'
    And dapp backend receives the following advance parameters:
        | msg_sender          | <msg_sender>    |
        | block_number        | <block_number>  |
        | timestamp           | <timestamp>     |
        | epoch_index         | <epoch>         |
        | input_index         | <input>         |
        | input_payload       | <input_payload> |
    Examples:
        | active_epoch | current_input | msg_sender                               | block_number | timestamp | epoch | input | input_payload |
        | 0            | 0             | ffffffffffffffffffffffffffffffffffffffff | 123          | 214324234 | 124   | 125   | deadbeef      |

# Erroneous scenarios for the advance request
@serial
Scenario: asking host server manager to advance state incorrectly within existent session
    Given host server manager is up
    And dapp backend is up
    And host server manager has session with id `test_session`
    When client asks host server manager to advance with the following parameters:
        | active_epoch_index  | <active_epoch>  |
        | current_input_index | <current_input> |
        | msg_sender          | <msg_sender>    |
        | block_number        | <block_number>  |
        | timestamp           | <timestamp>     |
        | epoch_index         | <epoch>         |
        | input_index         | <input>         |
        | input_payload       | <input_payload> |
    Then host server manager reports <error_code> error
    Examples:
        | active_epoch | current_input | msg_sender | block_number | timestamp | epoch | input | input_payload | error_code      |
        | 0            | 0             | None       | None         | None      | None  | None  | deadbeef      | InvalidArgument |
        | 0            | 0             | None       | 123          | 214324234 | 124   | 125   | deadbeef      | InvalidArgument |
        | 100          | 0             | ffffffffffffffffffffffffffffffffffffffff | 123          | 0         | 124   | 125   | deadbeef      | InvalidArgument |
        | 0            | 60            | ffffffffffffffffffffffffffffffffffffffff | 123          | 0         | 124   | 125   | deadbeef      | InvalidArgument |
