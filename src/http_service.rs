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
use std::error::Error;

use super::config::Config;
use super::model::{Notice, Report, Voucher};
use super::proxy::{ProxyChannel, ProxyError};

/// Setup the HTTP server that receives requests from the DApp backend
pub async fn run(config: &Config, proxy: ProxyChannel) -> Result<(), Box<dyn Error + Send + Sync>> {
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(proxy.clone()))
            .wrap(Logger::default())
            .service(voucher)
            .service(notice)
            .service(report)
            .service(finish)
    })
    .bind((config.proxy_http_address, config.proxy_http_port))?
    .run()
    .await
    .map_err(|e| e.into())
}

#[actix_web::post("/voucher")]
async fn voucher(
    voucher: Json<Voucher>,
    proxy: Data<ProxyChannel>,
) -> Result<impl Responder, ProxyError> {
    let id = proxy.add_voucher(voucher.0).await?;
    Ok(HttpResponse::Created().json(IdResponse { id }))
}

#[actix_web::post("/notice")]
async fn notice(
    notice: Json<Notice>,
    proxy: Data<ProxyChannel>,
) -> Result<impl Responder, ProxyError> {
    let id = proxy.add_notice(notice.0).await?;
    Ok(HttpResponse::Created().json(IdResponse { id }))
}

#[actix_web::post("/report")]
async fn report(report: Json<Report>, proxy: Data<ProxyChannel>) -> impl Responder {
    proxy.add_report(report.0).await;
    HttpResponse::Accepted()
}

#[actix_web::post("/report")]
async fn finish(body: Json<FinishRequest>, proxy: Data<ProxyChannel>) -> impl Responder {
    match body.status.as_str() {
        "accept" => {
            proxy.accept().await;
            HttpResponse::Accepted()
        }
        "reject" => {
            proxy.reject().await;
            HttpResponse::Accepted()
        }
        _ => HttpResponse::UnprocessableEntity(),
    }
}

#[derive(Deserialize)]
struct FinishRequest {
    status: String,
}

#[derive(Serialize)]
struct IdResponse {
    id: u64,
}

impl ResponseError for ProxyError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            ProxyError::OutOfSync => HttpResponse::Conflict().body(self.to_string()),
        }
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            ProxyError::OutOfSync => StatusCode::CONFLICT,
        }
    }
}
