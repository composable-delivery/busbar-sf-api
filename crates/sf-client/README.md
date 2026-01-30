# busbar-sf-client

Core HTTP client infrastructure shared by the Salesforce API crates in this repository (retry, compression, rate limiting primitives, request/response wiring).

This crate is part of the **busbar-sf-api** workspace.

- Prefer the facade crate for most usage: https://crates.io/crates/busbar-sf-api
- Docs: https://docs.rs/busbar-sf-client
- Repo: https://github.com/composable-delivery/busbar-sf-api

## When to use this crate directly

Use `busbar-sf-client` if you’re building your own Salesforce API surface but want to reuse the HTTP + retry foundation.

## WASM Support (Experimental)

This crate supports WebAssembly targets through a pluggable transport architecture:

- **Native** (default): Uses `reqwest` for async HTTP with full retry, compression, and connection pooling
- **WASM**: Uses `extism-pdk` for synchronous HTTP in Extism plugin environments

### Building for WASM

```bash
cargo build --target wasm32-unknown-unknown --features wasm --no-default-features
```

### Usage Notes

- **Native builds**: Use the full async API with `SalesforceClient` and `SfHttpClient`
- **WASM builds**: Use `SfHttpClient` directly with synchronous methods (no `.await`)
- `SalesforceClient` is currently only available in native builds
- The `Response` type automatically adapts its methods (async for native, sync for WASM)
- **WASM limitation**: Retry policies are not supported in WASM due to the inability to sleep between retries. Configure your `ClientConfig` with `retry: None` for WASM environments.

### Example (Native)

```rust,no_run
use busbar_sf_client::{SfHttpClient, ClientConfig};

#[tokio::main]
async fn main() {
    let client = SfHttpClient::default_client().unwrap();
    let response = client
        .get("https://api.example.com/resource")
        .bearer_auth("token")
        .execute()
        .await
        .unwrap();
}
```

### Example (WASM)

```rust,no_run
use busbar_sf_client::{SfHttpClient, ClientConfig};

fn main() {
    // WASM does not support retry policies - configure without retry
    let client = SfHttpClient::new(
        ClientConfig::builder().without_retry().build()
    ).unwrap();
    let response = client
        .get("https://api.example.com/resource")
        .bearer_auth("token")
        .execute()  // No .await in WASM
        .unwrap();
}
```

## Testing

### Running Native Tests

```bash
# Run all tests with default (native) features
cargo test --package busbar-sf-client

# Run specific test module
cargo test --package busbar-sf-client --lib client::tests
```

### Running WASM Tests

```bash
# Run WASM unit tests
cargo test --package busbar-sf-client --no-default-features --features wasm --lib wasm_tests

# Run WASM integration tests (requires wasm32-unknown-unknown target)
rustup target add wasm32-unknown-unknown
cargo test --package busbar-sf-client --test test_wasm_integration -- --ignored
```

For more details on WASM testing, see [tests/README.md](tests/README.md).

