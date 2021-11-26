# Rollups Mock Manager integration tests

## Quick start

1. Build test dapp: `cd test-backend && cargo build`
2. Set the following environment variables:
    - `CARTESI_DAPP_BACKEND_BIN` - path to the test dapp executable
    - `CARTESI_HOST_SERVER_MANAGER_BIN` - path to the tested manager executable
    - `CARTESI_ECHO_DAPP_BACKEND_BIN` - path to the echo dapp executable
    - `CARTESI_MACHINE_FILES` - path to the cartesi machine files (rom, fs, kernel)
3. Execute tests: `cd integration && cargo test`
