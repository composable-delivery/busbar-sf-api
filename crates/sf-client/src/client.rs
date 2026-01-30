//! Core HTTP client with retry, compression, and Salesforce-specific handling.

use std::time::Duration;

#[cfg(feature = "native")]
use tracing::{debug, info, warn};

use crate::config::ClientConfig;
use crate::error::{Error, ErrorKind, Result};
use crate::request::{RequestBody, RequestBuilder, RequestMethod};
use crate::response::{Response, ResponseExt};

#[cfg(feature = "native")]
use crate::retry::RetryPolicy;

// Native backend
#[cfg(feature = "native")]
use tracing::instrument;

// WASM backend using extism
#[cfg(feature = "wasm")]
use extism_pdk;

/// HTTP client for Salesforce APIs with built-in retry, compression, and error handling.
#[cfg(feature = "native")]
#[derive(Debug, Clone)]
pub struct SfHttpClient {
    inner: reqwest::Client,
    config: ClientConfig,
}

/// HTTP client for Salesforce APIs with built-in retry, compression, and error handling.
#[cfg(feature = "wasm")]
#[derive(Debug, Clone)]
pub struct SfHttpClient {
    config: ClientConfig,
}

#[cfg(feature = "native")]
impl SfHttpClient {
    /// Create a new HTTP client with default configuration.
    pub fn new(config: ClientConfig) -> Result<Self> {
        let mut builder = reqwest::Client::builder()
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .pool_idle_timeout(config.pool_idle_timeout)
            .pool_max_idle_per_host(config.pool_max_idle_per_host)
            .user_agent(&config.user_agent);

        // Configure compression
        if config.compression.accept_compressed {
            builder = builder.gzip(true).deflate(true);
        } else {
            builder = builder.gzip(false).deflate(false);
        }

        let inner = builder
            .build()
            .map_err(|e| Error::with_source(ErrorKind::Config(e.to_string()), e))?;

        Ok(Self { inner, config })
    }

    /// Create a new HTTP client with default configuration.
    pub fn default_client() -> Result<Self> {
        Self::new(ClientConfig::default())
    }

    /// Get the client configuration.
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Create a GET request builder.
    pub fn get(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(RequestMethod::Get, url)
    }

    /// Create a POST request builder.
    pub fn post(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(RequestMethod::Post, url)
    }

    /// Create a PATCH request builder.
    pub fn patch(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(RequestMethod::Patch, url)
    }

    /// Create a PUT request builder.
    pub fn put(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(RequestMethod::Put, url)
    }

    /// Create a DELETE request builder.
    pub fn delete(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(RequestMethod::Delete, url)
    }

    /// Create a HEAD request builder.
    pub fn head(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(RequestMethod::Head, url)
    }

    /// Execute a request with automatic retry handling.
    #[instrument(skip(self, request), fields(method = ?request.method, url = %request.url))]
    pub async fn execute(&self, request: RequestBuilder) -> Result<Response> {
        let mut retry_policy = self
            .config
            .retry
            .as_ref()
            .map(|c| RetryPolicy::new(c.clone()));

        loop {
            let result = self.execute_once(&request).await;

            match result {
                Ok(response) => {
                    // Check for Salesforce API errors
                    return response.check_salesforce_error().await;
                }
                Err(err) if err.is_retryable() => {
                    if let Some(ref mut policy) = retry_policy {
                        if let Some(delay) = policy.next_delay(err.retry_after()) {
                            warn!(
                                attempt = policy.attempt(),
                                delay_ms = delay.as_millis(),
                                error = %err,
                                "Request failed, retrying"
                            );
                            tokio::time::sleep(delay).await;
                            continue;
                        }

                        // Exhausted retries
                        return Err(Error::new(ErrorKind::RetriesExhausted {
                            attempts: policy.attempt(),
                        }));
                    }

                    // No retry policy configured
                    return Err(err);
                }
                Err(err) => {
                    // Non-retryable error
                    return Err(err);
                }
            }
        }
    }

