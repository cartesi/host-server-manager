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
    error::Result as HttpResult, middleware::Logger, web::Data, web::Json, App, HttpResponse,
    HttpServer, Responder,
};

use crate::config::Config;
use crate::dapp_client::Controller;
use crate::model::{FinishStatus, Notice, Report, Voucher};

use super::model::{HttpFinishRequest, HttpIndexResponse, HttpNotice, HttpReport, HttpVoucher};

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
        config.http_dispatcher_address.as_str(),
        config.http_dispatcher_port,
    ))?
    .run()
    .await
}

#[actix_web::post("/voucher")]
async fn voucher(
    voucher: Json<HttpVoucher>,
    controller: Data<Controller>,
) -> HttpResult<impl Responder> {
    let voucher: Voucher = voucher.into_inner().try_into()?;
    let index = controller.insert_voucher(voucher).await?;
    let response = HttpIndexResponse { index };
    Ok(HttpResponse::Created().json(response))
}

#[actix_web::post("/notice")]
async fn notice(
    notice: Json<HttpNotice>,
    controller: Data<Controller>,
) -> HttpResult<impl Responder> {
    let notice: Notice = notice.into_inner().try_into()?;
    let index = controller.insert_notice(notice).await?;
    let response = HttpIndexResponse { index };
    Ok(HttpResponse::Created().json(response))
}

#[actix_web::post("/report")]
async fn report(
    report: Json<HttpReport>,
    controller: Data<Controller>,
) -> HttpResult<impl Responder> {
    let report: Report = report.into_inner().try_into()?;
    controller.insert_report(report).await;
    Ok(HttpResponse::Accepted())
}

#[actix_web::post("/finish")]
async fn finish(
    body: Json<HttpFinishRequest>,
    controller: Data<Controller>,
) -> HttpResult<impl Responder> {
    let status: FinishStatus = body.into_inner().try_into()?;
    controller.finish(status).await;
    Ok(HttpResponse::Accepted().finish())
}
