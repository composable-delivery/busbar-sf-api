# busbar-sf-bridge

Extism host bridge for sandboxed WASM access to Salesforce APIs.

## Overview

This crate provides `SfBridge`, which loads WASM guest plugins via Extism and exposes Salesforce API operations as host functions. **Credentials are managed entirely on the host side** — WASM guests never see tokens.

## Authentication Model

### How Authentication Works

1. **Host-Side Authentication**: The host application (your Rust code) authenticates with Salesforce using `busbar-sf-auth` and obtains credentials (instance URL + access token).

2. **Client Creation**: You create Salesforce API clients (REST, Bulk, Tooling, Metadata) with those credentials:
   ```rust
   let rest_client = SalesforceRestClient::new(instance_url, access_token)?;
   ```

3. **Bridge Initialization**: You pass the authenticated client to `SfBridge::new()`:
   ```rust
   let wasm_bytes = std::fs::read("plugin.wasm")?;
   let bridge = SfBridge::new(wasm_bytes, rest_client)?;
   ```

4. **Credential Isolation**: The bridge stores the clients internally. When the WASM guest calls host functions (like `sf_query`), the bridge:
   - Receives the request from the guest (e.g., SOQL query)
   - Uses the **host's authenticated client** to make the Salesforce API call
   - Returns only the API response data to the guest
   - **Never exposes the access token or credentials to the WASM guest**

### Security Guarantees

- **WASM guests cannot**:
  - See the access token or refresh token
  - Make raw HTTP requests (no network access in sandbox)
  - Read environment variables
  - Access the filesystem
  - Extract credentials through any mechanism

- **All Salesforce API calls** go through registered host functions
- **All authentication** happens on the host side
- **Guest code is untrusted** and fully sandboxed

## Architecture

```text
┌─────────────────────────────────────────────────┐
│  Host Application (Your Rust Code)              │
│                                                 │
│  1. Authenticate with Salesforce               │
│     ├─ OAuth 2.0 Web Flow                       │
│     ├─ JWT Bearer Flow                          │
│     ├─ Refresh Token                            │
│     └─ SFDX CLI                                 │
│                                                 │
│  2. Create Salesforce Clients                   │
│     ├─ SalesforceRestClient (with creds)       │
│     ├─ BulkApiClient (with creds)              │
│     ├─ ToolingClient (with creds)              │
│     └─ MetadataClient (with creds)             │
│                                                 │
│  3. Create Bridge                               │
│     ├─ Load WASM plugin bytes                   │
│     └─ SfBridge::new(wasm, rest_client)        │
│                                                 │
│  4. Call Guest Functions                        │
│     └─ bridge.call("my_function", input)       │
└──────────────┬──────────────────────────────────┘
               │
               │ Extism WASM Runtime
               ▼
┌─────────────────────────────────────────────────┐
│  WASM Guest Plugin (compiled with sf-guest-sdk) │
│                                                 │
│  - Calls: query(), create(), update(), etc.     │
│  - Never sees: access_token, instance_url       │
│  - Sandboxed: no network, no filesystem         │
└──────────────┬──────────────────────────────────┘
               │
               │ Host Function Calls (JSON over shared memory)
               ▼
┌─────────────────────────────────────────────────┐
│  SfBridge (this crate)                          │
│                                                 │
│  - Owns authenticated Salesforce clients        │
│  - Registers 98 host functions                  │
│  - Validates guest inputs                       │
│  - Executes API calls with host credentials     │
│  - Returns sanitized responses                  │
└─────────────────────────────────────────────────┘
               │
               │ HTTPS with credentials
               ▼
         Salesforce API
```

## Example

```rust
use busbar_sf_auth::{AuthFlow, oauth::WebServerFlow};
use busbar_sf_bridge::SfBridge;
use busbar_sf_rest::SalesforceRestClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Authenticate with Salesforce (host-side only)
    let auth_flow = WebServerFlow::new(
        "client_id",
        "client_secret",
        "https://login.salesforce.com",
        "http://localhost:3000/callback",
    )?;
    let credentials = auth_flow.authenticate("authorization_code").await?;
    
    // 2. Create REST client with credentials (host-side only)
    let rest_client = SalesforceRestClient::new(
        credentials.instance_url(),
        credentials.access_token(),
    )?;
    
    // 3. Load WASM plugin and create bridge
    let wasm_bytes = std::fs::read("my_plugin.wasm")?;
    let bridge = SfBridge::new(wasm_bytes, rest_client)?;
    
    // 4. Call guest function - guest will use host's credentials transparently
    let input = serde_json::json!({"limit": 10});
    let result = bridge.call("query_accounts", serde_json::to_vec(&input)?).await?;
    
    println!("Result: {}", String::from_utf8_lossy(&result));
    
    Ok(())
}
```

## Guest Plugin Example

```rust
// plugin.rs - compiled to WASM
use busbar_sf_guest_sdk::*;
use extism_pdk::*;

#[plugin_fn]
pub fn query_accounts(input: String) -> FnResult<Json<Vec<serde_json::Value>>> {
    // This call goes through the host function bridge
    // The host's credentials are used automatically
    // You never see the access token!
    let result = query("SELECT Id, Name FROM Account LIMIT 10")?;
    Ok(Json(result.records))
}
```

## Features

- `default = ["full"]` - All API surfaces
- `full = ["rest", "bulk", "tooling", "metadata"]` - All APIs
- `rest` - REST API endpoints only
- `bulk` - Bulk API endpoints (requires `rest`)
- `tooling` - Tooling API endpoints (requires `rest`)
- `metadata` - Metadata API endpoints (requires `rest`)

## Testing

Integration tests run against a real Salesforce org:

```bash
# Build the test WASM plugin first
bash tests/wasm-test-plugin/build.sh

# Run integration tests with credentials
SF_AUTH_URL=... cargo test --test integration bridge::
```

The tests demonstrate:
1. Host authenticates with Salesforce
2. Host creates bridge with authenticated clients
3. Bridge loads WASM test plugin
4. Guest calls host functions through the bridge
5. Host executes real Salesforce API calls
6. Responses flow back to guest
7. Guest never sees credentials
