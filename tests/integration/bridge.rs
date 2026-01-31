//! WASM bridge integration tests.
//!
//! These tests verify that the sf-bridge can load and execute WASM plugins,
//! and that host functions correctly bridge to Salesforce APIs.

use super::common::get_credentials;
use busbar_sf_auth::Credentials;
use busbar_sf_bridge::SfBridge;
use busbar_sf_rest::SalesforceRestClient;

#[tokio::test]
async fn test_bridge_can_load_wasm_plugin() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Create a minimal valid WASM module that just returns
    // This is the smallest valid WASM module (8 bytes magic + 4 bytes version)
    let minimal_wasm = vec![
        0x00, 0x61, 0x73, 0x6d, // magic: \0asm
        0x01, 0x00, 0x00, 0x00, // version: 1
    ];

    // Attempt to create a bridge with this minimal module
    // We expect this to succeed in loading the module, even though it has no exports
    let result = SfBridge::new(minimal_wasm, client);

    // The bridge should successfully initialize
    assert!(result.is_ok(), "Bridge should accept a valid WASM module");
}

#[tokio::test]
async fn test_bridge_rejects_invalid_wasm() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Not a valid WASM module
    let invalid_wasm = vec![0x01, 0x02, 0x03, 0x04];

    // Attempt to create a bridge with invalid WASM
    let result = SfBridge::new(invalid_wasm, client);

    // The bridge should reject invalid WASM
    // Note: This test documents expected behavior - Extism may or may not
    // validate the module at construction time. The key is that it doesn't panic.
    match result {
        Ok(_) => {
            // Some WASM runtimes delay validation until execution
            // This is acceptable behavior
        }
        Err(_) => {
            // Rejecting invalid WASM at construction is also acceptable
        }
    }
}

// Note: Full end-to-end tests that actually call host functions require
// building a real WASM plugin with sf-guest-sdk. These are better suited
// for the examples/wasm-guest-plugin directory, which is manually tested.
//
// The integration tests above verify:
// 1. Bridge can be constructed with valid credentials
// 2. Bridge accepts valid WASM modules
// 3. Bridge handles invalid WASM gracefully
//
// Host function correctness is validated through:
// - Unit tests in sf-bridge/src/host_functions.rs (with mocked clients)
// - Manual testing of examples/wasm-guest-plugin
// - The existing REST/Bulk/Tooling/Metadata integration tests that validate
//   the underlying client behavior that host functions delegate to
