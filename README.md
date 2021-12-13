# Mock Rollup Machine Manager

This project simulates the gRPC API provided by the Rollups Machine Manager Container.
Instead of instantiating a Cartesi Machine for the DApp, the Mock makes HTTP requests to a DApp that should be running in the host machine.
This project also simulates the Inspect API provided by the Inspect Container.

## Getting Started

This project requires Rust.
To install Rust follow the instructions [here](https://www.rust-lang.org/tools/install).

## Depedencies

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

It is possible to configure the mock behaviour passing CLI arguments and using environment variables.
Execute the following command to check the available options.

```
cargo run -- -h
```

## Tests

To run the tests, execute the command:

```
cargo tests
```
