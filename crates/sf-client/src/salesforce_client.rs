//! High-level Salesforce client with typed HTTP methods.
//!
//! This module provides `SalesforceClient`, which combines credentials with
//! an HTTP client and provides typed JSON methods for API interactions.
//!
//! ## Security
//!
//! - Access tokens are redacted in Debug output
//! - Sensitive parameters are skipped in tracing spans

use serde::{de::DeserializeOwned, Serialize};
use tracing::instrument;

use crate::client::SfHttpClient;
use crate::config::ClientConfig;
use crate::error::{Error, ErrorKind, Result};
use crate::request::RequestBuilder;
use crate::DEFAULT_API_VERSION;

/// High-level Salesforce API client.
///
/// This client combines credentials with HTTP infrastructure and provides
/// typed methods for making API requests. It's designed to be used by
/// higher-level API-specific crates (sf-rest, sf-bulk, etc.).
///
/// ## Security
///
/// The access token is redacted in Debug output to prevent accidental
/// exposure in logs.
///
/// # Example
///
/// ```rust,ignore
/// use sf_client::SalesforceClient;
/// use sf_auth::SalesforceCredentials;
///
/// let creds = SalesforceCredentials::from_env()?;
/// let client = SalesforceClient::new(creds)?;
///
/// // GET with typed response
/// let user: UserInfo = client.get_json("/services/oauth2/userinfo").await?;
///
/// // POST with body and typed response
/// let result: CreateResult = client
///     .post_json("/services/data/v62.0/sobjects/Account", &account)
///     .await?;
/// ```
#[derive(Clone)]
pub struct SalesforceClient {
    http: SfHttpClient,
    instance_url: String,
    access_token: String,
    api_version: String,
}

impl std::fmt::Debug for SalesforceClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SalesforceClient")
            .field("instance_url", &self.instance_url)
            .field("access_token", &"[REDACTED]")
            .field("api_version", &self.api_version)
            .finish_non_exhaustive()
    }
}

impl SalesforceClient {
    /// Create a new Salesforce client with the given instance URL and access token.
    pub fn new(
        instance_url: impl Into<String>,
        access_token: impl Into<String>,
    ) -> Result<Self> {
        Self::with_config(instance_url, access_token, ClientConfig::default())
    }

    /// Create a new Salesforce client with custom configuration.
    pub fn with_config(
        instance_url: impl Into<String>,
        access_token: impl Into<String>,
        config: ClientConfig,
    ) -> Result<Self> {
        let http = SfHttpClient::new(config)?;
        Ok(Self {
            http,
            instance_url: instance_url.into().trim_end_matches('/').to_string(),
            access_token: access_token.into(),
            api_version: DEFAULT_API_VERSION.to_string(),
        })
    }

