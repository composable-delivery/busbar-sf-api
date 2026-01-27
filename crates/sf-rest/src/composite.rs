//! Composite API operations.

use serde::{Deserialize, Serialize};

/// A composite request containing multiple subrequests.
#[derive(Debug, Clone, Serialize)]
pub struct CompositeRequest {
    #[serde(rename = "allOrNone")]
    pub all_or_none: bool,
    #[serde(rename = "collateSubrequests")]
    pub collate_subrequests: bool,
    #[serde(rename = "compositeRequest")]
    pub subrequests: Vec<CompositeSubrequest>,
}

/// A single subrequest within a composite request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeSubrequest {
    pub method: String,
    pub url: String,
    #[serde(rename = "referenceId")]
    pub reference_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

/// Response from a composite request.
#[derive(Debug, Clone, Deserialize)]
pub struct CompositeResponse {
    #[serde(rename = "compositeResponse")]
    pub responses: Vec<CompositeSubresponse>,
}

/// Response from a single subrequest.
#[derive(Debug, Clone, Deserialize)]
pub struct CompositeSubresponse {
    pub body: serde_json::Value,
    #[serde(rename = "httpHeaders")]
    pub http_headers: serde_json::Value,
    #[serde(rename = "httpStatusCode")]
    pub http_status_code: u16,
    #[serde(rename = "referenceId")]
    pub reference_id: String,
}
