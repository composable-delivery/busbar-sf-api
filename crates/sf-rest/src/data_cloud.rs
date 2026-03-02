//! Data Cloud (Data 360) API types.
//!
//! These types support the Salesforce Data Cloud (formerly CDP/C360) API endpoints:
//! - SQL query via `/services/data/v{version}/ssot/query-sql`
//! - Vector search via `/services/data/v{version}/ssot/search-vector`
//! - Unified profile via `/api/v1/profile/{dataModelName}`
//! - Metadata discovery via `/api/v1/metadata`
//!
//! All Data Cloud calls go to the TSE (Tenant Service Endpoint) URL and require
//! a Data Cloud access token obtained via
//! [`OAuthClient::exchange_for_data_cloud`](busbar_sf_auth::OAuthClient::exchange_for_data_cloud).

use serde::{Deserialize, Serialize};

/// Request body for a Data Cloud SQL query.
///
/// Used with both synchronous and asynchronous query execution.
#[derive(Debug, Clone, Serialize)]
pub struct DataCloudQueryRequest {
    /// ANSI SQL statement to execute against Data Model Objects (DMOs).
    pub sql: String,
    /// Number of rows to return per page. Defaults to the server default when `None`.
    #[serde(rename = "pageSize", skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u32>,
    /// When `true`, submit an asynchronous query and receive a `queryId` instead
    /// of immediate results. Poll status with [`DataCloudClient::query_status`](crate::DataCloudClient::query_status)
    /// and fetch rows with [`DataCloudClient::query_rows`](crate::DataCloudClient::query_rows).
    #[serde(rename = "async", skip_serializing_if = "Option::is_none")]
    pub r#async: Option<bool>,
}

/// Metadata about a column in a Data Cloud query result.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ColumnInfo {
    /// Column name (may retain original API casing as of 2026, e.g. `ssot__Id__c`).
    pub name: String,
    /// Column data type, e.g. `"numeric"`, `"varchar"`, `"timestamp_with_timezone"`.
    #[serde(rename = "type")]
    pub col_type: String,
}

/// Metadata section of a Data Cloud query response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QueryMetadata {
    /// Ordered list of columns returned by the query.
    pub columns: Vec<ColumnInfo>,
}

/// Response from a synchronous Data Cloud SQL query.
///
/// For asynchronous queries, the `query_id` field is populated and `data`/`metadata`
/// may be empty until the query completes.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataCloudQueryResponse {
    /// Result rows as a list of JSON objects (one per row).
    pub data: Vec<serde_json::Value>,
    /// Column metadata for the result set.
    pub metadata: QueryMetadata,
    /// `true` when all rows have been returned (no more pages).
    pub done: bool,
    /// Query ID for async queries or for fetching subsequent pages.
    #[serde(rename = "queryId", default)]
    pub query_id: Option<String>,
    /// ID of the next batch when paginating large result sets.
    #[serde(rename = "nextBatchId", default)]
    pub next_batch_id: Option<String>,
}

/// Status of an asynchronous Data Cloud query.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AsyncQueryStatus {
    /// Unique identifier for this async query.
    #[serde(rename = "queryId")]
    pub query_id: String,
    /// Current status string, e.g. `"running"`, `"success"`, `"failed"`.
    pub status: String,
    /// Error message when `status` is `"failed"`.
    #[serde(rename = "errorMessage", default)]
    pub error_message: Option<String>,
}

/// Request body for a Data Cloud vector search (RAG).
///
/// Used with the `/services/data/v{version}/ssot/search-vector` endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct VectorSearchRequest {
    /// Name of the vector search index to query.
    #[serde(rename = "indexName")]
    pub index_name: String,
    /// Natural-language query text to embed and search with.
    #[serde(rename = "queryText")]
    pub query_text: String,
    /// Maximum number of results to return. Defaults to the server default when `None`.
    #[serde(rename = "topK", skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
}

/// A single chunk returned by a vector search.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VectorSearchResult {
    /// Source record ID for the chunk.
    pub id: String,
    /// Similarity score (higher is more similar).
    pub score: f64,
    /// Text content of the matched chunk.
    pub content: String,
}

/// Response from a Data Cloud vector search.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VectorSearchResponse {
    /// Ordered list of matching chunks (most similar first).
    pub results: Vec<VectorSearchResult>,
}

/// A Data Model Object (DMO) or other metadata entity returned by the discovery API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataCloudMetadataObject {
    /// API name of the entity.
    pub name: String,
    /// Display label.
    #[serde(default)]
    pub label: Option<String>,
    /// Entity type, e.g. `"DataModelObject"`.
    #[serde(rename = "entityType", default)]
    pub entity_type: Option<String>,
    /// Field definitions (structure varies by entity type).
    #[serde(default)]
    pub fields: Option<Vec<serde_json::Value>>,
}

/// Response from the Data Cloud metadata discovery API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataCloudMetadataResponse {
    /// List of metadata entities.
    pub metadata: Vec<DataCloudMetadataObject>,
}
