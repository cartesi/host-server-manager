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

Feature: StartSession rpc

@serial
Scenario: asking host server manager to create a new valid session
    Given host server manager is up
    When client asks host server manager to start session with id `test_session`
    Then host server manager reports no error

@serial
Scenario: asking host server manager to create duplicated sessions
    Given host server manager is up
    When client asks host server manager to start session with id `test_session`
    And client asks host server manager to start session with id `test_session`
    Then host server manager reports AlreadyExists error

@serial
Scenario: asking host server manager to create multiple sessions fails because it supports a single session
    Given host server manager is up
    When client asks host server manager to start session with id `test_session1`
    And client asks host server manager to start session with id `test_session2`
    Then host server manager reports AlreadyExists error
