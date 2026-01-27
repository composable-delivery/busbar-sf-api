//! # sf-auth
//!
//! Salesforce authentication library supporting secure OAuth 2.0 flows.
//!
//! ## Security
//!
//! This library is designed with security in mind:
//! - Sensitive data (tokens, secrets) are redacted in Debug output
//! - Tracing/logging skips credential parameters
//! - Error messages sanitize any credential data
//! - Device Code Flow excluded (deprecated for security reasons)
//!
//! ## Supported Authentication Methods
//!
//! - **OAuth 2.0 Web Server Flow** - For web applications with user interaction
//! - **OAuth 2.0 JWT Bearer Flow** - For server-to-server integration
//! - **OAuth 2.0 Refresh Token** - For refreshing expired access tokens
//!
//! ## Example
//!
//! ```rust,ignore
//! use sf_auth::{Credentials, SalesforceCredentials, JwtAuth};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), sf_auth::Error> {
//!     // From environment variables
//!     let creds = SalesforceCredentials::from_env()?;
//!
//!     // From SFDX CLI
//!     let creds = SalesforceCredentials::from_sfdx_alias("myorg").await?;
//!
//!     // JWT Bearer Flow (server-to-server)
//!     let private_key = std::fs::read("path/to/key.pem")?;
//!     let jwt_auth = JwtAuth::new("consumer_key", "username", private_key);
//!     let token = jwt_auth.authenticate("https://login.salesforce.com").await?;
//!
//!     Ok(())
//! }
//! ```

mod credentials;
mod error;
mod jwt;
mod oauth;
mod storage;

pub use credentials::{Credentials, SalesforceCredentials};
pub use error::{Error, ErrorKind, Result};
pub use jwt::JwtAuth;
pub use oauth::{OAuthClient, OAuthConfig, TokenInfo, TokenResponse, WebFlowAuth};
pub use storage::{FileTokenStorage, TokenStorage};

/// Default Salesforce login URL for production.
pub const PRODUCTION_LOGIN_URL: &str = "https://login.salesforce.com";

/// Default Salesforce login URL for sandbox.
pub const SANDBOX_LOGIN_URL: &str = "https://test.salesforce.com";