    /// Execute a single request without retry logic.
    async fn execute_once(&self, request: &RequestBuilder) -> Result<Response> {
        // Build URL with query parameters
        let url = if !request.query_params.is_empty() {
            let mut url = url::Url::parse(&request.url)
                .map_err(|e| Error::with_source(ErrorKind::InvalidUrl(request.url.clone()), e))?;

            // Add query parameters
            for (key, value) in &request.query_params {
                url.query_pairs_mut().append_pair(key, value);
            }

            url.to_string()
        } else {
            request.url.clone()
        };

        let mut req = self.inner.request(request.method.to_reqwest(), &url);

        // Add bearer token
        if let Some(ref token) = request.bearer_token {
            req = req.bearer_auth(token);
        }

        // Add headers
        for (name, value) in &request.headers {
            req = req.header(name.as_str(), value.as_str());
        }

        // Add conditional headers
        if let Some(ref etag) = request.if_match {
            req = req.header("If-Match", etag.as_str());
        }
        if let Some(ref etag) = request.if_none_match {
            req = req.header("If-None-Match", etag.as_str());
        }
        if let Some(ref ts) = request.if_modified_since {
            req = req.header("If-Modified-Since", ts.as_str());
        }
        if let Some(ref ts) = request.if_unmodified_since {
            req = req.header("If-Unmodified-Since", ts.as_str());
        }

        // Add compression headers if enabled
        if self.config.compression.accept_compressed {
            req = req.header("Accept-Encoding", "gzip, deflate");
        }

        // Add body
        if let Some(ref body) = request.body {
            req = match body {
                RequestBody::Json(value) => req.json(value),
                RequestBody::Text(text) => req.body(text.clone()),
                RequestBody::Bytes(bytes) => req.body(bytes.clone()),
                RequestBody::Form(data) => {
                    // Serialize form data to URL-encoded format
                    let encoded = serde_urlencoded::to_string(data).map_err(|e| {
                        Error::with_source(
                            ErrorKind::Serialization("Failed to encode form data".to_string()),
                            e,
                        )
                    })?;
                    req.header("Content-Type", "application/x-www-form-urlencoded")
                        .body(encoded)
                }
            };
        }

        if self.config.enable_tracing {
            debug!(
                method = ?request.method,
                url = %request.url,
                "Sending request"
            );
        }

        let response = req.send().await?;

        if self.config.enable_tracing {
            let status = response.status().as_u16();
            let content_length = response.content_length();

            if response.status().is_success() {
                debug!(status, content_length, "Response received");
            } else {
                info!(status, content_length, "Non-success response");
            }
        }

        let status = response.status().as_u16();

        // Check for rate limiting
        if status == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(Duration::from_secs);

            return Err(Error::new(ErrorKind::RateLimited { retry_after }));
        }

        // Check for retryable server errors (500, 502, 503, 504)
        if matches!(status, 500 | 502 | 503 | 504) {
            return Err(Error::new(ErrorKind::Http {
                status,
                message: format!("Server error: {}", status),
            }));
        }

        Ok(Response::new(response))
    }

    /// Execute a request and return the response, checking for errors.
    /// This is a convenience method that combines execute and error checking.
    pub async fn send(&self, request: RequestBuilder) -> Result<Response> {
        self.execute(request).await
    }

    /// Execute a request and deserialize the JSON response.
    pub async fn send_json<T: serde::de::DeserializeOwned>(
        &self,
        request: RequestBuilder,
    ) -> Result<T> {
        let response = self.execute(request).await?;
        response.json().await
    }
}

// WASM implementation using Extism PDK
#[cfg(feature = "wasm")]
impl SfHttpClient {
    /// Create a new HTTP client with the given configuration.
    ///
    /// Note: Retry policies are not supported in WASM. The configuration must have
    /// `retry: None`, otherwise this will return an error. Use `ClientConfig::builder()
    /// .without_retry().build()` to create a valid WASM configuration.
    pub fn new(config: ClientConfig) -> Result<Self> {
        // Validate that retry is not configured for WASM
        if config.retry.is_some() {
            return Err(Error::new(ErrorKind::Config(
                "Retry policies are not supported in WASM environments. \
                 Please configure ClientConfig with retry: None using \
                 ClientConfig::builder().without_retry().build()."
                    .to_string(),
            )));
        }

        Ok(Self { config })
    }

    /// Create a new HTTP client with default configuration (no retry for WASM).
    pub fn default_client() -> Result<Self> {
        // For WASM, default to no retry since retry is not supported
        Self::new(ClientConfig::builder().without_retry().build())
    }

    /// Get the client configuration.
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Create a GET request builder.
    pub fn get(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(RequestMethod::Get, url)
    }

