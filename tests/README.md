# Host Server Manager Integration Tests

## Quick start

Perform the following steps to run the integration tests.
These steps assume that the host-server-manager is already built.

1. Build the test dapp.

```
cd test-backend
cargo build
cd -
```

2. Build the echo dapp. This step requires the [Rust nightly toolchain](https://rust-lang.github.io/rustup/concepts/channels.html).

```
cd machine-emulator-tools/linux/rollup/http/echo-dapp
cargo +nightly build
cd -
```

3. Set the following environment variables
    - `CARTESI_HOST_SERVER_MANAGER_BIN` - path to the tested manager executable
    - `CARTESI_DAPP_BACKEND_BIN` - path to the test dapp executable
    - `CARTESI_ECHO_DAPP_BACKEND_BIN` - path to the echo dapp executable

```
export CARTESI_HOST_SERVER_MANAGER_BIN=$(readlink -f ../target/debug/host-server-manager)
export CARTESI_DAPP_BACKEND_BIN=$(readlink -f test-backend/target/debug/test-backend)
export CARTESI_ECHO_DAPP_BACKEND_BIN=$(readlink -f machine-emulator-tools/linux/rollup/http/echo-dapp/target/debug/echo-dapp)
```

4. Execute tests.

```
cd integration
cargo test
```
