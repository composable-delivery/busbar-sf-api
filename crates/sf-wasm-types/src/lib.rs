//! # busbar-sf-wasm-types
//!
//! Shared ABI types for the busbar-sf WASM bridge.
//!
//! This crate defines the request/response types that cross the WASM boundary
//! between the host (sf-bridge) and guest (sf-guest-sdk). These types are
//! serialized as JSON at the ABI boundary.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────┐
//! │  WASM Guest (sf-guest-sdk)               │
//! │  Uses these types to call host functions  │
//! └──────────────┬───────────────────────────┘
//!               │ JSON serialized
//!               ▼
//! ┌──────────────────────────────────────────┐
//! │  Host (sf-bridge)                        │
//! │  Uses these types to parse requests and  │
//! │  serialize responses                     │
//! └──────────────────────────────────────────┘
//! ```
//!
//! ## Design Principles
//!
//! - **Pure data**: No I/O, no async, no platform-specific code
//! - **Serde only**: Just `serde` and `serde_json` dependencies
//! - **Compiles everywhere**: Native, wasm32-unknown-unknown, wasm32-wasi

use serde::{Deserialize, Serialize};

// =============================================================================
// Bridge Error
// =============================================================================

/// Error returned by bridge host functions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeError {
    /// Machine-readable error code (e.g., "INVALID_SOQL", "AUTH_FAILED").
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// Optional field-level errors.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for BridgeError {}

/// Result type for bridge operations.
///
/// Serialized as JSON: `{"ok": <data>}` or `{"err": <error>}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeResult<T> {
    Ok(T),
    Err(BridgeError),
}

impl<T> BridgeResult<T> {
    pub fn ok(value: T) -> Self {
        BridgeResult::Ok(value)
    }

    pub fn err(code: impl Into<String>, message: impl Into<String>) -> Self {
        BridgeResult::Err(BridgeError {
            code: code.into(),
            message: message.into(),
            fields: vec![],
        })
    }

    pub fn into_result(self) -> Result<T, BridgeError> {
        match self {
            BridgeResult::Ok(v) => Ok(v),
            BridgeResult::Err(e) => Err(e),
        }
    }
}

impl<T> From<BridgeResult<T>> for Result<T, BridgeError> {
    fn from(r: BridgeResult<T>) -> Self {
        r.into_result()
    }
}

// =============================================================================
// Salesforce API Error (matches Salesforce error envelope)
// =============================================================================

/// Salesforce error in operation results.
///
/// This mirrors the error format returned by Salesforce APIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesforceApiError {
    #[serde(rename = "statusCode")]
    pub status_code: String,
    pub message: String,
    #[serde(default)]
    pub fields: Vec<String>,
}

// =============================================================================
// Query
// =============================================================================

/// Request for SOQL query operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    /// SOQL query string.
    pub soql: String,
    /// If true, include deleted/archived records (queryAll endpoint).
    #[serde(default)]
    pub include_deleted: bool,
}

/// Response from a SOQL query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    /// Total number of records matching the query.
    pub total_size: u64,
    /// Whether all records have been returned.
    pub done: bool,
    /// The records in this page.
    pub records: Vec<serde_json::Value>,
    /// URL for the next page (if `done` is false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_records_url: Option<String>,
}

// =============================================================================
// CRUD Operations
// =============================================================================

/// Request to create a record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRequest {
    /// SObject type (e.g., "Account").
    pub sobject: String,
    /// Record fields as JSON.
    pub record: serde_json::Value,
}

/// Response from a create operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateResponse {
    pub id: String,
    pub success: bool,
    #[serde(default)]
    pub errors: Vec<SalesforceApiError>,
}

/// Request to read a record by ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRequest {
    /// SObject type (e.g., "Account").
    pub sobject: String,
    /// Record ID (15 or 18 character Salesforce ID).
    pub id: String,
    /// Optional list of fields to retrieve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
}

/// Request to update a record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRequest {
    /// SObject type.
    pub sobject: String,
    /// Record ID.
    pub id: String,
    /// Fields to update.
    pub record: serde_json::Value,
}

/// Request to delete a record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequest {
    /// SObject type.
    pub sobject: String,
    /// Record ID.
    pub id: String,
}

