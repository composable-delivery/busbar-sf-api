//! HTTP request building with Salesforce-specific headers.

use std::collections::HashMap;
use bytes::Bytes;
use serde::Serialize;

use crate::error::Result;

/// HTTP request method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestMethod {
    Get,
    Post,
    Patch,
    Put,
    Delete,
    Head,
}

impl RequestMethod {
    /// Convert to reqwest::Method.
    pub fn to_reqwest(&self) -> reqwest::Method {
        match self {
            RequestMethod::Get => reqwest::Method::GET,
            RequestMethod::Post => reqwest::Method::POST,
            RequestMethod::Patch => reqwest::Method::PATCH,
            RequestMethod::Put => reqwest::Method::PUT,
            RequestMethod::Delete => reqwest::Method::DELETE,
            RequestMethod::Head => reqwest::Method::HEAD,
        }
    }
}

/// Builder for HTTP requests with Salesforce-specific options.
#[derive(Debug)]
pub struct RequestBuilder {
    pub(crate) method: RequestMethod,
    pub(crate) url: String,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) query_params: Vec<(String, String)>,
    pub(crate) body: Option<RequestBody>,
    pub(crate) bearer_token: Option<String>,
    /// ETag for If-Match header (optimistic concurrency).
    pub(crate) if_match: Option<String>,
    /// ETag for If-None-Match header (conditional GET).
    pub(crate) if_none_match: Option<String>,
    /// Timestamp for If-Modified-Since header.
    pub(crate) if_modified_since: Option<String>,
    /// Timestamp for If-Unmodified-Since header.
    pub(crate) if_unmodified_since: Option<String>,
}

/// Request body content.
#[derive(Debug)]
pub enum RequestBody {
    Json(serde_json::Value),
    Text(String),
    Bytes(Bytes),
    Form(HashMap<String, String>),
}

impl RequestBuilder {
    /// Create a new request builder.
    pub fn new(method: RequestMethod, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            headers: HashMap::new(),
            query_params: Vec::new(),
            body: None,
            bearer_token: None,
            if_match: None,
            if_none_match: None,
            if_modified_since: None,
            if_unmodified_since: None,
        }
    }

    /// Set the bearer token for authentication.
    pub fn bearer_auth(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(token.into());
        self
    }

    /// Add a header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Add a query parameter.
    pub fn query(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.query_params.push((name.into(), value.into()));
        self
    }

    /// Set JSON body.
    pub fn json<T: Serialize>(mut self, body: &T) -> Result<Self> {
        let value = serde_json::to_value(body)?;
        self.body = Some(RequestBody::Json(value));
        self
            .headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        Ok(self)
    }

    /// Set raw JSON body.
    pub fn json_value(mut self, body: serde_json::Value) -> Self {
        self.body = Some(RequestBody::Json(body));
        self.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        self
    }

    /// Set text body.
    pub fn text(mut self, body: impl Into<String>) -> Self {
        self.body = Some(RequestBody::Text(body.into()));
        self.headers
            .insert("Content-Type".to_string(), "text/plain".to_string());
        self
    }

    /// Set bytes body.
    pub fn bytes(mut self, body: impl Into<Bytes>) -> Self {
        self.body = Some(RequestBody::Bytes(body.into()));
        self
    }

    /// Set form body.
    pub fn form(mut self, data: HashMap<String, String>) -> Self {
        self.body = Some(RequestBody::Form(data));
        self.headers.insert(
            "Content-Type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        );
        self
    }

    /// Set CSV body (for Bulk API).
    pub fn csv(mut self, data: impl Into<String>) -> Self {
        self.body = Some(RequestBody::Text(data.into()));
        self.headers
            .insert("Content-Type".to_string(), "text/csv".to_string());
        self
    }

    /// Set XML body (for SOAP/Metadata API).
    pub fn xml(mut self, data: impl Into<String>) -> Self {
        self.body = Some(RequestBody::Text(data.into()));
        self.headers
            .insert("Content-Type".to_string(), "text/xml; charset=UTF-8".to_string());
        self
    }

    /// Set If-Match header for optimistic concurrency.
    /// The request will fail with 412 if the ETag doesn't match.
    pub fn if_match(mut self, etag: impl Into<String>) -> Self {
        self.if_match = Some(etag.into());
        self
    }

    /// Set If-None-Match header for conditional GET.
    /// Returns 304 Not Modified if the ETag matches.
    pub fn if_none_match(mut self, etag: impl Into<String>) -> Self {
        self.if_none_match = Some(etag.into());
        self
    }

    /// Set If-Modified-Since header.
    /// Returns 304 Not Modified if not modified since the given timestamp.
    pub fn if_modified_since(mut self, timestamp: impl Into<String>) -> Self {
        self.if_modified_since = Some(timestamp.into());
        self
    }

    /// Set If-Unmodified-Since header.
    /// The request will fail with 412 if modified since the given timestamp.
    pub fn if_unmodified_since(mut self, timestamp: impl Into<String>) -> Self {
        self.if_unmodified_since = Some(timestamp.into());
        self
    }

    /// Accept gzip compression for the response.
    pub fn accept_gzip(mut self) -> Self {
        self.headers
            .insert("Accept-Encoding".to_string(), "gzip".to_string());
        self
    }

    /// Set Sforce-Call-Options header (for partner API, etc.).
    pub fn sforce_call_options(mut self, options: impl Into<String>) -> Self {
        self.headers
            .insert("Sforce-Call-Options".to_string(), options.into());
        self
    }

    /// Set Sforce-Query-Options header (for query batch size).
    pub fn sforce_query_options(mut self, batch_size: u32) -> Self {
        self.headers.insert(
            "Sforce-Query-Options".to_string(),
            format!("batchSize={}", batch_size),
        );
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_builder() {
        let req = RequestBuilder::new(RequestMethod::Get, "https://example.com/api")
            .bearer_auth("token123")
            .header("X-Custom", "value")
            .query("q", "SELECT Id FROM Account");

        assert_eq!(req.method, RequestMethod::Get);
        assert_eq!(req.url, "https://example.com/api");
        assert_eq!(req.bearer_token, Some("token123".to_string()));
        assert_eq!(req.headers.get("X-Custom"), Some(&"value".to_string()));
        assert_eq!(req.query_params.len(), 1);
    }

    #[test]
    fn test_conditional_headers() {
        let req = RequestBuilder::new(RequestMethod::Get, "https://example.com")
            .if_none_match("\"abc123\"")
            .if_modified_since("Wed, 21 Oct 2015 07:28:00 GMT");

        assert_eq!(req.if_none_match, Some("\"abc123\"".to_string()));
        assert_eq!(
            req.if_modified_since,
            Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string())
        );
    }

    #[test]
    fn test_json_body() {
        let data = serde_json::json!({"Name": "Test Account"});
        let req = RequestBuilder::new(RequestMethod::Post, "https://example.com")
            .json(&data)
            .unwrap();

        assert!(matches!(req.body, Some(RequestBody::Json(_))));
        assert_eq!(
            req.headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );
    }

    #[test]
    fn test_csv_body() {
        let req = RequestBuilder::new(RequestMethod::Put, "https://example.com")
            .csv("Id,Name\n001xx,Test");

        assert!(matches!(req.body, Some(RequestBody::Text(_))));
        assert_eq!(
            req.headers.get("Content-Type"),
            Some(&"text/csv".to_string())
        );
    }
}
