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

Feature: GetStatus rpc

@serial
Scenario: asking host server manager status with no sessions running
    Given host server manager is up
    When client asks host server manager status
    Then host server manager returns ``

@serial
Scenario: asking host server manager status with one session running
    Given host server manager is up
    And host server manager has session with id `<session_id>`
    When client asks host server manager status
    Then host server manager returns `<session_id>`
    Examples:
        |  session_id  |
        | test_session |