    /// Create a POST request builder.
    pub fn post(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(RequestMethod::Post, url)
    }

    /// Create a PATCH request builder.
    pub fn patch(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(RequestMethod::Patch, url)
    }

    /// Create a PUT request builder.
    pub fn put(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(RequestMethod::Put, url)
    }

    /// Create a DELETE request builder.
    pub fn delete(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(RequestMethod::Delete, url)
    }

    /// Create a HEAD request builder.
    pub fn head(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(RequestMethod::Head, url)
    }

    /// Execute a request (synchronous for WASM).
    ///
    /// Note: Retry logic is not supported in WASM. The client creation validates that
    /// retry is not configured, so this method executes requests without retry logic.
    pub fn execute(&self, request: RequestBuilder) -> Result<Response> {
        let result = self.execute_once(&request);

        match result {
            Ok(response) => {
                // Check for Salesforce API errors
                response.check_salesforce_error()
            }
            Err(err) => Err(err),
        }
    }

    /// Execute a single request without retry logic (synchronous for WASM).
    fn execute_once(&self, request: &RequestBuilder) -> Result<Response> {
        // Build URL with query parameters
        let url = if !request.query_params.is_empty() {
            let mut url = url::Url::parse(&request.url)
                .map_err(|e| Error::with_source(ErrorKind::InvalidUrl(request.url.clone()), e))?;

            // Add query parameters
            for (key, value) in &request.query_params {
                url.query_pairs_mut().append_pair(key, value);
            }

            url.to_string()
        } else {
            request.url.clone()
        };

        // Build headers
        let mut headers = std::collections::BTreeMap::new();

        // Add bearer token
        if let Some(ref token) = request.bearer_token {
            headers.insert("Authorization".to_string(), format!("Bearer {}", token));
        }

        // Add custom headers
        for (name, value) in &request.headers {
            headers.insert(name.clone(), value.clone());
        }

        // Add conditional headers
        if let Some(ref etag) = request.if_match {
            headers.insert("If-Match".to_string(), etag.clone());
        }
        if let Some(ref etag) = request.if_none_match {
            headers.insert("If-None-Match".to_string(), etag.clone());
        }
        if let Some(ref ts) = request.if_modified_since {
            headers.insert("If-Modified-Since".to_string(), ts.clone());
        }
        if let Some(ref ts) = request.if_unmodified_since {
            headers.insert("If-Unmodified-Since".to_string(), ts.clone());
        }

        // Add compression headers if enabled
        if self.config.compression.accept_compressed {
            headers.insert("Accept-Encoding".to_string(), "gzip, deflate".to_string());
        }

        // Build body
        let body = if let Some(ref body) = request.body {
            match body {
                RequestBody::Json(value) => {
                    headers.insert("Content-Type".to_string(), "application/json".to_string());
                    serde_json::to_vec(value)?
                }
                RequestBody::Text(text) => text.as_bytes().to_vec(),
                RequestBody::Bytes(bytes) => bytes.to_vec(),
                RequestBody::Form(data) => {
                    let encoded = serde_urlencoded::to_string(data).map_err(|e| {
                        Error::with_source(
                            ErrorKind::Serialization("Failed to encode form data".to_string()),
                            e,
                        )
                    })?;
                    headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
                    encoded.into_bytes()
                }
            }
        } else {
            Vec::new()
        };

        // Note: Tracing/logging in WASM is handled by the host environment
        // The enable_tracing config has no effect in WASM builds

        // Make the HTTP request using Extism PDK
        let http_request = extism_manifest::HttpRequest {
            url,
            headers,
            method: Some(request.method.as_str().to_string()),
        };

        let http_response = extism_pdk::http::request::<Vec<u8>>(&http_request, Some(body))?;

        // Note: Tracing/logging in WASM is handled by the host environment
        // The enable_tracing config has no effect in WASM builds

        let status = http_response.status_code();

        // Extract headers from response
        let response_headers = http_response.headers().clone();

        // Check for rate limiting
        if status == 429 {
            let retry_after = response_headers
                .get("retry-after")
                .and_then(|v| v.parse::<u64>().ok())
                .map(Duration::from_secs);

            return Err(Error::new(ErrorKind::RateLimited { retry_after }));
        }

        // Check for retryable server errors (500, 502, 503, 504)
        if matches!(status, 500 | 502 | 503 | 504) {
            return Err(Error::new(ErrorKind::Http {
                status,
                message: format!("Server error: {}", status),
            }));
        }

        let extism_response = crate::response::ExtismResponse::new(
            status,
            response_headers,
            http_response.body(),
        );

        Ok(Response::new_extism(extism_response))
    }

