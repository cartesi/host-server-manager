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
    http::StatusCode, middleware::Logger, web::Data, App, HttpResponse, HttpServer, Responder,
    ResponseError,
};

use crate::config::Config;
use crate::controller::{Controller, InspectError};
use crate::model::{InspectRequest, InspectResponse};

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
async fn inspect(payload: String, controller: Data<Controller>) -> impl Responder {
    controller
        .inspect(InspectRequest { payload })
        .await
        .map(|reports| HttpResponse::Ok().json(InspectResponse { reports }))
}

impl ResponseError for InspectError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::InternalServerError().body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
