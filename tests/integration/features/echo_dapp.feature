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

Feature: HTTP API with echo dapp

@serial
Scenario: perform echo dapp request within an advance procedure
    Given host server manager is up
    And echo backend is up and will generate <vouchers> vouchers, <notices> notices and <reports> reports
    And host server manager has session with id `test_session`
    And client asks host server manager to advance with the following parameters:
        | active_epoch_index  | <active_epoch>  |
        | current_input_index | <current_input> |
        | msg_sender          | <msg_sender>    |
        | block_number        | <block_number>  |
        | timestamp           | <timestamp>     |
        | epoch_index         | <epoch_index>   |
        | input_index         | <input_index>   |
        | input_payload       | <payload>       |
    And host server manager logs 'setting state to advance'
    And client asks echo dapp to advance with the following parameters:
        | msg_sender          | <msg_sender>    |
        | block_number        | <block_number>  |
        | timestamp           | <timestamp>     |
        | epoch_index         | <epoch_index>   |
        | input_index         | <input_index>   |
        | input_payload       | <payload>       |
    Then host server manager logs 'POST /<request> HTTP/1.1" <code>'
    Examples:
        | active_epoch | current_input | msg_sender                               | block_number | timestamp | epoch_index | input_index | payload  | vouchers | notices | reports | request | code |
        | 0            | 0             | ffffffffffffffffffffffffffffffffffffffff | 123          | 123124123 | 124         | 125         | deadbeef | 1        | 0       | 0       | voucher | 201  |
        | 0            | 0             | ffffffffffffffffffffffffffffffffffffffff | 123          | 123124123 | 124         | 125         | deadbeef | 0        | 1       | 0       | notice  | 201  |
        | 0            | 0             | ffffffffffffffffffffffffffffffffffffffff | 123          | 123124123 | 124         | 125         | deadbeef | 0        | 0       | 1       | report  | 202  |