    /// Execute a request and return the response, checking for errors.
    pub fn send(&self, request: RequestBuilder) -> Result<Response> {
        self.execute(request)
    }

    /// Execute a request and deserialize the JSON response.
    pub fn send_json<T: serde::de::DeserializeOwned>(
        &self,
        request: RequestBuilder,
    ) -> Result<T> {
        let response = self.execute(request)?;
        response.json()
    }
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_client_creation() {
        let client = SfHttpClient::default_client().unwrap();
        assert!(client.config().compression.enabled);
    }

    #[tokio::test]
    async fn test_successful_request() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true
            })))
            .mount(&mock_server)
            .await;

        let client = SfHttpClient::new(ClientConfig::builder().without_retry().build()).unwrap();

        let response = client
            .send(
                client
                    .get(format!("{}/test", mock_server.uri()))
                    .bearer_auth("test-token"),
            )
            .await
            .unwrap();

        assert!(response.is_success());
    }

    #[tokio::test]
    async fn test_salesforce_error_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/error"))
            .respond_with(
                ResponseTemplate::new(400).set_body_json(serde_json::json!([{
                    "errorCode": "INVALID_FIELD",
                    "message": "No such column 'foo' on entity 'Account'",
                    "fields": ["foo"]
                }])),
            )
            .mount(&mock_server)
            .await;

        let client = SfHttpClient::new(ClientConfig::builder().without_retry().build()).unwrap();

        let result = client
            .send(
                client
                    .get(format!("{}/error", mock_server.uri()))
                    .bearer_auth("token"),
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err.kind, ErrorKind::SalesforceApi { .. }));
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/limited"))
            .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "30"))
            .mount(&mock_server)
            .await;

        let client = SfHttpClient::new(ClientConfig::builder().without_retry().build()).unwrap();

        let result = client
            .send(
                client
                    .get(format!("{}/limited", mock_server.uri()))
                    .bearer_auth("token"),
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_rate_limited());
        assert_eq!(err.retry_after(), Some(Duration::from_secs(30)));
    }

    #[tokio::test]
    async fn test_retry_on_503() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let mock_server = MockServer::start().await;
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        // Use a respond_with_fn to control responses based on call count
        Mock::given(method("GET"))
            .and(path("/retry"))
            .respond_with(move |_: &wiremock::Request| {
                let count = call_count_clone.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    ResponseTemplate::new(503)
                } else {
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({
                        "success": true
                    }))
                }
            })
            .mount(&mock_server)
            .await;

        let client = SfHttpClient::new(
            ClientConfig::builder()
                .with_retry(
                    crate::RetryConfig::default()
                        .with_max_attempts(3)
                        .with_initial_delay(Duration::from_millis(10)),
                )
                .build(),
        )
        .unwrap();

        let response = client
            .send(
                client
                    .get(format!("{}/retry", mock_server.uri()))
                    .bearer_auth("token"),
            )
            .await
            .unwrap();

        assert!(response.is_success());
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_conditional_request_304() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/cached"))
            .and(header("If-None-Match", "\"abc123\""))
            .respond_with(ResponseTemplate::new(304))
            .mount(&mock_server)
            .await;

        let client = SfHttpClient::new(ClientConfig::builder().without_retry().build()).unwrap();

        let response = client
            .send(
                client
                    .get(format!("{}/cached", mock_server.uri()))
                    .bearer_auth("token")
                    .if_none_match("\"abc123\""),
            )
            .await
            .unwrap();

        assert!(response.is_not_modified());
    }
}

#[cfg(all(test, feature = "wasm"))]
mod wasm_tests {
    use super::*;

    #[test]
    fn test_wasm_default_client_creation() {
        // default_client should succeed and have no retry configured
        let client = SfHttpClient::default_client().unwrap();
        assert!(client.config().retry.is_none());
        assert!(client.config().compression.enabled);
    }

    #[test]
    fn test_wasm_client_creation_without_retry() {
        let client = SfHttpClient::new(ClientConfig::builder().without_retry().build()).unwrap();
        assert!(client.config().retry.is_none());
    }

