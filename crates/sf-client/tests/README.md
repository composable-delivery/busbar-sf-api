# WASM Integration Tests

This directory contains integration tests for the WASM backend of sf-client.

## Overview

The integration tests compile the sf-client library to WebAssembly and test it using the Extism plugin system. This ensures that the WASM backend works correctly in a real plugin environment.

## Structure

- `wasm-plugin/`: A small Extism plugin that uses sf-client with WASM features
  - Tests basic HTTP operations (GET, POST)
  - Tests client configuration
  - Tests that retry policies are properly rejected in WASM
- `test_wasm_integration.rs`: Rust test harness that compiles the plugin and runs tests

## Prerequisites

To run these tests, you need:

1. **Rust with wasm32-unknown-unknown target**:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

2. **Extism CLI** (optional, for manual testing):
   ```bash
   # macOS
   brew install extism/tap/extism
   
   # Linux
   curl -L https://github.com/extism/cli/releases/latest/download/extism-x86_64-linux.tar.gz | tar -xz
   sudo mv extism /usr/local/bin/
   
   # Or use cargo
   cargo install extism-cli
   ```

## Running the Tests

### Automated Tests

The integration tests will automatically compile the WASM plugin and test it:

```bash
# Run WASM integration tests
cargo test --package busbar-sf-client --test test_wasm_integration --no-default-features --features wasm
```

### Manual Testing with Extism CLI

You can also manually build and test the plugin:

```bash
# Build the WASM plugin
cd crates/sf-client/tests/wasm-plugin
cargo build --target wasm32-unknown-unknown --release

# Test with Extism CLI
extism call target/wasm32-unknown-unknown/release/sf_client_wasm_test_plugin.wasm test_client_config --input '{}'
extism call target/wasm32-unknown-unknown/release/sf_client_wasm_test_plugin.wasm test_retry_rejected --input '{}'
```

## What's Tested

1. **Client Creation**: Tests that WASM clients can be created with appropriate configurations
2. **Configuration Validation**: Ensures retry policies are rejected in WASM (since sleep is not available)
3. **Request Building**: Tests all HTTP methods (GET, POST, PUT, PATCH, DELETE, HEAD)
4. **JSON Serialization**: Tests sending and receiving JSON data
5. **Headers**: Tests authentication headers and custom headers
6. **Query Parameters**: Tests URL query parameter handling

## Limitations

- The WASM tests cannot make real HTTP requests without a proper Extism runtime environment
- These tests focus on API surface testing and client configuration validation
- For end-to-end HTTP testing, the native backend tests are more comprehensive

## Adding New Tests

To add new test functions to the WASM plugin:

1. Add a new `#[plugin_fn]` in `wasm-plugin/src/lib.rs`
2. Implement the test logic using sf-client WASM APIs
3. Update the test harness in `test_wasm_integration.rs` to call your new function
