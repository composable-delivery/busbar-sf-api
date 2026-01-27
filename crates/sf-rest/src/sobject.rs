//! SObject CRUD operations.

use serde::{Deserialize, Serialize};

/// Result of a create operation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateResult {
    pub id: String,
    pub success: bool,
    #[serde(default)]
    pub errors: Vec<SalesforceError>,
}

/// Result of an update operation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateResult {
    pub success: bool,
    #[serde(default)]
    pub errors: Vec<SalesforceError>,
}

/// Result of a delete operation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteResult {
    pub success: bool,
    #[serde(default)]
    pub errors: Vec<SalesforceError>,
}

/// Result of an upsert operation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpsertResult {
    pub id: String,
    pub success: bool,
    pub created: bool,
    #[serde(default)]
    pub errors: Vec<SalesforceError>,
}

/// Salesforce error in operation results.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SalesforceError {
    #[serde(rename = "statusCode")]
    pub status_code: String,
    pub message: String,
    #[serde(default)]
    pub fields: Vec<String>,
}
