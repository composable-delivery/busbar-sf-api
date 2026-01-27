//! SObject Collections for batch operations.

use crate::sobject::SalesforceError;
use serde::{Deserialize, Serialize};

/// Request for SObject Collections operations.
#[derive(Debug, Clone, Serialize)]
pub struct CollectionRequest {
    #[serde(rename = "allOrNone")]
    pub all_or_none: bool,
    pub records: Vec<serde_json::Value>,
}

/// Result of a collection operation.
#[derive(Debug, Clone, Deserialize)]
pub struct CollectionResult {
    pub id: Option<String>,
    pub success: bool,
    #[serde(default)]
    pub errors: Vec<SalesforceError>,
    pub created: Option<bool>,
}