    /// Set the API version (e.g., "62.0").
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }

    /// Get the instance URL.
    pub fn instance_url(&self) -> &str {
        &self.instance_url
    }

    /// Get the access token.
    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    /// Get the API version.
    pub fn api_version(&self) -> &str {
        &self.api_version
    }

    /// Build the full URL for a path.
    ///
    /// If the path starts with `/`, it's appended to the instance URL.
    /// Otherwise, it's assumed to be a full URL.
    pub fn url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else if path.starts_with('/') {
            format!("{}{}", self.instance_url, path)
        } else {
            format!("{}/{}", self.instance_url, path)
        }
    }

    /// Build the REST API URL for a path.
    ///
    /// Example: `rest_url("sobjects/Account")` -> `/services/data/v62.0/sobjects/Account`
    pub fn rest_url(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!(
            "{}/services/data/v{}/{}",
            self.instance_url, self.api_version, path
        )
    }

    /// Build the Tooling API URL for a path.
    ///
    /// Example: `tooling_url("sobjects/ApexClass")` -> `/services/data/v62.0/tooling/sobjects/ApexClass`
    pub fn tooling_url(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!(
            "{}/services/data/v{}/tooling/{}",
            self.instance_url, self.api_version, path
        )
    }

    /// Build the Metadata API URL (SOAP endpoint).
    pub fn metadata_url(&self) -> String {
        format!(
            "{}/services/Soap/m/{}",
            self.instance_url, self.api_version
        )
    }

    /// Build the Bulk API 2.0 URL for a path.
    pub fn bulk_url(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!(
            "{}/services/data/v{}/jobs/{}",
            self.instance_url, self.api_version, path
        )
    }

    // =========================================================================
    // Base HTTP Methods (with authentication)
    // =========================================================================

    /// Create a GET request builder with authentication.
    pub fn get(&self, url: &str) -> RequestBuilder {
        self.http.get(url).bearer_auth(&self.access_token)
    }

    /// Create a POST request builder with authentication.
    pub fn post(&self, url: &str) -> RequestBuilder {
        self.http.post(url).bearer_auth(&self.access_token)
    }

    /// Create a PATCH request builder with authentication.
    pub fn patch(&self, url: &str) -> RequestBuilder {
        self.http.patch(url).bearer_auth(&self.access_token)
    }

    /// Create a PUT request builder with authentication.
    pub fn put(&self, url: &str) -> RequestBuilder {
        self.http.put(url).bearer_auth(&self.access_token)
    }

    /// Create a DELETE request builder with authentication.
    pub fn delete(&self, url: &str) -> RequestBuilder {
        self.http.delete(url).bearer_auth(&self.access_token)
    }

    /// Execute a request and return the raw response.
    pub async fn execute(&self, request: RequestBuilder) -> Result<crate::Response> {
        self.http.execute(request).await
    }

    // =========================================================================
    // Typed JSON Methods
    // =========================================================================

    /// GET request with JSON response deserialization.
    #[instrument(skip(self), fields(url = %url))]
    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let full_url = self.url(url);
        let request = self.get(&full_url);
        let response = self.http.execute(request).await?;
        response.json().await
    }

    /// GET request to REST API with JSON response.
    pub async fn rest_get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.get_json(&self.rest_url(path)).await
    }

    /// GET request to Tooling API with JSON response.
    pub async fn tooling_get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.get_json(&self.tooling_url(path)).await
    }

    /// POST request with JSON body and response.
    #[instrument(skip(self, body), fields(url = %url))]
    pub async fn post_json<T: DeserializeOwned, B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<T> {
        let full_url = self.url(url);
        let request = self.post(&full_url).json(body)?;
        let response = self.http.execute(request).await?;
        response.json().await
    }

    /// POST request to REST API with JSON body and response.
    pub async fn rest_post<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.post_json(&self.rest_url(path), body).await
    }

    /// POST request to Tooling API with JSON body and response.
    pub async fn tooling_post<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.post_json(&self.tooling_url(path), body).await
    }

    /// PATCH request with JSON body and optional response.
    #[instrument(skip(self, body), fields(url = %url))]
    pub async fn patch_json<B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<()> {
        let full_url = self.url(url);
        let request = self.patch(&full_url).json(body)?;
        let response = self.http.execute(request).await?;

        // PATCH typically returns 204 No Content on success
        if response.status() == 204 || response.is_success() {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Http {
                status: response.status(),
                message: "PATCH request failed".to_string(),
            }))
        }
    }

    /// PATCH request to REST API with JSON body.
    pub async fn rest_patch<B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<()> {
        self.patch_json(&self.rest_url(path), body).await
    }

    /// DELETE request.
    #[instrument(skip(self), fields(url = %url))]
    pub async fn delete_request(&self, url: &str) -> Result<()> {
        let full_url = self.url(url);
        let request = self.delete(&full_url);
        let response = self.http.execute(request).await?;

        // DELETE typically returns 204 No Content on success
        if response.status() == 204 || response.is_success() {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Http {
                status: response.status(),
                message: "DELETE request failed".to_string(),
            }))
        }
    }

    /// DELETE request to REST API.
    pub async fn rest_delete(&self, path: &str) -> Result<()> {
        self.delete_request(&self.rest_url(path)).await
    }

    // =========================================================================
    // Conditional Request Methods (ETags, If-Modified-Since)
    // =========================================================================

    /// GET request with If-None-Match header (ETag caching).
    /// Returns None if the resource hasn't changed (304 response).
    pub async fn get_json_if_changed<T: DeserializeOwned>(
        &self,
        url: &str,
        etag: &str,
    ) -> Result<Option<(T, Option<String>)>> {
        let full_url = self.url(url);
        let request = self.get(&full_url).if_none_match(etag);
        let response = self.http.execute(request).await?;

        if response.is_not_modified() {
            return Ok(None);
        }

        let new_etag = response.etag().map(|s| s.to_string());
        let data: T = response.json().await?;
        Ok(Some((data, new_etag)))
    }

    /// GET request with If-Modified-Since header.
    /// Returns None if the resource hasn't changed (304 response).
    pub async fn get_json_if_modified<T: DeserializeOwned>(
        &self,
        url: &str,
        since: &str,
    ) -> Result<Option<(T, Option<String>)>> {
        let full_url = self.url(url);
        let request = self.get(&full_url).if_modified_since(since);
        let response = self.http.execute(request).await?;

        if response.is_not_modified() {
            return Ok(None);
        }

        let last_modified = response.last_modified().map(|s| s.to_string());
        let data: T = response.json().await?;
        Ok(Some((data, last_modified)))
    }

    // =========================================================================
    // Query Helpers
    // =========================================================================

    /// Execute a SOQL query via REST API.
    pub async fn query<T: DeserializeOwned>(&self, soql: &str) -> Result<QueryResult<T>> {
        let encoded = urlencoding::encode(soql);
        let url = format!(
            "{}/services/data/v{}/query?q={}",
            self.instance_url, self.api_version, encoded
        );
        self.get_json(&url).await
    }

    /// Execute a SOQL query via Tooling API.
    pub async fn tooling_query<T: DeserializeOwned>(&self, soql: &str) -> Result<QueryResult<T>> {
        let encoded = urlencoding::encode(soql);
        let url = format!(
            "{}/services/data/v{}/tooling/query?q={}",
            self.instance_url, self.api_version, encoded
        );
        self.get_json(&url).await
    }

    /// Execute a SOQL query and automatically fetch all pages.
    pub async fn query_all<T: DeserializeOwned + Clone>(&self, soql: &str) -> Result<Vec<T>> {
        let mut all_records = Vec::new();
        let mut result: QueryResult<T> = self.query(soql).await?;

        all_records.extend(result.records);

        while let Some(ref next_url) = result.next_records_url {
            result = self.get_json(next_url).await?;
            all_records.extend(result.records);
        }

        Ok(all_records)
    }

    /// Execute a Tooling API query and automatically fetch all pages.
    pub async fn tooling_query_all<T: DeserializeOwned + Clone>(&self, soql: &str) -> Result<Vec<T>> {
        let mut all_records = Vec::new();
        let mut result: QueryResult<T> = self.tooling_query(soql).await?;

        all_records.extend(result.records);

        while let Some(ref next_url) = result.next_records_url {
            result = self.get_json(next_url).await?;
            all_records.extend(result.records);
        }

        Ok(all_records)
    }
}

