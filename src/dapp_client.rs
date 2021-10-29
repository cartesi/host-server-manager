// Copyright 2021 Cartesi Pte. Ltd.
//
// This file is part of the machine-emulator. The machine-emulator is free
// software: you can redistribute it and/or modify it under the terms of the GNU
// Lesser General Public License as published by the Free Software Foundation,
// either version 3 of the License, or (at your option) any later version.
//
// The machine-emulator is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE. See the GNU Lesser General Public License
// for more details.
//
// You should have received a copy of the GNU Lesser General Public License
// along with the machine-emulator. If not, see http://www.gnu.org/licenses/.

use std::error::Error;

use super::model::{AdvanceRequest, InspectRequest, Report};

/// Send a advance state request to the DApp.
pub async fn advance(_request: AdvanceRequest) -> Result<(), Box<dyn Error>> {
    unimplemented!()
}

/// Send a inspect request to the DApp and return the reports.
pub async fn inspect(_request: InspectRequest) -> Result<Vec<Report>, Box<dyn Error>> {
    unimplemented!()
}
