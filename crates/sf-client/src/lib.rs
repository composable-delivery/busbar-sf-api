//! # sf-client
//!
//! Core HTTP client infrastructure for Salesforce APIs.
//!
//! This crate provides the foundational HTTP client with:
//! - Automatic retry with exponential backoff and jitter
//! - Compression support (gzip, deflate)
//! - Rate limit detection and handling
//! - ETag/conditional request support
//! - Connection pooling
//! - Request/response tracing
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Application Layer                        │
//! │  (sf-rest, sf-bulk, sf-metadata, sf-tooling)               │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   SalesforceClient                          │
//! │  - Holds credentials + HTTP client                          │
//! │  - Provides typed JSON methods (get_json, post_json, etc.)  │
//! │  - Handles authentication headers                           │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    SfHttpClient                             │
//! │  - Raw HTTP with retry, compression, rate limiting          │
//! │  - Request building with conditionals                       │
//! │  - Response handling                                        │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use sf_client::{SalesforceClient, ClientConfig};
//! use sf_auth::SalesforceCredentials;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), sf_client::Error> {
//!     let creds = SalesforceCredentials::from_env()?;
//!     let client = SalesforceClient::new(creds)?;
//!
//!     // Typed JSON request
//!     let user: serde_json::Value = client
//!         .get_json("/services/oauth2/userinfo")
//!         .await?;
//!
//!     // POST with body
//!     let result: CreateResult = client
//!         .post_json("/services/data/v62.0/sobjects/Account", &new_account)
//!         .await?;
//!
//!     Ok(())
//! }
//! ```

// Compile-time check: ensure exactly one backend is enabled
#[cfg(all(feature = "native", feature = "wasm"))]
compile_error!("Cannot enable both 'native' and 'wasm' features simultaneously. Please enable only one.");

#[cfg(not(any(feature = "native", feature = "wasm")))]
compile_error!("At least one backend feature must be enabled: 'native' or 'wasm'.");

mod client;
mod config;
mod error;
mod request;
mod response;
mod retry;
mod salesforce_client;
pub mod security;

pub use client::SfHttpClient;
pub use config::{ClientConfig, ClientConfigBuilder, CompressionConfig};
pub use error::{Error, ErrorKind, Result};
pub use request::{RequestBuilder, RequestMethod};
pub use response::{ApiUsage, Response, ResponseExt};
pub use retry::{BackoffStrategy, RetryConfig, RetryPolicy};
pub use salesforce_client::QueryResult;

#[cfg(feature = "native")]
pub use salesforce_client::SalesforceClient;

/// Default Salesforce API version
pub const DEFAULT_API_VERSION: &str = "62.0";

/// User-Agent string for the client
pub const USER_AGENT: &str = concat!("busbar-sf-api/", env!("CARGO_PKG_VERSION"));
