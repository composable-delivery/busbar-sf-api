//! HTTP response handling with Salesforce-specific extensions.

use serde::de::DeserializeOwned;
use std::time::Duration;

use crate::error::{Error, ErrorKind, Result};

/// Internal response wrapper that can hold either backend.
#[derive(Debug)]
enum InnerResponse {
    #[cfg(feature = "native")]
    Native(reqwest::Response),
    #[cfg(feature = "wasm")]
    Wasm(ExtismResponse),
}

/// Custom response type for WASM backend using Extism.
#[cfg(feature = "wasm")]
#[derive(Debug)]
pub struct ExtismResponse {
    status: u16,
    headers: std::collections::HashMap<String, String>,
    body: Vec<u8>,
}

#[cfg(feature = "wasm")]
impl ExtismResponse {
    pub fn new(status: u16, headers: std::collections::HashMap<String, String>, body: Vec<u8>) -> Self {
        // Normalize header names to lowercase for case-insensitive lookups
        let normalized_headers = headers
            .into_iter()
            .map(|(k, v)| (k.to_lowercase(), v))
            .collect();
        
        Self {
            status,
            headers: normalized_headers,
            body,
        }
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        // Lookup with lowercase key for case-insensitive matching
        self.headers.get(&name.to_lowercase()).map(|s| s.as_str())
    }

    pub fn body(self) -> Vec<u8> {
        self.body
    }
}

/// Wrapper around HTTP response with additional functionality.
#[derive(Debug)]
pub struct Response {
    inner: InnerResponse,
}

impl Response {
    /// Create a new Response from a reqwest::Response (native).
    #[cfg(feature = "native")]
    pub(crate) fn new(inner: reqwest::Response) -> Self {
        Self {
            inner: InnerResponse::Native(inner),
        }
    }

    /// Create a new Response from an ExtismResponse (wasm).
    #[cfg(feature = "wasm")]
    pub(crate) fn new_extism(inner: ExtismResponse) -> Self {
        Self {
            inner: InnerResponse::Wasm(inner),
        }
    }

    /// Get the HTTP status code.
    pub fn status(&self) -> u16 {
        match &self.inner {
            #[cfg(feature = "native")]
            InnerResponse::Native(resp) => resp.status().as_u16(),
            #[cfg(feature = "wasm")]
            InnerResponse::Wasm(resp) => resp.status(),
        }
    }

    /// Returns true if the response status is successful (2xx).
    pub fn is_success(&self) -> bool {
        let status = self.status();
        (200..300).contains(&status)
    }

    /// Returns true if this is a 304 Not Modified response.
    pub fn is_not_modified(&self) -> bool {
        self.status() == 304
    }

    /// Get a header value.
    pub fn header(&self, name: &str) -> Option<&str> {
        match &self.inner {
            #[cfg(feature = "native")]
            InnerResponse::Native(resp) => resp.headers().get(name)?.to_str().ok(),
            #[cfg(feature = "wasm")]
            InnerResponse::Wasm(resp) => resp.header(name),
        }
    }

    /// Get the ETag header value.
    pub fn etag(&self) -> Option<&str> {
        self.header("etag")
    }

    /// Get the Last-Modified header value.
    pub fn last_modified(&self) -> Option<&str> {
        self.header("last-modified")
    }

    /// Get the Retry-After header as a Duration.
    pub fn retry_after(&self) -> Option<Duration> {
        let value = self.header("retry-after")?;

        // Try parsing as seconds first
        if let Ok(seconds) = value.parse::<u64>() {
            return Some(Duration::from_secs(seconds));
        }

        // Try parsing as HTTP date (simplified - just extract seconds from now)
        // In practice, most Salesforce Retry-After headers are in seconds
        None
    }

    /// Get the Sforce-Locator header (used for Bulk API pagination).
    pub fn sforce_locator(&self) -> Option<&str> {
        self.header("sforce-locator")
    }

    /// Get the Content-Type header.
    pub fn content_type(&self) -> Option<&str> {
        self.header("content-type")
    }

    /// Get the response body as text.
    #[cfg(feature = "native")]
    pub async fn text(self) -> Result<String> {
        match self.inner {
            InnerResponse::Native(resp) => resp.text().await.map_err(Into::into),
        }
    }

