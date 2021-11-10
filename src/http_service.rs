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

use rocket::{http::Status, serde::json::Json, Responder, State};
use serde::{Deserialize, Serialize};
use std::error::Error;

use super::config::Config;
use super::model::{Notice, Report, Voucher};
use super::proxy::ProxyChannel;

/// Creates the server and start it
pub async fn run(config: &Config, proxy: ProxyChannel) -> Result<(), Box<dyn Error + Send + Sync>> {
    let figment = rocket::Config::figment()
        .merge(("address", config.proxy_http_address))
        .merge(("port", config.proxy_http_port));
    rocket::custom(figment)
        .manage(proxy)
        .mount("/", rocket::routes![voucher, notice, report, finish])
        .launch()
        .await?;
    Ok(())
}

#[rocket::post("/voucher", data = "<voucher>")]
async fn voucher(voucher: Json<Voucher>, proxy: &State<ProxyChannel>) -> IdResponder {
    IdResponder::from(proxy.add_voucher(voucher.0).await)
}

#[rocket::post("/notice", data = "<notice>")]
async fn notice(notice: Json<Notice>, proxy: &State<ProxyChannel>) -> IdResponder {
    IdResponder::from(proxy.add_notice(notice.0).await)
}

#[rocket::post("/report", data = "<report>")]
async fn report(report: Json<Report>, proxy: &State<ProxyChannel>) -> Status {
    check_accept(proxy.add_report(report.0).await)
}

#[rocket::post("/finish", data = "<body>")]
async fn finish(body: Json<FinishBody<'_>>, proxy: &State<ProxyChannel>) -> Status {
    match body.status {
        "accept" => check_accept(proxy.accept().await),
        "reject" => check_accept(proxy.reject().await),
        _ => Status::UnprocessableEntity,
    }
}

#[derive(Serialize)]
struct IdResponse {
    id: u64,
}

#[derive(Responder)]
enum IdResponder {
    #[response(status = 201)]
    Created(Json<IdResponse>),
    #[response(status = 500)]
    InternalServerError(()),
}

impl IdResponder {
    fn from(add_result: Result<u64, Box<dyn Error + Send + Sync>>) -> Self {
        match add_result {
            Ok(id) => IdResponder::Created(Json(IdResponse { id })),
            Err(_) => IdResponder::InternalServerError(()),
        }
    }
}

fn check_accept(result: Result<(), Box<dyn Error + Send + Sync>>) -> Status {
    match result {
        Ok(()) => Status::Accepted,
        Err(_) => Status::InternalServerError,
    }
}

#[derive(Deserialize)]
struct FinishBody<'a> {
    status: &'a str,
}
