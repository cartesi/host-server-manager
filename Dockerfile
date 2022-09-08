# Copyright 2021 Cartesi Pte. Ltd.
#
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use
# this file except in compliance with the License. You may obtain a copy of the
# License at http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed
# under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
# CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

FROM rust:1.59 as builder

# Setup work directory
WORKDIR /usr/src/
RUN cargo new --bin host-server-manager
WORKDIR /usr/src/host-server-manager

# Install rustfmt (required by tonic when building grpc interfaces)
RUN rustup component add rustfmt

# Install protoc version 3.20
RUN export ARCH=$(uname -m | sed 's/aarch64/aarch_64/') && \
    curl -LO https://github.com/protocolbuffers/protobuf/releases/download/v3.20.1/protoc-3.20.1-linux-$ARCH.zip && \
    unzip protoc-3.20.1-linux-$ARCH.zip -d /usr/local

# Build dependencies
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./third-party ./third-party
RUN cargo build --release

# Build application
RUN rm ./target/release/deps/host_server_manager*
RUN rm src/*.rs
COPY ./src ./src
COPY ./build.rs ./build.rs
RUN cargo install --path .

# Install grpc-health-probe
FROM golang:buster as grpc_health_probe
RUN go install github.com/grpc-ecosystem/grpc-health-probe@2ff33ce40f97594e25068ca634d657b6aac4f72a

# Build final image
FROM debian:buster-slim
RUN apt-get update && apt-get install -y libssl1.1 && rm -rf /var/lib/apt/lists/*
COPY --from=grpc_health_probe /go/bin/grpc-health-probe /usr/local/bin/grpc-health-probe
COPY --from=builder /usr/local/cargo/bin/host-server-manager /usr/local/bin/host-server-manager
CMD ["host-server-manager"]
