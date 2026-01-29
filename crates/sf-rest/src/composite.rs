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

/// A composite batch request containing multiple independent subrequests.
///
/// Unlike the standard composite request, batch subrequests are executed independently
/// and cannot reference each other's results. Available since API v34.0.
#[derive(Debug, Clone, Serialize)]
pub struct CompositeBatchRequest {
    #[serde(rename = "batchRequests")]
    pub batch_requests: Vec<CompositeBatchSubrequest>,
    #[serde(rename = "haltOnError")]
    pub halt_on_error: bool,
}

/// A single subrequest within a composite batch request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeBatchSubrequest {
    pub method: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "richInput")]
    pub rich_input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "binaryPartName")]
    pub binary_part_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "binaryPartNameAlias")]
    pub binary_part_name_alias: Option<String>,
}

/// Response from a composite batch request.
#[derive(Debug, Clone, Deserialize)]
pub struct CompositeBatchResponse {
    #[serde(rename = "hasErrors")]
    pub has_errors: bool,
    pub results: Vec<CompositeBatchSubresponse>,
}

/// Response from a single batch subrequest.
#[derive(Debug, Clone, Deserialize)]
pub struct CompositeBatchSubresponse {
    #[serde(rename = "statusCode")]
    pub status_code: u16,
    pub result: serde_json::Value,
}

/// A composite tree request for creating record hierarchies.
///
/// Allows creation of parent records with nested child records in a single request.
/// Available since API v42.0.
#[derive(Debug, Clone, Serialize)]
pub struct CompositeTreeRequest {
    pub records: Vec<CompositeTreeRecord>,
}

/// A record in a composite tree request with optional nested child records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeTreeRecord {
    pub attributes: CompositeTreeAttributes,
    #[serde(rename = "referenceId")]
    pub reference_id: String,
    #[serde(flatten)]
    pub fields: serde_json::Map<String, serde_json::Value>,
}

/// Attributes for a record in a composite tree request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeTreeAttributes {
    #[serde(rename = "type")]
    pub sobject_type: String,
}

/// Response from a composite tree request.
#[derive(Debug, Clone, Deserialize)]
pub struct CompositeTreeResponse {
    #[serde(rename = "hasErrors")]
    pub has_errors: bool,
    pub results: Vec<CompositeTreeResult>,
}

/// Result of a single record creation in a composite tree request.
#[derive(Debug, Clone, Deserialize)]
pub struct CompositeTreeResult {
    #[serde(rename = "referenceId")]
    pub reference_id: String,
    pub id: Option<String>,
    #[serde(default)]
    pub errors: Vec<CompositeTreeError>,
}

/// Error details for a failed record creation in a composite tree request.
#[derive(Debug, Clone, Deserialize)]
pub struct CompositeTreeError {
    #[serde(rename = "statusCode")]
    pub status_code: String,
    pub message: String,
    pub fields: Vec<String>,
}
