//! Example WASM guest plugin for the busbar-sf bridge.
//!
//! This plugin demonstrates how to use the guest SDK to interact with
//! Salesforce APIs from a sandboxed WASM environment.
//!
//! ## Building
//!
//! ```sh
//! # Install the WASM target (one-time)
//! rustup target add wasm32-unknown-unknown
//!
//! # Build the plugin
//! cargo build --target wasm32-unknown-unknown --release
//! ```
//!
//! ## Running
//!
//! The host loads this .wasm file and calls the exported functions.
//! The host has already authenticated with Salesforce and manages
//! credentials. This plugin never sees any tokens.

use busbar_sf_guest_sdk::*;
use extism_pdk::*;
use serde::{Deserialize, Serialize};

/// Input for the query_accounts function.
#[derive(Debug, Deserialize)]
struct QueryAccountsInput {
    /// Maximum number of accounts to return.
    limit: Option<u32>,
}

/// Output from the query_accounts function.
#[derive(Debug, Serialize)]
struct QueryAccountsOutput {
    total: u64,
    accounts: Vec<serde_json::Value>,
}

/// Query Salesforce accounts.
///
/// The host calls this function. The plugin uses the guest SDK to query
/// Salesforce through the bridge - never seeing the access token.
#[plugin_fn]
pub fn query_accounts(input: String) -> FnResult<Json<QueryAccountsOutput>> {
    let input: QueryAccountsInput = serde_json::from_str(&input)
        .map_err(|e| Error::msg(format!("invalid input: {e}")))?;

    let limit = input.limit.unwrap_or(10);
    let limit_str = limit.to_string();
    let escaped_limit = soql::escape_string(&limit_str);
    let soql = format!(
        "SELECT Id, Name, Industry FROM Account LIMIT {}",
        escaped_limit
    );

    let result = query(&soql)?;

    Ok(Json(QueryAccountsOutput {
        total: result.total_size,
        accounts: result.records,
    }))
}

/// Create a new account.
#[plugin_fn]
pub fn create_account(input: String) -> FnResult<Json<CreateResponse>> {
    let record: serde_json::Value = serde_json::from_str(&input)
        .map_err(|e| Error::msg(format!("invalid input: {e}")))?;

    let result = create("Account", &record)?;
    Ok(Json(result))
}

/// Demonstrate a composite operation: create an account and a related contact.
#[plugin_fn]
pub fn create_account_with_contact(input: String) -> FnResult<Json<CompositeResponse>> {
    let input: serde_json::Value = serde_json::from_str(&input)
        .map_err(|e| Error::msg(format!("invalid input: {e}")))?;

    let account_name = input["account_name"]
        .as_str()
        .unwrap_or("Default Account");
    let contact_name = input["contact_last_name"]
        .as_str()
        .unwrap_or("Default Contact");

    let request = CompositeRequest {
        all_or_none: true,
        subrequests: vec![
            CompositeSubrequest {
                method: "POST".to_string(),
                url: "/services/data/v62.0/sobjects/Account".to_string(),
                reference_id: "NewAccount".to_string(),
                body: Some(serde_json::json!({
                    "Name": account_name
                })),
            },
            CompositeSubrequest {
                method: "POST".to_string(),
                url: "/services/data/v62.0/sobjects/Contact".to_string(),
                reference_id: "NewContact".to_string(),
                body: Some(serde_json::json!({
                    "LastName": contact_name,
                    "AccountId": "@{NewAccount.id}"
                })),
            },
        ],
    };

    let result = composite(&request)?;
    Ok(Json(result))
}

/// Get API limits - demonstrates a parameterless host function call.
#[plugin_fn]
pub fn check_limits(_input: String) -> FnResult<Json<serde_json::Value>> {
    let result = limits()?;
    Ok(Json(result))
}
