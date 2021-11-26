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

Feature: InspectState rpc

# Erroneous scenarios for the inspect request
@serial
Scenario: asking host server manager to inspect state incorrectly within existent session
    Given host server manager is up
    And dapp backend is up
    And host server manager has session with id `test_session`
    When client asks host server manager to inspect with the following parameters:
        | active_epoch_index  | <active_epoch>    |
        | session_payload     | <session_payload> |
    Then host server manager reports <error_code> error
    Examples:
        | active_epoch | session_payload | error_code    |
        | 0            | deadbeef        | Unimplemented |
