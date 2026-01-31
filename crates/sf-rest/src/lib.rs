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

mod client;
mod collections;
mod composite;
mod consent;
mod describe;
mod embedded_service;
mod error;
mod invocable_actions;
mod knowledge;
mod layout;
mod list_views;
mod process;
mod query;
mod query_builder;
mod quick_actions;
mod scheduler;
mod search;
mod sobject;
mod types;
mod user_password;

// Main client
pub use client::{
    ApiVersion, DeletedRecord, GetDeletedResult, GetUpdatedResult, SObjectInfo,
    SObjectInfoDescribe, SalesforceRestClient, SearchResult,
};

// Collection operations
pub use collections::{CollectionRequest, CollectionResult};

// Composite API
pub use composite::{
    CompositeBatchRequest, CompositeBatchResponse, CompositeBatchSubrequest,
    CompositeBatchSubresponse, CompositeGraphRequest, CompositeGraphResponse, CompositeRequest,
    CompositeResponse, CompositeSubrequest, CompositeSubresponse, CompositeTreeAttributes,
    CompositeTreeError, CompositeTreeRecord, CompositeTreeRequest, CompositeTreeResponse,
    CompositeTreeResult, GraphRequest, GraphResponse, GraphResponseBody,
};

// Convenience aliases for SObject Tree types
pub use composite::CompositeBatchSubrequest as BatchSubrequest;
pub use composite::CompositeTreeAttributes as SObjectTreeAttributes;
pub use composite::CompositeTreeRecord as SObjectTreeRecord;
pub use composite::CompositeTreeRequest as SObjectTreeRequest;

// Describe types
pub use describe::{
    ActionOverride, ChildRelationship, DescribeGlobalResult, DescribeSObjectResult, FieldDescribe,
    FilteredLookupInfo, NamedLayoutInfo, PicklistValue, RecordTypeInfo, SObjectBasicInfo,
    ScopeInfo,
};

// Layout types
pub use layout::{
    ApprovalLayoutsResult, CompactLayoutsResult, DescribeLayoutsResult,
    GlobalPublisherLayoutsResult, NamedLayoutResult,
};

// Search types
pub use search::{
    ParameterizedSearchRequest, ParameterizedSearchResponse, ScopeEntity, SearchLayoutColumn,
    SearchLayoutInfo, SearchMetadata, SearchRecordAttributes, SearchRecordGroup, SearchSObjectSpec,
    SearchSuggestionResult, Suggestion, SuggestionAttributes,
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

// PR #53: Quick Actions types
pub use quick_actions::{QuickAction, QuickActionDescribe, QuickActionIcon, QuickActionResult};

// PR #53: List View types
pub use list_views::{
    ListView, ListViewCollection, ListViewColumn, ListViewDescribe, ListViewOrderBy, ListViewResult,
};

// PR #53: Process and Approval types
pub use process::{
    ApprovalActionType, ApprovalRequest, ApprovalResult, PendingApproval,
    PendingApprovalCollection, ProcessRule, ProcessRuleCollection, ProcessRuleRequest,
    ProcessRuleResult,
};

// PR #53: Invocable Action types
pub use invocable_actions::{
    InvocableAction, InvocableActionCollection, InvocableActionDescribe, InvocableActionParameter,
    InvocableActionRequest, InvocableActionResult,
};

// PR #54: Consent types
pub use consent::{
    ConsentRecord, ConsentResponse, ConsentWriteRecord, ConsentWriteRequest, MultiConsentResponse,
};

// PR #54: Knowledge types
pub use knowledge::{
    DataCategoriesResponse, DataCategory, DataCategoryGroup, DataCategoryGroupsResponse,
    KnowledgeArticle, KnowledgeArticlesResponse, KnowledgeSettings,
};

// PR #54: User Password types
pub use user_password::{SetPasswordRequest, SetPasswordResponse, UserPasswordStatus};

// PR #54: Scheduler types
pub use scheduler::{
    AppointmentCandidate, AppointmentCandidatesRequest, AppointmentCandidatesResponse,
};

// PR #54: Embedded Service types
pub use embedded_service::EmbeddedServiceConfig;

// Re-export sf-client types that users might need
pub use busbar_sf_client::{ClientConfig, ClientConfigBuilder};