/// Request to upsert a record using an external ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertRequest {
    /// SObject type.
    pub sobject: String,
    /// External ID field name.
    pub external_id_field: String,
    /// External ID value.
    pub external_id_value: String,
    /// Record fields.
    pub record: serde_json::Value,
}

/// Response from an upsert operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertResponse {
    pub id: String,
    pub success: bool,
    pub created: bool,
    #[serde(default)]
    pub errors: Vec<SalesforceApiError>,
}

// =============================================================================
// Describe
// =============================================================================

/// Request to describe a specific SObject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeSObjectRequest {
    /// SObject type (e.g., "Account").
    pub sobject: String,
}

// =============================================================================
// Search (SOSL)
// =============================================================================

/// Request for a SOSL search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// SOSL search string.
    pub sosl: String,
}

/// Response from a SOSL search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub search_records: Vec<serde_json::Value>,
}

// =============================================================================
// Composite API
// =============================================================================

/// Request for a composite API call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeRequest {
    /// If true, all subrequests are rolled back on any failure.
    pub all_or_none: bool,
    /// The subrequests to execute.
    pub subrequests: Vec<CompositeSubrequest>,
}

/// A single subrequest in a composite call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeSubrequest {
    /// HTTP method (GET, POST, PATCH, DELETE).
    pub method: String,
    /// Relative URL for the subrequest.
    pub url: String,
    /// Reference ID for cross-referencing between subrequests.
    pub reference_id: String,
    /// Optional request body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

/// Response from a composite API call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeResponse {
    pub responses: Vec<CompositeSubresponse>,
}

/// Response from a single composite subrequest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeSubresponse {
    pub body: serde_json::Value,
    pub http_status_code: u16,
    pub reference_id: String,
}

// =============================================================================
// Collections (Batch CRUD)
// =============================================================================

/// Request to create multiple records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMultipleRequest {
    /// SObject type.
    pub sobject: String,
    /// Records to create (up to 200).
    pub records: Vec<serde_json::Value>,
    /// If true, all records fail if any single record fails.
    pub all_or_none: bool,
}

/// Request to delete multiple records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMultipleRequest {
    /// Record IDs to delete (up to 200).
    pub ids: Vec<String>,
    /// If true, all deletes fail if any single delete fails.
    pub all_or_none: bool,
}

/// Result of a single record in a collection operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionResult {
    pub id: Option<String>,
    pub success: bool,
    #[serde(default)]
    pub errors: Vec<SalesforceApiError>,
    pub created: Option<bool>,
}

// =============================================================================
// Limits
// =============================================================================

/// Response from the limits endpoint.
///
/// Returned as a JSON object where keys are limit names and values
/// contain `Max` and `Remaining` fields.
pub type LimitsResponse = serde_json::Value;

// =============================================================================
// Host Function Names (constants for ABI contract)
// =============================================================================

/// Host function name constants.
///
/// These are the names used to register/import host functions across
/// the WASM boundary. Both sf-bridge and sf-guest-sdk use these.
pub mod host_fn_names {
    pub const QUERY: &str = "sf_query";
    pub const CREATE: &str = "sf_create";
    pub const GET: &str = "sf_get";
    pub const UPDATE: &str = "sf_update";
    pub const DELETE: &str = "sf_delete";
    pub const UPSERT: &str = "sf_upsert";
    pub const DESCRIBE_GLOBAL: &str = "sf_describe_global";
    pub const DESCRIBE_SOBJECT: &str = "sf_describe_sobject";
    pub const SEARCH: &str = "sf_search";
    pub const COMPOSITE: &str = "sf_composite";
    pub const CREATE_MULTIPLE: &str = "sf_create_multiple";
    pub const DELETE_MULTIPLE: &str = "sf_delete_multiple";
    pub const LIMITS: &str = "sf_limits";
}

