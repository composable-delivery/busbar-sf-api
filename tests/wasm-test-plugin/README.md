# WASM Test Plugin

This test plugin exercises the WASM bridge's host functions through comprehensive integration tests.

## Building

**Note**: Due to workspace inheritance limitations with excluded packages, this plugin cannot be built with a simple `cargo build` command. Use one of these methods:

### Method 1: Build Script (Recommended)

From the repository root:

```bash
bash tests/wasm-test-plugin/build.sh
```

### Method 2: Manual Build with Patching

Temporarily modify the guest SDK's `Cargo.toml` to use explicit values instead of workspace inheritance:

```bash
# In crates/sf-guest-sdk/Cargo.toml, replace workspace = true with explicit values
edition = "2021"
version = "0.0.3"
# ... etc

cargo build --manifest-path tests/wasm-test-plugin/Cargo.toml --target wasm32-unknown-unknown --release
```

### Method 3: CI/CD

The CI pipeline should build this plugin before running integration tests:

```yaml
- name: Build WASM test plugin
  run: |
    rustup target add wasm32-unknown-unknown
    bash tests/wasm-test-plugin/build.sh
```

## Testing

Once built, the integration tests in `tests/integration/bridge.rs` will automatically find and use the compiled WASM at:

```
target/wasm32-unknown-unknown/release/wasm_test_plugin.wasm
```

If the WASM file is not present, the tests will skip with a warning message.

## Architecture

The test plugin exports functions that exercise different Salesforce APIs through the guest SDK:

- **Core CRUD**: `test_query`, `test_crud_operations`, `test_upsert`
- **Composite**: `test_composite`, `test_composite_batch`, `test_composite_tree`
- **Batch Operations**: `test_batch_operations`
- **Describe**: `test_describe_global`, `test_describe_sobject`
- **Process & Approvals**: `test_process_rules`, `test_list_pending_approvals`
- **List Views**: `test_list_views`, `test_list_view_operations`
- **Quick Actions**: `test_list_global_quick_actions`, `test_list_quick_actions`
- **Search**: `test_search`, `test_parameterized_search`
- **Sync**: `test_get_deleted`, `test_get_updated`
- **Bulk API**: `test_bulk_ingest`, `test_bulk_query`
- **Tooling API**: `test_tooling_query`, `test_execute_anonymous_apex`
- **Metadata API**: `test_metadata_list`, `test_metadata_describe`
- **Layouts**: `test_get_sobject_layouts`, `test_get_compact_layouts`
- **Named Credentials**: `test_list_named_credentials`, `test_get_named_credential`
- **Limits/Versions**: `test_limits`, `test_versions`

Each function takes JSON input, performs the operation via the guest SDK, and returns JSON with test results.
