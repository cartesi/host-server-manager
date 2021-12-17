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

use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub struct SyncRequest<T, U>
where
    T: Send + Sync,
    U: Send + Sync,
{
    request: T,
    response_tx: oneshot::Sender<U>,
}

impl<T, U> SyncRequest<T, U>
where
    T: std::fmt::Debug + Send + Sync,
    U: std::fmt::Debug + Send + Sync,
{
    pub async fn send(tx: &mpsc::Sender<Self>, request: T) -> oneshot::Receiver<U> {
        let (response_tx, response_rx) = oneshot::channel();
        if let Err(e) = tx
            .send(SyncRequest {
                request,
                response_tx,
            })
            .await
        {
            log::error!("failed to send request ({})", e)
        }
        response_rx
    }

    pub fn into_inner(self) -> (T, oneshot::Sender<U>) {
        (self.request, self.response_tx)
    }

    pub fn process<F>(self, f: F)
    where
        F: FnOnce(T) -> U,
    {
        let response = f(self.request);
        Self::respond(self.response_tx, response);
    }

    pub async fn process_async<F, Fut>(self, f: F)
    where
        F: FnOnce(T) -> Fut,
        Fut: std::future::Future<Output = U>,
    {
        let response = f(self.request).await;
        Self::respond(self.response_tx, response);
    }

    pub async fn try_process_async<F, Fut>(self, f: F)
    where
        F: FnOnce(T) -> Fut,
        Fut: std::future::Future<Output = Option<U>>,
    {
        if let Some(response) = f(self.request).await {
            Self::respond(self.response_tx, response);
        }
    }

    fn respond(tx: oneshot::Sender<U>, response: U) {
        if let Err(_) = tx.send(response) {
            log::warn!("failed to send response (channel dropped)");
        }
    }
}
