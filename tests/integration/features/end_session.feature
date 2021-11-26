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

Feature: EndSession rpc

@serial
Scenario: asking host server manager to end existent session
    Given host server manager is up
    And host server manager has session with id `<session_id>`
    When client asks host server manager to end session with id `<session_id>`
    And client asks host server manager session `<session_id>` status
    Then host server manager reports InvalidArgument error
    Examples:
        |  session_id  |
        | test_session |

@serial
Scenario: asking host server manager to end unexistent session
    Given host server manager is up
    And host server manager has session with id `<session_id1>`
    When client asks host server manager to end session with id `<session_id2>`
    Then host server manager reports InvalidArgument error
    Examples:
        |  session_id1  |  session_id2  |
        | test_session  | session_test  |
