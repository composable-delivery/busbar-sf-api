//! Integration test plugin for sf-client WASM support.
//!
//! This plugin demonstrates and tests the WASM functionality of sf-client.

use extism_pdk::*;
use busbar_sf_client::{SfHttpClient, ClientConfig};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct TestRequest {
    url: String,
    method: String,
    bearer_token: Option<String>,
}

#[derive(Serialize)]
struct TestResponse {
    success: bool,
    status: u16,
    error: Option<String>,
}

/// Test basic GET request functionality
#[plugin_fn]
pub fn test_get_request(input: String) -> FnResult<Json<TestResponse>> {
    let req: TestRequest = serde_json::from_str(&input)
        .map_err(|e| Error::msg(format!("Failed to parse input: {}", e)))?;
    
    // Create WASM client (no retry support)
    let client = SfHttpClient::default_client()
        .map_err(|e| Error::msg(format!("Failed to create client: {}", e)))?;
    
    // Build and execute request
    let mut request_builder = client.get(&req.url);
    
    if let Some(token) = req.bearer_token {
        request_builder = request_builder.bearer_auth(token);
    }
    
    match client.execute(request_builder) {
        Ok(response) => {
            Ok(Json(TestResponse {
                success: response.is_success(),
                status: response.status(),
                error: None,
            }))
        }
        Err(e) => {
            Ok(Json(TestResponse {
                success: false,
                status: 0,
                error: Some(format!("{}", e)),
            }))
        }
    }
}

/// Test POST request with JSON body
#[plugin_fn]
pub fn test_post_json(input: String) -> FnResult<Json<TestResponse>> {
    let req: TestRequest = serde_json::from_str(&input)
        .map_err(|e| Error::msg(format!("Failed to parse input: {}", e)))?;
    
    let client = SfHttpClient::default_client()
        .map_err(|e| Error::msg(format!("Failed to create client: {}", e)))?;
    
    let test_data = serde_json::json!({
        "test": "data",
        "method": req.method
    });
    
    let mut request_builder = client.post(&req.url)
        .json(&test_data)
        .map_err(|e| Error::msg(format!("Failed to build JSON request: {}", e)))?;
    
    if let Some(token) = req.bearer_token {
        request_builder = request_builder.bearer_auth(token);
    }
    
    match client.execute(request_builder) {
        Ok(response) => {
            Ok(Json(TestResponse {
                success: response.is_success(),
                status: response.status(),
                error: None,
            }))
        }
        Err(e) => {
            Ok(Json(TestResponse {
                success: false,
                status: 0,
                error: Some(format!("{}", e)),
            }))
        }
    }
}

/// Test client configuration
#[plugin_fn]
pub fn test_client_config(_: ()) -> FnResult<Json<TestResponse>> {
    use std::time::Duration;
    
    // Test creating client with custom config
    let config = ClientConfig::builder()
        .without_retry()
        .with_timeout(Duration::from_secs(30))
        .with_user_agent("WASM-Test/1.0")
        .with_compression(true)
        .build();
    
    match SfHttpClient::new(config) {
        Ok(client) => {
            let cfg = client.config();
            let success = cfg.timeout == Duration::from_secs(30)
                && cfg.user_agent == "WASM-Test/1.0"
                && cfg.compression.enabled
                && cfg.retry.is_none();
            
            Ok(Json(TestResponse {
                success,
                status: 200,
                error: if !success { Some("Config validation failed".to_string()) } else { None },
            }))
        }
        Err(e) => {
            Ok(Json(TestResponse {
                success: false,
                status: 0,
                error: Some(format!("{}", e)),
            }))
        }
    }
}

/// Test that retry configuration is rejected in WASM
#[plugin_fn]
pub fn test_retry_rejected(_: ()) -> FnResult<Json<TestResponse>> {
    let config = ClientConfig::builder()
        .with_retry(busbar_sf_client::RetryConfig::default())
        .build();
    
    match SfHttpClient::new(config) {
        Ok(_) => {
            // This should not succeed
            Ok(Json(TestResponse {
                success: false,
                status: 0,
                error: Some("Client with retry should have been rejected".to_string()),
            }))
        }
        Err(_) => {
            // This is expected - retry should be rejected
            Ok(Json(TestResponse {
                success: true,
                status: 200,
                error: None,
            }))
        }
    }
}
