# Mock Rollup Machine Manager

This project simulates the gRPC API provided by the Rollups Machine Manager Container.
Instead of instantiating a Cartesi Machine for the DApp, the Mock makes HTTP requests to a DApp that should be running in the host machine.
This project also simulates the Inspect API provided by the Inspect Container.

## Getting Started

This project requires Rust.
To install Rust follow the instructions [here](https://www.rust-lang.org/tools/install).

## Depencies

Before building and running the project, you should download the submoules with:

```
git submodule update --init --recursive
```

## Running

To run the mock, execute the command:

```
cargo run
```

## Configuration

It is possible to configure the server behaviour with environment variables.

- `CARTESI_GRPC_MACHINE_MANAGER_ADDRESS`

gRPC address of the Mock Rollups Machine Manager endpoint (default = `127.0.0.1`).

- `CARTESI_GRPC_MACHINE_MANAGER_PORT`

gRPC port of the Mock Rollups Machine Manager endpoint (default = `5000`).

- `CARTESI_HTTP_TARGET_PROXY_ADDRESS`

HTTP address of the Target Proxy endpoint (default = `127.0.0.1`).
This is the endpoint that receives HTTP requests from the DApp backend.

- `CARTESI_HTTP_TARGET_PROXY_PORT`

HTTP port of the Target Proxy endpoint (default = `5001`).

- `CARTESI_HTTP_INSPECT_ADDRESS`

HTTP address of the Inspect endpoint (default = `127.0.0.1`).
This is the endpoint that receives the HTTP `/inspect` request from the DApp frontend.

- `CARTESI_HTTP_INSPECT_PORT`

HTTP port of the Inspect endpoint (default = `5002`).

- `CARTESI_DAPP_HTTP_ADDRESS`

HTTP address of the DApp backend (default = `127.0.0.1`).

- `CARTESI_DAPP_HTTP_PORT`

HTTP port of the DApp backend (default = `5003`).

## Tests

To run the tests, execute the command:

```
cargo tests
```
