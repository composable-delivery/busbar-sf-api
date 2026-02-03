# busbar-sf-guest-sdk

Guest SDK for building WASM plugins that interact with Salesforce APIs through the busbar bridge.

## Overview

This crate is compiled to `wasm32-unknown-unknown` and provides ergonomic Rust wrappers around host function imports. Your plugin code **never sees Salesforce credentials** — all authentication is handled by the host.

## Authentication

### You Don't Handle Authentication!

This is the key concept: **authentication happens entirely on the host side**.

1. **Host authenticates** with Salesforce using `busbar-sf-auth`
2. **Host creates** `SfBridge` with authenticated clients
3. **Host loads** your WASM plugin
4. **You call** functions like `query()`, `create()`, `update()`
5. **Bridge executes** API calls using host's credentials
6. **You receive** only the response data

### What You Cannot Do

Your WASM plugin code **cannot**:
- See the access token or refresh token
- Make raw HTTP requests
- Read environment variables
- Access the filesystem
- Extract credentials through any mechanism

### What You Can Do

You **can** call any exposed Salesforce API through the bridge:

```rust
use busbar_sf_guest_sdk::*;
use extism_pdk::*;

#[plugin_fn]
pub fn my_function(_input: String) -> FnResult<String> {
    // All of these work - credentials are handled transparently by the host
    let accounts = query("SELECT Id, Name FROM Account LIMIT 5")?;
    let id = create("Contact", &serde_json::json!({"LastName": "Doe"}))?;
    let record = get("Account", &id, Some(&["Name", "Industry"]))?;
    update("Account", &id, &serde_json::json!({"Name": "Updated"}))?;
    delete("Account", &id)?;
    
    Ok("Success!".to_string())
}
```

## How It Works

```text
Your WASM Plugin Code
       ↓
   query("SELECT ...")  ← You call this
       ↓
   call_host_fn("sf_query", request)  ← SDK serializes request
       ↓
   sf_query (host function)  ← Crosses WASM boundary
       ↓
   SfBridge receives request  ← Bridge on host side
       ↓
   rest_client.query(...)  ← Uses host's authenticated client
       ↓
   Salesforce API  ← Real API call with credentials
       ↓
   Response flows back  ← Only data, no credentials
       ↓
   BridgeResult<QueryResponse>  ← Deserialized by SDK
       ↓
   Your code receives result  ← You get the data!
```

## Example Plugin

```rust
use busbar_sf_guest_sdk::*;
use extism_pdk::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Input {
    sobject: String,
    limit: u32,
}

#[derive(Serialize)]
struct Output {
    count: u64,
    records: Vec<serde_json::Value>,
}

#[plugin_fn]
pub fn query_sobjects(input: String) -> FnResult<Json<Output>> {
    let input: Input = serde_json::from_str(&input)?;
    
    // Build safe SOQL query
    let escaped_sobject = soql::escape_string(&input.sobject);
    let soql = format!("SELECT Id, Name FROM {} LIMIT {}", escaped_sobject, input.limit);
    
    // Execute query - credentials handled by host
    let result = query(&soql)?;
    
    Ok(Json(Output {
        count: result.total_size,
        records: result.records,
    }))
}

#[plugin_fn]
pub fn create_records(input: String) -> FnResult<Json<Vec<CreateResponse>>> {
    let records: Vec<serde_json::Value> = serde_json::from_str(&input)?;
    
    // Batch create - credentials handled by host
    let result = create_multiple("Contact", records)?;
    
    Ok(Json(result))
}
```

## Building Your Plugin

```bash
# Install WASM target (one-time)
rustup target add wasm32-unknown-unknown

# Build your plugin
cargo build --target wasm32-unknown-unknown --release

# Your .wasm file will be at:
# target/wasm32-unknown-unknown/release/your_plugin.wasm
```

## Available APIs

### REST API
- **CRUD**: `query()`, `create()`, `get()`, `update()`, `delete()`, `upsert()`
- **Collections**: `create_multiple()`, `update_multiple()`, `get_multiple()`, `delete_multiple()`
- **Composite**: `composite()`, `composite_batch()`, `composite_tree()`, `composite_graph()`
- **Describe**: `describe_global()`, `describe_sobject()`
- **Search**: `search()`, `parameterized_search()`, `search_suggestions()`
- **Process**: `list_process_rules()`, `trigger_process_rules()`, `submit_approval()`
- **List Views**: `list_views()`, `execute_list_view()`
- **Quick Actions**: `list_quick_actions()`, `invoke_quick_action()`
- And many more...

### Bulk API
- `bulk_create_ingest_job()`, `bulk_upload_job_data()`, `bulk_close_ingest_job()`
- `bulk_get_ingest_job()`, `bulk_get_job_results()`, `bulk_get_query_results()`

### Tooling API
- `tooling_query()`, `tooling_execute_anonymous()`, `tooling_get()`, `tooling_create()`

### Metadata API
- `metadata_deploy()`, `metadata_retrieve()`, `metadata_list()`, `metadata_describe()`

## Security Utilities

The SDK includes the same security utilities as the REST client:

```rust
use busbar_sf_guest_sdk::soql;

// Prevent SOQL injection
let safe_name = soql::escape_string(user_input);
let soql = format!("SELECT Id FROM Account WHERE Name = '{}'", safe_name);

// Validate SObject names
if !soql::is_safe_sobject_name(sobject) {
    return Err(Error::msg("Invalid SObject name"));
}
```

## Testing

This crate cannot have traditional unit tests because:
1. It requires the Extism WASM runtime
2. Host functions are only available when running in the bridge
3. Testing happens through integration tests in `sf-bridge`

The SDK is thoroughly tested via:
- Integration tests in `sf-bridge` that load real WASM plugins
- Example `wasm-guest-plugin` that exercises all APIs
- Type safety enforced by compiler (shared types with `sf-wasm-types`)

## Cargo.toml Setup

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
busbar-sf-guest-sdk = "0.0.3"
extism-pdk = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

## Next Steps

1. See `examples/wasm-guest-plugin/` for a complete example
2. Check the `sf-bridge` crate README for how the host side works
3. Read the `sf-bridge` lib.rs documentation for architecture details