/// Result of a SOQL query.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct QueryResult<T> {
    /// Total number of records matching the query.
    #[serde(rename = "totalSize")]
    pub total_size: u64,

    /// Whether all records are returned (no more pages).
    pub done: bool,

    /// URL to fetch next batch of results.
    #[serde(rename = "nextRecordsUrl")]
    pub next_records_url: Option<String>,

    /// The records.
    pub records: Vec<T>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_building() {
        let client = SalesforceClient::new(
            "https://na1.salesforce.com",
            "token123",
        ).unwrap();

        // Absolute paths
        assert_eq!(
            client.url("/services/oauth2/userinfo"),
            "https://na1.salesforce.com/services/oauth2/userinfo"
        );

        // Relative paths
        assert_eq!(
            client.url("services/oauth2/userinfo"),
            "https://na1.salesforce.com/services/oauth2/userinfo"
        );

        // Full URLs
        assert_eq!(
            client.url("https://other.com/path"),
            "https://other.com/path"
        );

        // REST API URL
        assert_eq!(
            client.rest_url("sobjects/Account"),
            "https://na1.salesforce.com/services/data/v62.0/sobjects/Account"
        );

        // Tooling API URL
        assert_eq!(
            client.tooling_url("sobjects/ApexClass"),
            "https://na1.salesforce.com/services/data/v62.0/tooling/sobjects/ApexClass"
        );

        // Bulk API URL
        assert_eq!(
            client.bulk_url("ingest"),
            "https://na1.salesforce.com/services/data/v62.0/jobs/ingest"
        );
    }

    #[test]
    fn test_api_version() {
        let client = SalesforceClient::new("https://na1.salesforce.com", "token")
            .unwrap()
            .with_api_version("60.0");

        assert_eq!(client.api_version(), "60.0");
        assert_eq!(
            client.rest_url("limits"),
            "https://na1.salesforce.com/services/data/v60.0/limits"
        );
    }

    #[test]
    fn test_trailing_slash_handling() {
        let client = SalesforceClient::new(
            "https://na1.salesforce.com/",  // Trailing slash
            "token",
        ).unwrap();

        assert_eq!(client.instance_url(), "https://na1.salesforce.com");
        assert_eq!(
            client.rest_url("limits"),
            "https://na1.salesforce.com/services/data/v62.0/limits"
        );
    }
}
