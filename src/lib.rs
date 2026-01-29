//! # busbar-sf-api
//!
//! A comprehensive Salesforce API client library for Rust.
//!
//! This library provides type-safe access to Salesforce APIs with built-in
//! authentication, retry logic, and error handling.
//!
//! ## Security
//!
//! This library is designed with security in mind:
//! - Sensitive data (tokens, secrets) are redacted in Debug output
//! - Tracing/logging skips credential parameters
//! - Error messages sanitize any credential data
//!
//! ## Crates
//!
//! - **busbar-sf-client** - Core HTTP client infrastructure with retry, compression, rate limiting
//! - **busbar-sf-auth** - Authentication: OAuth 2.0 flows, JWT Bearer, credentials management
//! - **busbar-sf-rest** - REST API: CRUD, Query, Describe, Composite, Collections
//! - **busbar-sf-tooling** - Tooling API: Apex operations, debug logs, code coverage
//! - **busbar-sf-bulk** - Bulk API 2.0: Large-scale data operations
//! - **busbar-sf-metadata** - Metadata API: Deploy and retrieve metadata
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use busbar_sf_auth::SalesforceCredentials;
//! use busbar_sf_rest::SalesforceRestClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Get credentials from SF CLI
//!     let creds = SalesforceCredentials::from_sfdx_alias("my-org").await?;
//!
//!     // Create REST client
//!     let client = SalesforceRestClient::new(
//!         creds.instance_url(),
//!         creds.access_token(),
//!     )?;
//!
//!     // Query accounts
//!     let accounts: Vec<serde_json::Value> = client
//!         .query_all("SELECT Id, Name FROM Account LIMIT 10")
//!         .await?;
//!
//!     for account in accounts {
//!         println!("{}", account["Name"]);
//!     }
//!
//!     Ok(())
//! }
//! ```

// Re-export all crates for convenient access
#[cfg(feature = "auth")]
pub use busbar_sf_auth as auth;

#[cfg(feature = "bulk")]
pub use busbar_sf_bulk as bulk;

#[cfg(feature = "client")]
pub use busbar_sf_client as client;

#[cfg(feature = "metadata")]
pub use busbar_sf_metadata as metadata;

#[cfg(feature = "rest")]
pub use busbar_sf_rest as rest;

#[cfg(feature = "tooling")]
pub use busbar_sf_tooling as tooling;

// Re-export commonly used types at the top level
#[cfg(feature = "auth")]
pub use busbar_sf_auth::{Credentials, SalesforceCredentials};

#[cfg(feature = "client")]
pub use busbar_sf_client::{ClientConfig, SalesforceClient};

#[cfg(feature = "rest")]
pub use busbar_sf_rest::SalesforceRestClient;

#[cfg(feature = "tooling")]
pub use busbar_sf_tooling::ToolingClient;
