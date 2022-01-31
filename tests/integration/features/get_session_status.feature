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

Feature: GetSessionStatus rpc

@serial
Scenario: asking host server manager session status with no advance requests
    Given host server manager is up
    And host server manager has session with id `<session_id>`
    When client asks host server manager session `<session_id>` status
    Then host server manager returns the following session status data:
        | session_id   | active_epoch | epoch |
        | <session_id> | 0            | [0]   |
    Examples:
        |  session_id  |
        | test_session |

@serial
Scenario: asking host server manager session status after advance request
    Given host server manager is up
    And dapp backend is up
    And host server manager has session with id `<session_id>`
    When client asks host server manager to advance with the following parameters:
        | active_epoch_index  | <active_epoch>  |
        | current_input_index | <current_input> |
        | msg_sender          | <msg_sender>    |
        | block_number        | <block_number>  |
        | timestamp           | <timestamp>     |
        | epoch_index         | <epoch>         |
        | input_index         | <input>         |
        | input_payload       | <input_payload> |
    And client asks host server manager session `<session_id>` status
    Then host server manager returns the following session status data:
        | session_id   | active_epoch | epoch |
        | <session_id> | 0            | [0]   |
    Examples:
        |  session_id  | active_epoch | current_input | msg_sender                               | block_number | timestamp | epoch | input | input_payload |
        | test_session | 0            | 0             | ffffffffffffffffffffffffffffffffffffffff | 123          | 214324234 | 0     | 0     | deadbeef      |

    Scenario: asking host server manager unexistent session status
    Given host server manager is up
    And host server manager has session with id `test_session`
    When client asks host server manager session `session_test` status
    Then host server manager reports InvalidArgument error
