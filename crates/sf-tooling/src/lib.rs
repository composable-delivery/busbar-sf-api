//! # sf-tooling
//!
//! Salesforce Tooling API client for development and deployment operations.
//!
//! ## Features
//!
//! - **Apex Operations** - Execute anonymous Apex, query Apex logs
//! - **Metadata Query** - Query ApexClass, ApexTrigger, and other tooling objects
//! - **Debug Logs** - Retrieve and manage debug logs
//! - **Trace Flags** - Manage debug trace flags
//! - **Test Execution** - Run Apex and Flow tests (async/sync, discovery, v65.0+ unified API)
//! - **Code Coverage** - Get code coverage information
//! - **Describe** - Get tooling object metadata
//!
//! ## Example
//!
//! ```rust,ignore
//! use sf_tooling::ToolingClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), sf_tooling::Error> {
//!     let client = ToolingClient::new(
//!         "https://myorg.my.salesforce.com",
//!         "access_token_here",
//!     )?;
//!
//!     // Execute anonymous Apex
//!     let result = client
//!         .execute_anonymous("System.debug('Hello World');")
//!         .await?;
//!
//!     if result.success {
//!         println!("Apex executed successfully");
//!     }
//!
//!     // Query Apex classes
//!     let classes: Vec<ApexClass> = client
//!         .query_all("SELECT Id, Name, Body FROM ApexClass LIMIT 10")
//!         .await?;
//!
//!     // Get debug logs
//!     let logs = client.get_apex_logs(Some(10)).await?;
//!
//!     Ok(())
//! }
//! ```

mod client;
mod error;
mod types;

pub use client::ToolingClient;
pub use error::{Error, ErrorKind, Result};
pub use types::*;

// Re-export busbar-sf-client types that users might need
pub use busbar_sf_client::{ClientConfig, ClientConfigBuilder, QueryResult};

// Re-export composite and collection types from sf-rest for Tooling API usage
pub use busbar_sf_rest::{
    CollectionRequest, CollectionResult, CompositeBatchRequest, CompositeBatchResponse,
    CompositeBatchSubrequest, CompositeBatchSubresponse, CompositeRequest, CompositeResponse,
    CompositeSubrequest, CompositeSubresponse, CompositeTreeAttributes, CompositeTreeError,
    CompositeTreeRecord, CompositeTreeRequest, CompositeTreeResponse, CompositeTreeResult,
    DescribeGlobalResult, DescribeSObjectResult,
};