    /// Get the response body as text (synchronous for WASM).
    #[cfg(feature = "wasm")]
    pub fn text(self) -> Result<String> {
        match self.inner {
            InnerResponse::Wasm(resp) => {
                String::from_utf8(resp.body()).map_err(|e| {
                    Error::with_source(ErrorKind::Other("Failed to decode response as UTF-8".to_string()), e)
                })
            }
        }
    }

    /// Get the response body as bytes.
    #[cfg(feature = "native")]
    pub async fn bytes(self) -> Result<bytes::Bytes> {
        match self.inner {
            InnerResponse::Native(resp) => resp.bytes().await.map_err(Into::into),
        }
    }

    /// Get the response body as bytes (synchronous for WASM).
    #[cfg(feature = "wasm")]
    pub fn bytes(self) -> Result<bytes::Bytes> {
        match self.inner {
            InnerResponse::Wasm(resp) => Ok(bytes::Bytes::from(resp.body())),
        }
    }

    /// Deserialize the response body as JSON.
    #[cfg(feature = "native")]
    pub async fn json<T: DeserializeOwned>(self) -> Result<T> {
        match self.inner {
            InnerResponse::Native(resp) => resp.json().await.map_err(Into::into),
        }
    }

    /// Deserialize the response body as JSON (synchronous for WASM).
    #[cfg(feature = "wasm")]
    pub fn json<T: DeserializeOwned>(self) -> Result<T> {
        match self.inner {
            InnerResponse::Wasm(resp) => {
                serde_json::from_slice(&resp.body()).map_err(Into::into)
            }
        }
    }

    /// Get access to the inner reqwest::Response (native only).
    #[cfg(feature = "native")]
    pub fn into_inner(self) -> reqwest::Response {
        match self.inner {
            InnerResponse::Native(resp) => resp,
        }
    }

    /// Get API usage limits from response headers.
    pub fn api_usage(&self) -> Option<ApiUsage> {
        // Salesforce returns usage in Sforce-Limit-Info header
        // Format: "api-usage=25/15000"
        let info = self.header("sforce-limit-info")?;

        for part in info.split(',') {
            let part = part.trim();
            if part.starts_with("api-usage=") {
                let usage = part.trim_start_matches("api-usage=");
                let parts: Vec<&str> = usage.split('/').collect();
                if parts.len() == 2 {
                    let used = parts[0].parse().ok()?;
                    let limit = parts[1].parse().ok()?;
                    return Some(ApiUsage { used, limit });
                }
            }
        }

        None
    }
}

/// API usage information from response headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApiUsage {
    /// Number of API calls used.
    pub used: u64,
    /// Total API call limit.
    pub limit: u64,
}

impl ApiUsage {
    /// Get the remaining API calls.
    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.used)
    }

    /// Get the usage percentage.
    pub fn percentage(&self) -> f64 {
        if self.limit == 0 {
            100.0
        } else {
            (self.used as f64 / self.limit as f64) * 100.0
        }
    }

    /// Returns true if API usage is above the given percentage threshold.
    pub fn is_above_threshold(&self, threshold_percent: f64) -> bool {
        self.percentage() >= threshold_percent
    }
}

/// Extension trait for processing Salesforce API responses.
pub trait ResponseExt {
    /// Check for Salesforce API errors and convert to appropriate error type.
    #[cfg(feature = "native")]
    fn check_salesforce_error(self) -> impl std::future::Future<Output = Result<Response>> + Send;
    
    /// Check for Salesforce API errors and convert to appropriate error type (sync for WASM).
    #[cfg(feature = "wasm")]
    fn check_salesforce_error(self) -> Result<Response>;
}

