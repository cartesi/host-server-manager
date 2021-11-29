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
middleware::Logger, HttpServer,
    error::ResponseError, http::StatusCode, web::Data, web::Json, App, HttpResponse, Responder,
};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::model::{FinishStatus, Notice, Report, Voucher};
use crate::proxy::{InsertError, ProxyChannel};

pub async fn start_service(config: &Config, proxy: ProxyChannel) -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(proxy.clone()))
            .wrap(Logger::default())
            .service(voucher)
            .service(notice)
            .service(report)
            .service(finish)
    })
    .bind((config.proxy_http_address.as_str(), config.proxy_http_port))?
    .run()
    .await
}

#[actix_web::post("/voucher")]
async fn voucher(
    voucher: Json<Voucher>,
    proxy: Data<ProxyChannel>,
) -> Result<impl Responder, InsertError> {
    proxy
        .insert_voucher(voucher.0)
        .await
        .map(|id| HttpResponse::Created().json(IdResponse { id }))
}

#[actix_web::post("/notice")]
async fn notice(
    notice: Json<Notice>,
    proxy: Data<ProxyChannel>,
) -> Result<impl Responder, InsertError> {
    proxy
        .insert_notice(notice.0)
        .await
        .map(|id| HttpResponse::Created().json(IdResponse { id }))
}

#[actix_web::post("/report")]
async fn report(report: Json<Report>, proxy: Data<ProxyChannel>) -> impl Responder {
    proxy.insert_report(report.0).await;
    HttpResponse::Accepted()
}

#[actix_web::post("/report")]
async fn finish(body: Json<FinishRequest>, proxy: Data<ProxyChannel>) -> impl Responder {
    let status = match body.status.as_str() {
        "accept" => FinishStatus::Accept,
        "reject" => FinishStatus::Reject,
        _ => {
            return HttpResponse::UnprocessableEntity().body("status must be 'accept' or 'reject'");
        }
    };
    proxy.finish(status).await;
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
