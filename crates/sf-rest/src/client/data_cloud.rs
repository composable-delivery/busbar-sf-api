//! Data Cloud (Data 360) API client.
//!
//! [`DataCloudClient`] provides typed access to Salesforce Data Cloud endpoints.
//! Because Data Cloud runs on a separate infrastructure (the TSE — Tenant Service
//! Endpoint), it uses a different base URL and a different access token from the
//! standard Salesforce REST API.
//!
//! ## Quickstart
//!
//! ```rust,ignore
//! use busbar_sf_auth::{OAuthClient, OAuthConfig};
//! use busbar_sf_rest::DataCloudClient;
//! use busbar_sf_rest::data_cloud::DataCloudQueryRequest;
//!
//! // 1. Exchange your SF access token for a Data Cloud token.
//! let oauth = OAuthClient::new(OAuthConfig::new("consumer_key"));
//! let dc_token = oauth
//!     .exchange_for_data_cloud(&sf_access_token, &sf_instance_url)
//!     .await?;
//!
//! // 2. Build a DataCloudClient using the TSE URL and Data Cloud token.
//! let client = DataCloudClient::new(dc_token.instance_url, dc_token.access_token)?;
//!
//! // 3. Execute a SQL query.
//! let response = client
//!     .query_sql(&DataCloudQueryRequest {
//!         sql: "SELECT ssot__Id__c, ssot__Name__c FROM Individual__dlm LIMIT 10".into(),
//!         page_size: None,
//!         r#async: None,
//!     })
//!     .await?;
//! ```

use tracing::instrument;

use busbar_sf_client::{ClientConfig, SalesforceClient};

use crate::data_cloud::{
    AsyncQueryStatus, DataCloudMetadataResponse, DataCloudQueryRequest, DataCloudQueryResponse,
    VectorSearchRequest, VectorSearchResponse,
};
use crate::error::Result;

/// Client for Salesforce Data Cloud (Data 360) APIs.
///
/// Data Cloud endpoints are hosted on a separate infrastructure called the
/// TSE (Tenant Service Endpoint). This client wraps a [`SalesforceClient`] pointed at
/// the TSE URL with a Data Cloud access token.
///
/// Obtain the TSE URL and Data Cloud token by calling
/// [`OAuthClient::exchange_for_data_cloud`](busbar_sf_auth::OAuthClient::exchange_for_data_cloud)
/// with a valid Salesforce access token.
///
/// # Example
///
/// ```rust,ignore
/// use busbar_sf_rest::DataCloudClient;
/// use busbar_sf_rest::data_cloud::DataCloudQueryRequest;
///
/// let client = DataCloudClient::new(
///     "https://something.c360a.salesforce.com",
///     "data_cloud_access_token",
/// )?;
///
/// let result = client
///     .query_sql(&DataCloudQueryRequest {
///         sql: "SELECT ssot__Id__c FROM Individual__dlm LIMIT 5".into(),
///         page_size: None,
///         r#async: None,
///     })
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct DataCloudClient {
    client: SalesforceClient,
    /// Data Cloud API version (without "v" prefix, e.g. `"64.0"`).
    api_version: String,
}

impl DataCloudClient {
    /// Default API version for Data Cloud endpoints.
    const DEFAULT_DC_API_VERSION: &'static str = "64.0";

    /// Create a new Data Cloud client.
    ///
    /// # Arguments
    ///
    /// * `tse_url` — The TSE (Tenant Service Endpoint) base URL returned by the token exchange.
    /// * `access_token` — The Data Cloud access token returned by the token exchange.
    pub fn new(tse_url: impl Into<String>, access_token: impl Into<String>) -> Result<Self> {
        let client = SalesforceClient::new(tse_url, access_token)?;
        Ok(Self {
            client: client.with_api_version(Self::DEFAULT_DC_API_VERSION),
            api_version: Self::DEFAULT_DC_API_VERSION.to_string(),
        })
    }