/// Parse error response body and convert to appropriate error kind.
/// This is shared logic between native and WASM implementations.
fn parse_error_response(status: u16, body: &str) -> Error {
    // Check for rate limiting
    if status == 429 {
        return Error::new(ErrorKind::RateLimited { retry_after: None });
    }

    // Try to parse as Salesforce error JSON (array format)
    if let Ok(errors) = serde_json::from_str::<Vec<SalesforceErrorResponse>>(body) {
        if let Some(err) = errors.into_iter().next() {
            return Error::new(ErrorKind::SalesforceApi {
                error_code: err.error_code,
                message: sanitize_error_message(&err.message),
                fields: err.fields.unwrap_or_default(),
            });
        }
    }

    // Try to parse as single error object
    if let Ok(err) = serde_json::from_str::<SalesforceErrorResponse>(body) {
        return Error::new(ErrorKind::SalesforceApi {
            error_code: err.error_code,
            message: sanitize_error_message(&err.message),
            fields: err.fields.unwrap_or_default(),
        });
    }

    // Map status codes to error kinds - use sanitized messages to avoid
    // potentially exposing sensitive data from response bodies
    let sanitized = sanitize_error_message(body);
    let kind = match status {
        401 => ErrorKind::Authentication(sanitized),
        403 => ErrorKind::Authorization(sanitized),
        404 => ErrorKind::NotFound(sanitized),
        412 => ErrorKind::PreconditionFailed(sanitized),
        _ => ErrorKind::Http {
            status,
            message: sanitized,
        },
    };

    Error::new(kind)
}

#[cfg(feature = "native")]
impl ResponseExt for Response {
    async fn check_salesforce_error(self) -> Result<Response> {
        let status = self.status();

        if self.is_success() || self.is_not_modified() {
            return Ok(self);
        }

        // Try to parse Salesforce error response
        let body = self.text().await.unwrap_or_default();
        Err(parse_error_response(status, &body))
    }
}

#[cfg(feature = "wasm")]
impl ResponseExt for Response {
    fn check_salesforce_error(self) -> Result<Response> {
        let status = self.status();

        if self.is_success() || self.is_not_modified() {
            return Ok(self);
        }

        // Try to parse Salesforce error response
        let body = self.text().unwrap_or_default();
        Err(parse_error_response(status, &body))
    }
}

/// Sanitize an error message to prevent exposing sensitive data.
///
/// This function:
/// - Truncates messages longer than 500 characters
/// - Removes potential tokens (anything that looks like an access token)
/// - Removes potential session IDs
fn sanitize_error_message(message: &str) -> String {
    const MAX_LENGTH: usize = 500;

    let mut sanitized = message.to_string();

    // Remove anything that looks like a Bearer token or access token
    // Salesforce tokens typically start with "00D" and are 100+ chars
    let token_pattern = regex_lite::Regex::new(r"00[A-Za-z0-9]{13,}[!][A-Za-z0-9_.]+").unwrap();
    sanitized = token_pattern
        .replace_all(&sanitized, "[REDACTED_TOKEN]")
        .to_string();

    // Remove session IDs (typically 24 chars alphanumeric)
    let session_pattern = regex_lite::Regex::new(r"sid=[A-Za-z0-9]{20,}").unwrap();
    sanitized = session_pattern
        .replace_all(&sanitized, "sid=[REDACTED]")
        .to_string();

    // Truncate if too long
    if sanitized.len() > MAX_LENGTH {
        sanitized.truncate(MAX_LENGTH);
        sanitized.push_str("...[truncated]");
    }

    sanitized
}

/// Salesforce API error response format.
#[derive(Debug, serde::Deserialize)]
struct SalesforceErrorResponse {
    #[serde(alias = "errorCode")]
    error_code: String,
    message: String,
    fields: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_usage() {
        let usage = ApiUsage {
            used: 100,
            limit: 1000,
        };

        assert_eq!(usage.remaining(), 900);
        assert!((usage.percentage() - 10.0).abs() < 0.001);
        assert!(!usage.is_above_threshold(50.0));
        assert!(usage.is_above_threshold(5.0));
    }

    #[test]
    fn test_api_usage_edge_cases() {
        let usage = ApiUsage {
            used: 1000,
            limit: 1000,
        };
        assert_eq!(usage.remaining(), 0);
        assert!((usage.percentage() - 100.0).abs() < 0.001);

        let usage = ApiUsage { used: 0, limit: 0 };
        assert_eq!(usage.remaining(), 0);
        assert!((usage.percentage() - 100.0).abs() < 0.001);
    }
}
