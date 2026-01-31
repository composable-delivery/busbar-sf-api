//! Metadata API client.

use busbar_sf_auth::{Credentials, SalesforceCredentials};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

use crate::error::Result;
use crate::types::DEFAULT_API_VERSION;

mod deploy;
mod describe;
mod list;
mod retrieve;
mod xml_helpers;

/// SOAP Action header name.
static SOAP_ACTION_HEADER: HeaderName = HeaderName::from_static("soapaction");

/// Salesforce Metadata API client.
#[derive(Debug)]
pub struct MetadataClient {
    instance_url: String,
    access_token: String,
    api_version: String,
    http_client: reqwest::Client,
}

impl MetadataClient {
    /// Create a new Metadata API client from credentials.
    pub fn new(credentials: &SalesforceCredentials) -> Result<Self> {
        Ok(Self {
            instance_url: credentials.instance_url().to_string(),
            access_token: credentials.access_token().to_string(),
            api_version: DEFAULT_API_VERSION.to_string(),
            http_client: reqwest::Client::new(),
        })
    }

    /// Create a new Metadata API client from instance URL and access token.
    pub fn from_parts(instance_url: impl Into<String>, access_token: impl Into<String>) -> Self {
        Self {
            instance_url: instance_url.into(),
            access_token: access_token.into(),
            api_version: DEFAULT_API_VERSION.to_string(),
            http_client: reqwest::Client::new(),
        }
    }

    /// Set the API version.
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }

    /// Set a custom HTTP client.
    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = client;
        self
    }

    /// Get the Metadata API SOAP endpoint URL.
    pub(crate) fn metadata_url(&self) -> String {
        format!("{}/services/Soap/m/{}", self.instance_url, self.api_version)
    }

    /// Build common headers for SOAP requests.
    pub(crate) fn build_headers(&self, soap_action: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("text/xml;charset=UTF-8"),
        );
        headers.insert(
            SOAP_ACTION_HEADER.clone(),
            HeaderValue::from_str(soap_action).unwrap(),
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.access_token)).unwrap(),
        );
        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = MetadataClient::from_parts("https://test.salesforce.com", "token123");
        assert_eq!(client.api_version, DEFAULT_API_VERSION);
    }

    #[test]
    fn test_client_with_version() {
        let client = MetadataClient::from_parts("https://test.salesforce.com", "token123")
            .with_api_version("58.0");
        assert_eq!(client.api_version, "58.0");
    }

    #[test]
    fn test_metadata_url_construction() {
        let client = MetadataClient::from_parts("https://na1.salesforce.com", "token")
            .with_api_version("62.0");
        assert_eq!(
            client.metadata_url(),
            "https://na1.salesforce.com/services/Soap/m/62.0"
        );
    }

    #[test]
    fn test_build_headers() {
        let client = MetadataClient::from_parts("https://na1.salesforce.com", "token123");
        let headers = client.build_headers("deploy");

        assert_eq!(
            headers.get("content-type").unwrap(),
            "text/xml;charset=UTF-8"
        );
        assert_eq!(headers.get("soapaction").unwrap(), "deploy");
        assert_eq!(headers.get("authorization").unwrap(), "Bearer token123");
    }
}