/// The Extism namespace used for all bridge host functions.
pub const BRIDGE_NAMESPACE: &str = "busbar";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_result_ok_serialization() {
        let result: BridgeResult<String> = BridgeResult::ok("hello".to_string());
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"ok\""));
        assert!(json.contains("\"hello\""));

        let deserialized: BridgeResult<String> = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, BridgeResult::Ok(s) if s == "hello"));
    }

    #[test]
    fn test_bridge_result_err_serialization() {
        let result: BridgeResult<String> =
            BridgeResult::err("INVALID_SOQL", "unexpected token at position 5");
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"err\""));
        assert!(json.contains("INVALID_SOQL"));

        let deserialized: BridgeResult<String> = serde_json::from_str(&json).unwrap();
        match deserialized {
            BridgeResult::Err(e) => {
                assert_eq!(e.code, "INVALID_SOQL");
                assert_eq!(e.message, "unexpected token at position 5");
            }
            _ => panic!("expected Err"),
        }
    }

    #[test]
    fn test_bridge_result_into_result() {
        let ok: BridgeResult<u32> = BridgeResult::ok(42);
        assert_eq!(ok.into_result().unwrap(), 42);

        let err: BridgeResult<u32> = BridgeResult::err("FAIL", "failed");
        assert!(err.into_result().is_err());
    }

    #[test]
    fn test_bridge_error_display() {
        let err = BridgeError {
            code: "AUTH_FAILED".to_string(),
            message: "Invalid token".to_string(),
            fields: vec![],
        };
        assert_eq!(format!("{err}"), "AUTH_FAILED: Invalid token");
    }

    #[test]
    fn test_query_request_serialization() {
        let req = QueryRequest {
            soql: "SELECT Id, Name FROM Account".to_string(),
            include_deleted: false,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["soql"], "SELECT Id, Name FROM Account");
        assert_eq!(json["include_deleted"], false);
    }

    #[test]
    fn test_query_response_serialization() {
        let resp = QueryResponse {
            total_size: 2,
            done: true,
            records: vec![
                serde_json::json!({"Id": "001xx1", "Name": "Acme"}),
                serde_json::json!({"Id": "001xx2", "Name": "Widget Co"}),
            ],
            next_records_url: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["total_size"], 2);
        assert_eq!(json["done"], true);
        assert_eq!(json["records"].as_array().unwrap().len(), 2);
        assert!(json.get("next_records_url").is_none());
    }

    #[test]
    fn test_query_response_with_pagination() {
        let resp = QueryResponse {
            total_size: 5000,
            done: false,
            records: vec![serde_json::json!({"Id": "001xx1"})],
            next_records_url: Some("/services/data/v62.0/query/01gxx-2000".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(!json["done"].as_bool().unwrap());
        assert!(json["next_records_url"].is_string());
    }

    #[test]
    fn test_create_request_roundtrip() {
        let req = CreateRequest {
            sobject: "Account".to_string(),
            record: serde_json::json!({"Name": "Test Corp", "Industry": "Technology"}),
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: CreateRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.sobject, "Account");
        assert_eq!(deserialized.record["Name"], "Test Corp");
    }

    #[test]
    fn test_create_response_success() {
        let json = serde_json::json!({
            "id": "001xx000003DgAAAS",
            "success": true,
            "errors": []
        });
        let resp: CreateResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.id, "001xx000003DgAAAS");
        assert!(resp.success);
        assert!(resp.errors.is_empty());
    }

    #[test]
    fn test_create_response_failure() {
        let json = serde_json::json!({
            "id": "",
            "success": false,
            "errors": [{
                "statusCode": "REQUIRED_FIELD_MISSING",
                "message": "Required fields are missing: [Name]",
                "fields": ["Name"]
            }]
        });
        let resp: CreateResponse = serde_json::from_value(json).unwrap();
        assert!(!resp.success);
        assert_eq!(resp.errors.len(), 1);
        assert_eq!(resp.errors[0].status_code, "REQUIRED_FIELD_MISSING");
    }

    #[test]
    fn test_get_request_with_fields() {
        let req = GetRequest {
            sobject: "Contact".to_string(),
            id: "003xx000004TmiQAAS".to_string(),
            fields: Some(vec![
                "Id".to_string(),
                "Name".to_string(),
                "Email".to_string(),
            ]),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["fields"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_get_request_without_fields() {
        let req = GetRequest {
            sobject: "Account".to_string(),
            id: "001xx000003DgAAAS".to_string(),
            fields: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("fields").is_none());
    }

    #[test]
    fn test_upsert_request_roundtrip() {
        let req = UpsertRequest {
            sobject: "Account".to_string(),
            external_id_field: "External_Id__c".to_string(),
            external_id_value: "EXT-001".to_string(),
            record: serde_json::json!({"Name": "Upserted Corp"}),
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: UpsertRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.external_id_field, "External_Id__c");
        assert_eq!(deserialized.external_id_value, "EXT-001");
    }

    #[test]
    fn test_upsert_response_created() {
        let resp = UpsertResponse {
            id: "001xx000003DgAAAS".to_string(),
            success: true,
            created: true,
            errors: vec![],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["created"].as_bool().unwrap());
    }

    #[test]
    fn test_composite_request_serialization() {
        let req = CompositeRequest {
            all_or_none: true,
            subrequests: vec![
                CompositeSubrequest {
                    method: "POST".to_string(),
                    url: "/services/data/v62.0/sobjects/Account".to_string(),
                    reference_id: "NewAccount".to_string(),
                    body: Some(serde_json::json!({"Name": "Test"})),
                },
                CompositeSubrequest {
                    method: "GET".to_string(),
                    url: "/services/data/v62.0/sobjects/Account/@{NewAccount.id}".to_string(),
                    reference_id: "GetAccount".to_string(),
                    body: None,
                },
            ],
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json["all_or_none"].as_bool().unwrap());
        assert_eq!(json["subrequests"].as_array().unwrap().len(), 2);
        assert!(json["subrequests"][1].get("body").is_none());
    }

    #[test]
    fn test_composite_response_deserialization() {
        let json = serde_json::json!({
            "responses": [
                {
                    "body": {"id": "001xx", "success": true},
                    "http_status_code": 201,
                    "reference_id": "NewAccount"
                },
                {
                    "body": {"Id": "001xx", "Name": "Test"},
                    "http_status_code": 200,
                    "reference_id": "GetAccount"
                }
            ]
        });
        let resp: CompositeResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.responses.len(), 2);
        assert_eq!(resp.responses[0].http_status_code, 201);
    }

    #[test]
    fn test_create_multiple_request() {
        let req = CreateMultipleRequest {
            sobject: "Account".to_string(),
            records: vec![
                serde_json::json!({"Name": "Acme"}),
                serde_json::json!({"Name": "Widget Co"}),
            ],
            all_or_none: false,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["records"].as_array().unwrap().len(), 2);
        assert!(!json["all_or_none"].as_bool().unwrap());
    }

    #[test]
    fn test_collection_result_success() {
        let json = serde_json::json!({
            "id": "001xx000003DgAAAS",
            "success": true,
            "errors": [],
            "created": true
        });
        let result: CollectionResult = serde_json::from_value(json).unwrap();
        assert!(result.success);
        assert_eq!(result.created, Some(true));
    }

    #[test]
    fn test_search_request_roundtrip() {
        let req = SearchRequest {
            sosl: "FIND {Acme} IN ALL FIELDS RETURNING Account(Id, Name)".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: SearchRequest = serde_json::from_str(&json).unwrap();
        assert!(deserialized.sosl.contains("FIND {Acme}"));
    }

    #[test]
    fn test_salesforce_api_error_serialization() {
        let err = SalesforceApiError {
            status_code: "INVALID_FIELD".to_string(),
            message: "No such column 'Foo'".to_string(),
            fields: vec!["Foo".to_string()],
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["statusCode"], "INVALID_FIELD");
        let deserialized: SalesforceApiError = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.fields, vec!["Foo"]);
    }

    #[test]
    fn test_host_fn_names_are_unique() {
        use host_fn_names::*;
        let names = [
            QUERY,
            CREATE,
            GET,
            UPDATE,
            DELETE,
            UPSERT,
            DESCRIBE_GLOBAL,
            DESCRIBE_SOBJECT,
            SEARCH,
            COMPOSITE,
            CREATE_MULTIPLE,
            DELETE_MULTIPLE,
            LIMITS,
        ];
        let mut unique = std::collections::HashSet::new();
        for name in &names {
            assert!(unique.insert(name), "duplicate host function name: {name}");
        }
    }

    #[test]
    fn test_delete_multiple_request() {
        let req = DeleteMultipleRequest {
            ids: vec!["001xx000003Dg1".to_string(), "001xx000003Dg2".to_string()],
            all_or_none: true,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["ids"].as_array().unwrap().len(), 2);
        assert!(json["all_or_none"].as_bool().unwrap());
    }
}
