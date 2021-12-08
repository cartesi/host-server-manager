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
    error::ResponseError, http::StatusCode, middleware::Logger, web::Data, web::Json, App,
    HttpResponse, HttpServer, Responder,
};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::controller::{Controller, InsertError};
use crate::model::{FinishStatus, Notice, Report, Voucher};

pub async fn start_service(config: &Config, controller: Controller) -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(controller.clone()))
            .wrap(Logger::default())
            .service(voucher)
            .service(notice)
            .service(report)
            .service(finish)
    })
    .bind((
        config.http_target_proxy_address.as_str(),
        config.http_target_proxy_port,
    ))?
    .run()
    .await
}

#[actix_web::post("/voucher")]
async fn voucher(
    voucher: Json<Voucher>,
    controller: Data<Controller>,
) -> Result<impl Responder, InsertError> {
    controller
        .insert_voucher(voucher.0)
        .await
        .map(|id| HttpResponse::Created().json(IdResponse { id }))
}

#[actix_web::post("/notice")]
async fn notice(
    notice: Json<Notice>,
    controller: Data<Controller>,
) -> Result<impl Responder, InsertError> {
    controller
        .insert_notice(notice.0)
        .await
        .map(|id| HttpResponse::Created().json(IdResponse { id }))
}

#[actix_web::post("/report")]
async fn report(report: Json<Report>, controller: Data<Controller>) -> impl Responder {
    controller.insert_report(report.0).await;
    HttpResponse::Accepted()
}

#[actix_web::post("/finish")]
async fn finish(body: Json<FinishRequest>, controller: Data<Controller>) -> impl Responder {
    let status = match body.status.as_str() {
        "accept" => FinishStatus::Accept,
        "reject" => FinishStatus::Reject,
        _ => {
            return HttpResponse::UnprocessableEntity().body("status must be 'accept' or 'reject'");
        }
    };
    controller.finish(status).await;
    HttpResponse::Accepted().finish()
}

#[derive(Deserialize)]
struct FinishRequest {
    status: String,
}

#[derive(Serialize)]
struct IdResponse {
    id: u64,
}

impl ResponseError for InsertError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::Conflict().body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::CONFLICT
    }
}
