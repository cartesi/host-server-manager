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

Feature: FinishEpoch rpc

@serial
Scenario: asking host server manager to finish existent epoch
    Given host server manager is up
    And host server manager has session with id `<session_id>`
    When client asks host server manager to finish epoch with the following data:
        | session_id   |  epoch_index  |  processed_input_count  | storage_directory   |
        | <session_id> | <epoch_index> | <processed_input_count> | <storage_directory> |
    And client asks host server manager epoch status for with the following data:
        | session_id   |  epoch_index  |
        | <session_id> | <epoch_index> |
    Then host server manager reports no error
    Examples:
        |  session_id  | epoch_index | processed_input_count | storage_directory |
        | test_session | 0           | 0                     | test              |

@serial
Scenario: asking host server manager to finish unexistent epoch
    Given host server manager is up
    And host server manager has session with id `<session_id>`
    When client asks host server manager to finish epoch with the following data:
        | session_id   |  epoch_index  |  processed_input_count  | storage_directory   |
        | <session_id> | <epoch_index> | <processed_input_count> | <storage_directory> |
    Then host server manager reports InvalidArgument error
    Examples:
        |  session_id  | epoch_index | processed_input_count | storage_directory |
        | test_session | 100         | 0                     | test              |
