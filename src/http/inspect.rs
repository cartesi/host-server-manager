// Copyright 2021 Cartesi Pte. Ltd.
//
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

use actix_web::{
    error::Result as HttpResult, middleware::Logger, web::Data, App, HttpResponse, HttpServer,
    Responder,
};

use crate::config::Config;
use crate::controller::Controller;
use crate::conversions;
use crate::model::InspectRequest;

use super::model::{HttpInspectResponse, HttpReport};

pub async fn start_service(config: &Config, controller: Controller) -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(controller.clone()))
            .wrap(Logger::default())
            .service(inspect)
    })
    .bind((
        config.http_inspect_address.as_str(),
        config.http_inspect_port,
    ))?
    .run()
    .await
}

#[actix_web::get("/inspect/{payload}")]
async fn inspect(payload: String, controller: Data<Controller>) -> HttpResult<impl Responder> {
    let payload = conversions::decode_ethereum_binary(&payload)?;
    let request = InspectRequest { payload };
    let reports = controller.inspect(request).await?;
    let reports = reports.into_iter().map(HttpReport::from).collect();
    let response = HttpInspectResponse { reports };
    Ok(HttpResponse::Ok().json(response))
}
