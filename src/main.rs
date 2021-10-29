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
mod proxy;
mod repository;

use std::{error::Error, sync::Arc};
use tokio::{self, sync::Mutex};

use repository::Repository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let repository = Arc::new(Mutex::new(Repository::new()));
    let (advancer, _inspector, _dapp_interface, mut worker) = proxy::setup(repository);

    futures::try_join!(grpc_service::run(advancer), worker.run())?;

    Ok(())
}