    /// Create a new Data Cloud client with custom HTTP configuration.
    pub fn with_config(
        tse_url: impl Into<String>,
        access_token: impl Into<String>,
        config: ClientConfig,
    ) -> Result<Self> {
        let client = SalesforceClient::with_config(tse_url, access_token, config)?;
        Ok(Self {
            client: client.with_api_version(Self::DEFAULT_DC_API_VERSION),
            api_version: Self::DEFAULT_DC_API_VERSION.to_string(),
        })
    }

    /// Override the Data Cloud API version (default: `"64.0"`).
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        let version = version.into();
        self.client = self.client.with_api_version(version.clone());
        self.api_version = version;
        self
    }

    /// Get the TSE (Tenant Service Endpoint) URL.
    pub fn tse_url(&self) -> &str {
        self.client.instance_url()
    }

    /// Get the current API version.
    pub fn api_version(&self) -> &str {
        &self.api_version
    }

    // =========================================================================
    // SQL Query API  (/services/data/v{version}/ssot/query-sql)
    // =========================================================================

    /// Execute a synchronous or asynchronous Data Cloud SQL query.
    ///
    /// Executes ANSI SQL against Data Model Objects (DMOs). For large result sets,
    /// set `request.r#async = Some(true)` to receive a `queryId` and poll with
    /// [`query_status`](Self::query_status) / [`query_rows`](Self::query_rows).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let response = client
    ///     .query_sql(&DataCloudQueryRequest {
    ///         sql: "SELECT ssot__Id__c, ssot__Name__c FROM Individual__dlm LIMIT 100".into(),
    ///         page_size: Some(50),
    ///         r#async: None,
    ///     })
    ///     .await?;
    ///
    /// for row in &response.data {
    ///     println!("{}", row);
    /// }
    /// ```
    #[instrument(skip(self, request))]
    pub async fn query_sql(
        &self,
        request: &DataCloudQueryRequest,
    ) -> Result<DataCloudQueryResponse> {
        let url = format!(
            "{}/services/data/v{}/ssot/query-sql",
            self.client.instance_url(),
            self.api_version
        );
        self.client
            .post_json(&url, request)
            .await
            .map_err(Into::into)
    }

    /// Check the status of an asynchronous Data Cloud SQL query.
    ///
    /// Poll this after submitting an async query with `query_sql` (with `r#async: Some(true)`).
    /// When `status` is `"success"`, fetch results with [`query_rows`](Self::query_rows).
    #[instrument(skip(self))]
    pub async fn query_status(&self, query_id: &str) -> Result<AsyncQueryStatus> {
        let url = format!(
            "{}/services/data/v{}/ssot/query-sql/{}",
            self.client.instance_url(),
            self.api_version,
            urlencoding::encode(query_id)
        );
        self.client.get_json(&url).await.map_err(Into::into)
    }

    /// Fetch the result rows of a completed asynchronous Data Cloud SQL query.
    ///
    /// Call this after [`query_status`](Self::query_status) returns `"success"`.
    /// The response uses the same [`DataCloudQueryResponse`] shape as synchronous queries.
    #[instrument(skip(self))]
    pub async fn query_rows(&self, query_id: &str) -> Result<DataCloudQueryResponse> {
        let url = format!(
            "{}/services/data/v{}/ssot/query-sql/{}/rows",
            self.client.instance_url(),
            self.api_version,
            urlencoding::encode(query_id)
        );
        self.client.get_json(&url).await.map_err(Into::into)
    }

    // =========================================================================
    // Vector Search API  (/services/data/v{version}/ssot/search-vector)
    // =========================================================================

    /// Execute a vector (semantic) search for RAG grounding.
    ///
    /// Searches the specified vector index for chunks semantically similar to `queryText`.
    /// Use the returned chunk IDs to fetch full records via [`query_sql`](Self::query_sql)
    /// when needed.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let results = client
    ///     .vector_search(&VectorSearchRequest {
    ///         index_name: "Knowledge_Articles_Index".into(),
    ///         query_text: "How do I reset my API key?".into(),
    ///         top_k: Some(5),
    ///     })
    ///     .await?;
    ///
    /// for chunk in &results.results {
    ///     println!("score={:.3}  id={}  content={}", chunk.score, chunk.id, chunk.content);
    /// }
    /// ```
    #[instrument(skip(self, request))]
    pub async fn vector_search(
        &self,
        request: &VectorSearchRequest,
    ) -> Result<VectorSearchResponse> {
        let url = format!(
            "{}/services/data/v{}/ssot/search-vector",
            self.client.instance_url(),
            self.api_version
        );
        self.client
            .post_json(&url, request)
            .await
            .map_err(Into::into)
    }

    // =========================================================================
    // Unified Profile API  (/api/v1/profile/{dataModelName})
    // =========================================================================

    /// Look up a unified profile for a given Data Model Object (DMO).
    ///
    /// Returns the profile records matching the supplied OData-style `filters`
    /// (e.g. `"[EmailAddress__c='user@example.com']"`).  Pass `None` to retrieve
    /// all accessible records (subject to server-side limits).
    ///
    /// # Arguments
    ///
    /// * `data_model_name` — The DMO API name, e.g. `"Individual__dlm"`.
    /// * `filters` — Optional OData filter expression, e.g. `[EmailAddress__c='user@example.com']`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let profile = client
    ///     .profile("Individual__dlm", Some("[ssot__EmailAddress__c='user@example.com']"))
    ///     .await?;
    /// ```
    #[instrument(skip(self))]
    pub async fn profile(
        &self,
        data_model_name: &str,
        filters: Option<&str>,
    ) -> Result<serde_json::Value> {
        let base = format!(
            "{}/api/v1/profile/{}",
            self.client.instance_url(),
            urlencoding::encode(data_model_name)
        );
        let url = match filters {
            Some(f) => format!("{}?filters={}", base, urlencoding::encode(f)),
            None => base,
        };
        self.client.get_json(&url).await.map_err(Into::into)
    }

    // =========================================================================
    // Metadata Discovery API  (/api/v1/metadata)
    // =========================================================================

    /// Discover Data Cloud metadata entities (DMOs, fields, relationships).
    ///
    /// Optionally filter by `entity_type` (e.g. `"DataModelObject"`) to limit results
    /// to a specific category of metadata.  Use the returned field and relationship
    /// information to validate queries or build schema-aware tooling.
    ///
    /// # Arguments
    ///
    /// * `entity_type` — Optional entity type filter, e.g. `"DataModelObject"`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let meta = client.metadata(Some("DataModelObject")).await?;
    /// for obj in &meta.metadata {
    ///     println!("{}", obj.name);
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn metadata(&self, entity_type: Option<&str>) -> Result<DataCloudMetadataResponse> {
        let base = format!("{}/api/v1/metadata", self.client.instance_url());
        let url = match entity_type {
            Some(et) => format!("{}?entityType={}", base, urlencoding::encode(et)),
            None => base,
        };
        self.client.get_json(&url).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_cloud::{
        ColumnInfo, DataCloudMetadataObject, DataCloudQueryRequest, QueryMetadata,
        VectorSearchRequest,
    };

    #[test]
    fn test_data_cloud_client_creation() {
        let client =
            DataCloudClient::new("https://something.c360a.salesforce.com", "dc_token").unwrap();
        assert_eq!(client.tse_url(), "https://something.c360a.salesforce.com");
        assert_eq!(client.api_version(), "64.0");
    }

    #[test]
    fn test_api_version_override() {
        let client = DataCloudClient::new("https://tse.salesforce.com", "token")
            .unwrap()
            .with_api_version("65.0");
        assert_eq!(client.api_version(), "65.0");
    }

    #[test]
    fn test_query_request_serialization_sync() {
        let req = DataCloudQueryRequest {
            sql: "SELECT 1 FROM Individual__dlm".into(),
            page_size: Some(100),
            r#async: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["sql"], "SELECT 1 FROM Individual__dlm");
        assert_eq!(json["pageSize"], 100);
        assert!(
            json.get("async").is_none(),
            "async should be omitted when None"
        );
    }

    #[test]
    fn test_query_request_serialization_async() {
        let req = DataCloudQueryRequest {
            sql: "SELECT 1 FROM Individual__dlm".into(),
            page_size: None,
            r#async: Some(true),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["async"], true);
        assert!(
            json.get("pageSize").is_none(),
            "pageSize should be omitted when None"
        );
    }

    #[test]
    fn test_query_response_deserialization() {
        let json = serde_json::json!({
            "data": [{"ssot__Id__c": "abc", "ssot__Name__c": "Alice"}],
            "metadata": {
                "columns": [
                    {"name": "ssot__Id__c", "type": "varchar"},
                    {"name": "ssot__Name__c", "type": "varchar"}
                ]
            },
            "done": true,
            "queryId": "qry-123",
            "nextBatchId": null
        });
        let response: DataCloudQueryResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.metadata.columns.len(), 2);
        assert_eq!(response.metadata.columns[0].name, "ssot__Id__c");
        assert_eq!(response.metadata.columns[0].col_type, "varchar");
        assert!(response.done);
        assert_eq!(response.query_id, Some("qry-123".to_string()));
        assert!(response.next_batch_id.is_none());
    }

    #[test]
    fn test_column_info_deserialization() {
        let json =
            serde_json::json!({"name": "ssot__CreatedDate__c", "type": "timestamp_with_timezone"});
        let col: ColumnInfo = serde_json::from_value(json).unwrap();
        assert_eq!(col.name, "ssot__CreatedDate__c");
        assert_eq!(col.col_type, "timestamp_with_timezone");
    }

    #[test]
    fn test_query_metadata_deserialization() {
        let json = serde_json::json!({"columns": [{"name": "id", "type": "varchar"}]});
        let meta: QueryMetadata = serde_json::from_value(json).unwrap();
        assert_eq!(meta.columns.len(), 1);
    }

    #[test]
    fn test_async_query_status_deserialization() {
        let json = serde_json::json!({
            "queryId": "qry-456",
            "status": "success",
            "errorMessage": null
        });
        let status: AsyncQueryStatus = serde_json::from_value(json).unwrap();
        assert_eq!(status.query_id, "qry-456");
        assert_eq!(status.status, "success");
        assert!(status.error_message.is_none());
    }

    #[test]
    fn test_vector_search_request_serialization() {
        let req = VectorSearchRequest {
            index_name: "Knowledge_Index".into(),
            query_text: "How do I reset my password?".into(),
            top_k: Some(5),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["indexName"], "Knowledge_Index");
        assert_eq!(json["queryText"], "How do I reset my password?");
        assert_eq!(json["topK"], 5);
    }

    #[test]
    fn test_vector_search_response_deserialization() {
        let json = serde_json::json!({
            "results": [
                {"id": "rec-001", "score": 0.95, "content": "Reset steps..."},
                {"id": "rec-002", "score": 0.87, "content": "Password guide..."}
            ]
        });
        let resp: VectorSearchResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.results.len(), 2);
        assert_eq!(resp.results[0].id, "rec-001");
        assert!((resp.results[0].score - 0.95).abs() < f64::EPSILON);
        assert_eq!(resp.results[0].content, "Reset steps...");
    }

    #[test]
    fn test_metadata_response_deserialization() {
        let json = serde_json::json!({
            "metadata": [
                {
                    "name": "Individual__dlm",
                    "label": "Individual",
                    "entityType": "DataModelObject",
                    "fields": [{"name": "ssot__Id__c", "type": "varchar"}]
                }
            ]
        });
        let resp: DataCloudMetadataResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.metadata.len(), 1);
        assert_eq!(resp.metadata[0].name, "Individual__dlm");
        assert_eq!(
            resp.metadata[0].entity_type,
            Some("DataModelObject".to_string())
        );
    }

    #[test]
    fn test_metadata_object_optional_fields() {
        let json = serde_json::json!({"name": "Contact__dlm"});
        let obj: DataCloudMetadataObject = serde_json::from_value(json).unwrap();
        assert_eq!(obj.name, "Contact__dlm");
        assert!(obj.label.is_none());
        assert!(obj.entity_type.is_none());
        assert!(obj.fields.is_none());
    }

    // -------------------------------------------------------------------------
    // Wiremock integration tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_query_sql_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "data": [{"ssot__Id__c": "001", "ssot__Name__c": "Alice"}],
            "metadata": {"columns": [{"name": "ssot__Id__c", "type": "varchar"}]},
            "done": true
        });

        Mock::given(method("POST"))
            .and(path_regex(".*/ssot/query-sql$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = DataCloudClient::new(mock_server.uri(), "dc-token").unwrap();
        let result = client
            .query_sql(&DataCloudQueryRequest {
                sql: "SELECT ssot__Id__c FROM Individual__dlm".into(),
                page_size: None,
                r#async: None,
            })
            .await
            .expect("query_sql should succeed");

        assert_eq!(result.data.len(), 1);
        assert!(result.done);
    }

    #[tokio::test]
    async fn test_query_status_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "queryId": "qry-789",
            "status": "success"
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/ssot/query-sql/qry-789$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = DataCloudClient::new(mock_server.uri(), "dc-token").unwrap();
        let status = client
            .query_status("qry-789")
            .await
            .expect("query_status should succeed");

        assert_eq!(status.query_id, "qry-789");
        assert_eq!(status.status, "success");
    }

    #[tokio::test]
    async fn test_query_rows_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "data": [{"ssot__Id__c": "abc"}],
            "metadata": {"columns": [{"name": "ssot__Id__c", "type": "varchar"}]},
            "done": true
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/ssot/query-sql/.*/rows$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = DataCloudClient::new(mock_server.uri(), "dc-token").unwrap();
        let result = client
            .query_rows("qry-789")
            .await
            .expect("query_rows should succeed");

        assert_eq!(result.data.len(), 1);
    }

    #[tokio::test]
    async fn test_vector_search_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "results": [
                {"id": "ka-001", "score": 0.92, "content": "To reset, go to Settings..."}
            ]
        });

        Mock::given(method("POST"))
            .and(path_regex(".*/ssot/search-vector$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = DataCloudClient::new(mock_server.uri(), "dc-token").unwrap();
        let result = client
            .vector_search(&VectorSearchRequest {
                index_name: "KnowledgeIndex".into(),
                query_text: "How do I reset my password?".into(),
                top_k: Some(3),
            })
            .await
            .expect("vector_search should succeed");

        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].id, "ka-001");
    }

    #[tokio::test]
    async fn test_profile_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "data": [{"ssot__Id__c": "ind-001", "ssot__Name__c": "Alice"}],
            "done": true
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/api/v1/profile/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = DataCloudClient::new(mock_server.uri(), "dc-token").unwrap();
        let result = client
            .profile("Individual__dlm", Some("[ssot__EmailAddress__c='a@b.com']"))
            .await
            .expect("profile should succeed");

        assert!(result["data"].is_array());
    }

    #[tokio::test]
    async fn test_metadata_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "metadata": [
                {"name": "Individual__dlm", "entityType": "DataModelObject"}
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/api/v1/metadata.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = DataCloudClient::new(mock_server.uri(), "dc-token").unwrap();
        let result = client
            .metadata(Some("DataModelObject"))
            .await
            .expect("metadata should succeed");

        assert_eq!(result.metadata.len(), 1);
        assert_eq!(result.metadata[0].name, "Individual__dlm");
    }
}
