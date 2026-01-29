//! # sf-rest
//!
//! Salesforce REST API client with full CRUD, Query, and Composite API support.
//!
//! ## Features
//!
//! - **SObject CRUD** - Create, Read, Update, Delete individual records
//! - **SObject Collections** - Batch operations for up to 200 records
//! - **SOQL Query** - Execute queries with automatic pagination
//! - **SOSL Search** - Full-text search across objects
//! - **Describe** - Get object and field metadata
//! - **Composite API** - Execute multiple operations in a single request
//! - **Limits** - Check API usage and limits
//!
//! ## Example
//!
//! ```rust,ignore
//! use sf_rest::SalesforceRestClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), sf_rest::Error> {
//!     let client = SalesforceRestClient::new(
//!         "https://myorg.my.salesforce.com",
//!         "access_token_here",
//!     )?;
//!
//!     // Query
//!     let accounts: Vec<serde_json::Value> = client
//!         .query_all("SELECT Id, Name FROM Account LIMIT 10")
//!         .await?;
//!
//!     // Create
//!     let id = client
//!         .create("Account", &serde_json::json!({"Name": "New Account"}))
//!         .await?;
//!
//!     // Update
//!     client
//!         .update("Account", &id, &serde_json::json!({"Name": "Updated"}))
//!         .await?;
//!
//!     // Delete
//!     client.delete("Account", &id).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod actions;
mod client;
mod collections;
mod composite;
pub mod consent;
mod describe;
pub mod embedded_service;
mod error;
pub mod knowledge;
mod query;
mod query_builder;
pub mod scheduler;
mod sobject;
mod types;
pub mod user_password;

// Main client
pub use client::{ApiVersion, SalesforceRestClient, SearchResult};

// Collection operations
pub use collections::{CollectionRequest, CollectionResult};

// Composite API
pub use composite::{
    CompositeBatchRequest, CompositeBatchResponse, CompositeBatchSubrequest,
    CompositeBatchSubresponse, CompositeRequest, CompositeResponse, CompositeSubrequest,
    CompositeSubresponse, CompositeTreeAttributes, CompositeTreeError, CompositeTreeRecord,
    CompositeTreeRequest, CompositeTreeResponse, CompositeTreeResult,
};

// Describe types
pub use describe::{
    ActionOverride, ChildRelationship, DescribeGlobalResult, DescribeSObjectResult, FieldDescribe,
    FilteredLookupInfo, NamedLayoutInfo, PicklistValue, RecordTypeInfo, SObjectBasicInfo,
    ScopeInfo,
};

// Error types
pub use error::{Error, ErrorKind, Result};

// Query types
pub use query::{QueryOptions, QueryResult};

// Query builder (safe by default)
pub use query_builder::QueryBuilder;

// SObject CRUD types
pub use sobject::{CreateResult, DeleteResult, SalesforceError, UpdateResult, UpsertResult};

// Common types
pub use types::*;

// Re-export sf-client types that users might need
pub use busbar_sf_client::{ClientConfig, ClientConfigBuilder};