    #[test]
    fn test_wasm_client_creation_with_retry_fails() {
        // Creating a client with retry should fail in WASM
        let config = ClientConfig::builder()
            .with_retry(crate::RetryConfig::default())
            .build();
        let result = SfHttpClient::new(config);
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err.kind, ErrorKind::Config(_)));
    }

    #[test]
    fn test_wasm_request_builder_creation() {
        let client = SfHttpClient::default_client().unwrap();
        let request = client
            .get("https://example.com/api/test")
            .bearer_auth("test-token")
            .header("X-Custom", "value");

        assert_eq!(request.url, "https://example.com/api/test");
        assert_eq!(request.bearer_token, Some("test-token".to_string()));
        assert!(request.headers.contains_key("X-Custom"));
    }

    #[test]
    fn test_wasm_all_http_methods() {
        let client = SfHttpClient::default_client().unwrap();
        
        // Test all HTTP method builders
        let get_req = client.get("https://example.com/get");
        assert_eq!(get_req.method, RequestMethod::Get);
        
        let post_req = client.post("https://example.com/post");
        assert_eq!(post_req.method, RequestMethod::Post);
        
        let put_req = client.put("https://example.com/put");
        assert_eq!(put_req.method, RequestMethod::Put);
        
        let patch_req = client.patch("https://example.com/patch");
        assert_eq!(patch_req.method, RequestMethod::Patch);
        
        let delete_req = client.delete("https://example.com/delete");
        assert_eq!(delete_req.method, RequestMethod::Delete);
        
        let head_req = client.head("https://example.com/head");
        assert_eq!(head_req.method, RequestMethod::Head);
    }

    #[test]
    fn test_wasm_request_with_query_params() {
        let client = SfHttpClient::default_client().unwrap();
        let request = client
            .get("https://example.com/api/test")
            .query("key1", "value1")
            .query("key2", "value2")
            .bearer_auth("test-token");

        assert_eq!(request.query_params.len(), 2);
        assert_eq!(request.query_params[0], ("key1".to_string(), "value1".to_string()));
        assert_eq!(request.query_params[1], ("key2".to_string(), "value2".to_string()));
    }

    #[test]
    fn test_wasm_request_with_json_body() {
        let client = SfHttpClient::default_client().unwrap();
        let body_data = serde_json::json!({
            "name": "Test Account",
            "industry": "Technology"
        });
        
        let request = client
            .post("https://example.com/api/sobjects/Account")
            .json(&body_data)
            .unwrap()
            .bearer_auth("test-token");

        assert!(matches!(request.body, Some(RequestBody::Json(_))));
    }

    #[test]
    fn test_wasm_request_with_conditional_headers() {
        let client = SfHttpClient::default_client().unwrap();
        
        let request = client
            .get("https://example.com/api/test")
            .if_none_match("\"abc123\"")
            .if_modified_since("Wed, 21 Oct 2015 07:28:00 GMT")
            .bearer_auth("test-token");

        assert_eq!(request.if_none_match, Some("\"abc123\"".to_string()));
        assert_eq!(request.if_modified_since, Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string()));
    }

    #[test]
    fn test_wasm_request_with_custom_headers() {
        let client = SfHttpClient::default_client().unwrap();
        
        let request = client
            .get("https://example.com/api/test")
            .header("X-Custom-Header", "custom-value")
            .header("X-Request-ID", "12345")
            .bearer_auth("test-token");

        assert_eq!(request.headers.len(), 2);
        assert_eq!(request.headers.get("X-Custom-Header"), Some(&"custom-value".to_string()));
        assert_eq!(request.headers.get("X-Request-ID"), Some(&"12345".to_string()));
    }

    #[test]
    fn test_wasm_config_builder() {
        use std::time::Duration;
        
        let config = ClientConfig::builder()
            .without_retry()
            .with_timeout(Duration::from_secs(30))
            .with_user_agent("Custom-Agent/1.0")
            .build();
        
        let client = SfHttpClient::new(config).unwrap();
        assert_eq!(client.config().timeout, Duration::from_secs(30));
        assert_eq!(client.config().user_agent, "Custom-Agent/1.0");
    }

    #[test]
    fn test_wasm_compression_config() {
        let config = ClientConfig::builder()
            .without_retry()
            .with_compression(true)
            .build();
        
        let client = SfHttpClient::new(config).unwrap();
        assert!(client.config().compression.enabled);
    }
}
