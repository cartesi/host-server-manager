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

mod dapp_client;
mod grpc_service;
mod model;
mod repository;
mod rollups_manager;

use futures;
use std::{error::Error, sync::Arc};
use tokio::sync::Mutex;

use repository::Repository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let repository = Arc::new(Mutex::new(Repository::new()));
    let (advance_tx, mut rollups_worker, _rollups_manager) = rollups_manager::setup(repository);

    futures::try_join!(grpc_service::run(advance_tx), rollups_worker.run(),)?;

    Ok(())
}