//! Core HTTP client with retry, compression, and Salesforce-specific handling.

use std::time::Duration;
use tracing::{debug, info, warn, instrument};

use crate::config::ClientConfig;
use crate::error::{Error, ErrorKind, Result};
use crate::request::{RequestBody, RequestBuilder, RequestMethod};
use crate::response::{Response, ResponseExt};
use crate::retry::RetryPolicy;

/// HTTP client for Salesforce APIs with built-in retry, compression, and error handling.
#[derive(Debug, Clone)]
pub struct SfHttpClient {
    inner: reqwest::Client,
    config: ClientConfig,
}

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
        let mut req = self
            .inner
            .request(request.method.to_reqwest(), &request.url);

        // Add bearer token
        if let Some(ref token) = request.bearer_token {
            req = req.bearer_auth(token);
        }

        // Add headers
        for (name, value) in &request.headers {
            req = req.header(name.as_str(), value.as_str());
        }

        // Add query parameters
        if !request.query_params.is_empty() {
            req = req.query(&request.query_params);
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
                RequestBody::Form(data) => req.form(data),
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

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, header};
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

        let client = SfHttpClient::new(
            ClientConfig::builder()
                .without_retry()
                .build()
        ).unwrap();

        let response = client
            .send(client.get(format!("{}/test", mock_server.uri())).bearer_auth("test-token"))
            .await
            .unwrap();

        assert!(response.is_success());
    }

    #[tokio::test]
    async fn test_salesforce_error_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/error"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!([{
                "errorCode": "INVALID_FIELD",
                "message": "No such column 'foo' on entity 'Account'",
                "fields": ["foo"]
            }])))
            .mount(&mock_server)
            .await;

        let client = SfHttpClient::new(
            ClientConfig::builder()
                .without_retry()
                .build()
        ).unwrap();

        let result = client
            .send(client.get(format!("{}/error", mock_server.uri())).bearer_auth("token"))
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
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("Retry-After", "30")
            )
            .mount(&mock_server)
            .await;

        let client = SfHttpClient::new(
            ClientConfig::builder()
                .without_retry()
                .build()
        ).unwrap();

        let result = client
            .send(client.get(format!("{}/limited", mock_server.uri())).bearer_auth("token"))
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
                        .with_initial_delay(Duration::from_millis(10))
                )
                .build()
        ).unwrap();

        let response = client
            .send(client.get(format!("{}/retry", mock_server.uri())).bearer_auth("token"))
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

        let client = SfHttpClient::new(
            ClientConfig::builder()
                .without_retry()
                .build()
        ).unwrap();

        let response = client
            .send(
                client
                    .get(format!("{}/cached", mock_server.uri()))
                    .bearer_auth("token")
                    .if_none_match("\"abc123\"")
            )
            .await
            .unwrap();

        assert!(response.is_not_modified());
    }
}
