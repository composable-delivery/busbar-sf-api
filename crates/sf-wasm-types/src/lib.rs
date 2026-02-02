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

    pub fn err_with_fields(
        code: impl Into<String>,
        message: impl Into<String>,
        fields: Vec<String>,
    ) -> Self {
        BridgeResult::Err(BridgeError {
            code: code.into(),
            message: message.into(),
            fields,
        })
    }

    pub fn into_result(self) -> Result<T, BridgeError> {
        match self {
            BridgeResult::Ok(v) => Ok(v),
            BridgeResult::Err(e) => Err(e),
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, BridgeResult::Ok(_))
    }

    pub fn is_err(&self) -> bool {
        matches!(self, BridgeResult::Err(_))
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
// REST API: Query
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

/// Request to fetch the next page of query results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMoreRequest {
    /// The `next_records_url` from the previous query response.
    pub next_records_url: String,
}

// =============================================================================
// REST API: CRUD Operations
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

/// Request that identifies a resource by ID only (used for various single-ID operations).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdRequest {
    /// Resource ID (can be user ID, config ID, etc.).
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
// REST API: Describe
// =============================================================================

/// Request to describe a specific SObject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeSObjectRequest {
    /// SObject type (e.g., "Account").
    pub sobject: String,
}

// =============================================================================
// REST API: Search (SOSL)
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
// REST API: Composite
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

/// Request for a composite batch API call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeBatchRequest {
    /// If true, halt execution on first error.
    pub halt_on_error: bool,
    /// The batch subrequests to execute.
    pub subrequests: Vec<CompositeBatchSubrequest>,
}

/// A single subrequest in a composite batch call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeBatchSubrequest {
    /// HTTP method.
    pub method: String,
    /// Relative URL.
    pub url: String,
    /// Optional request body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rich_input: Option<serde_json::Value>,
}

/// Response from a composite batch API call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeBatchResponse {
    pub has_errors: bool,
    pub results: Vec<CompositeBatchSubresponse>,
}

/// Response from a single composite batch subrequest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeBatchSubresponse {
    pub status_code: u16,
    pub result: serde_json::Value,
}

/// Request for a composite tree API call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeTreeRequest {
    /// SObject type for the root records.
    pub sobject: String,
    /// Records with nested children.
    pub records: Vec<serde_json::Value>,
}

/// Response from a composite tree API call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeTreeResponse {
    pub has_errors: bool,
    pub results: Vec<CompositeTreeResult>,
}

/// Result of a single record in a composite tree response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeTreeResult {
    pub reference_id: String,
    pub id: Option<String>,
    #[serde(default)]
    pub errors: Vec<SalesforceApiError>,
}

// =============================================================================
// REST API: Collections (Batch CRUD)
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

/// Request to update multiple records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMultipleRequest {
    /// SObject type.
    pub sobject: String,
    /// Records to update as (id, fields) pairs.
    pub records: Vec<UpdateMultipleRecord>,
    /// If true, all records fail if any single record fails.
    pub all_or_none: bool,
}

/// A single record in an update multiple request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMultipleRecord {
    /// Record ID.
    pub id: String,
    /// Fields to update.
    pub fields: serde_json::Value,
}

/// Request to get multiple records by ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMultipleRequest {
    /// SObject type.
    pub sobject: String,
    /// Record IDs (up to 2000).
    pub ids: Vec<String>,
    /// Fields to retrieve.
    pub fields: Vec<String>,
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
// REST API: Limits & Versions
// =============================================================================

/// Response from the limits endpoint.
///
/// Returned as a JSON object where keys are limit names and values
/// contain `Max` and `Remaining` fields.
pub type LimitsResponse = serde_json::Value;

/// A single API version entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiVersion {
    pub label: String,
    pub url: String,
    pub version: String,
}

// =============================================================================
// Bulk API 2.0
// =============================================================================

/// Request to create a bulk ingest job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkCreateIngestJobRequest {
    /// SObject API name.
    pub sobject: String,
    /// Operation: insert, update, upsert, delete, hardDelete.
    pub operation: String,
    /// External ID field name (required for upsert).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id_field: Option<String>,
    /// Column delimiter (COMMA, TAB, SEMICOLON, PIPE, BACKQUOTE, CARET).
    #[serde(default = "default_column_delimiter")]
    pub column_delimiter: String,
    /// Line ending (LF, CRLF).
    #[serde(default = "default_line_ending")]
    pub line_ending: String,
}

fn default_column_delimiter() -> String {
    "COMMA".to_string()
}

fn default_line_ending() -> String {
    "LF".to_string()
}

/// Response from bulk job operations (create, close, abort, get).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkJobResponse {
    pub id: String,
    pub state: String,
    pub object: String,
    pub operation: String,
    #[serde(default)]
    pub number_records_processed: i64,
    #[serde(default)]
    pub number_records_failed: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_modstamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Request to upload CSV data to a bulk ingest job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkUploadJobDataRequest {
    pub job_id: String,
    pub csv_data: String,
}

/// Request that identifies a bulk job by ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkJobIdRequest {
    pub job_id: String,
}

/// Request to get job results (successful, failed, or unprocessed records).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkJobResultsRequest {
    pub job_id: String,
    /// One of: "successful", "failed", "unprocessed".
    pub result_type: String,
}

/// Response containing CSV results from a bulk job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkJobResultsResponse {
    pub csv_data: String,
}

/// Response from listing all ingest jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkJobListResponse {
    pub records: Vec<BulkJobResponse>,
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_records_url: Option<String>,
}

/// Request to get query job results with optional pagination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkQueryResultsRequest {
    pub job_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_records: Option<u64>,
}

/// Response containing CSV results from a bulk query job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkQueryResultsResponse {
    pub csv_data: String,
    /// Locator for next page, None if all results returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
}

// =============================================================================
// Tooling API
// =============================================================================

/// Request for a Tooling API SOQL query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolingQueryRequest {
    pub soql: String,
}

/// Request to execute anonymous Apex code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteAnonymousRequest {
    pub apex_code: String,
}

/// Response from executing anonymous Apex code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteAnonymousResponse {
    pub compiled: bool,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compile_problem: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exception_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exception_stack_trace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<i32>,
}

/// Request to get a Tooling API record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolingGetRequest {
    pub sobject: String,
    pub id: String,
}

/// Request to create a Tooling API record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolingCreateRequest {
    pub sobject: String,
    pub record: serde_json::Value,
}

/// Request to delete a Tooling API record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolingDeleteRequest {
    pub sobject: String,
    pub id: String,
}

// =============================================================================
// Metadata API
// =============================================================================

/// Request to deploy metadata (zipped package).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataDeployRequest {
    /// Base64-encoded zip file containing the metadata package.
    pub zip_base64: String,
    /// Deploy options.
    #[serde(default)]
    pub options: MetadataDeployOptions,
}

/// Options for a metadata deployment.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetadataDeployOptions {
    /// If true, validate only (don't actually deploy).
    #[serde(default)]
    pub check_only: bool,
    /// Test level: NoTestRun, RunLocalTests, RunAllTestsInOrg, RunSpecifiedTests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_level: Option<String>,
    /// Specific tests to run (when test_level is RunSpecifiedTests).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_tests: Vec<String>,
    /// If true, roll back on error.
    #[serde(default = "default_true")]
    pub rollback_on_error: bool,
}

fn default_true() -> bool {
    true
}

/// Response from a metadata deploy request (async process ID).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataDeployResponse {
    pub async_process_id: String,
}

/// Request to check deploy status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataCheckDeployStatusRequest {
    pub async_process_id: String,
    #[serde(default)]
    pub include_details: bool,
}

/// Result of a metadata deploy operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataDeployResult {
    pub id: String,
    pub done: bool,
    pub status: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(default)]
    pub number_component_errors: i32,
    #[serde(default)]
    pub number_components_deployed: i32,
    #[serde(default)]
    pub number_components_total: i32,
    #[serde(default)]
    pub number_test_errors: i32,
    #[serde(default)]
    pub number_tests_completed: i32,
    #[serde(default)]
    pub number_tests_total: i32,
}

/// Request to retrieve metadata as a zip package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataRetrieveRequest {
    /// If true, retrieve a named managed package.
    /// If false, retrieve unpackaged metadata using the `types` field.
    #[serde(default)]
    pub is_packaged: bool,
    /// For packaged retrieval: the managed package name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    /// For unpackaged retrieval: the metadata types to include.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<MetadataPackageType>,
    /// API version for the package manifest (default: "62.0").
    #[serde(default = "default_api_version")]
    pub api_version: String,
}

/// A metadata type entry in a package manifest for retrieve operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataPackageType {
    /// The metadata type name (e.g., "ApexClass", "ApexTrigger").
    pub name: String,
    /// Members to include. Use `["*"]` for all members.
    pub members: Vec<String>,
}

fn default_api_version() -> String {
    "65.0".to_string()
}

/// Response from a metadata retrieve request (async process ID).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataRetrieveResponse {
    pub async_process_id: String,
}

/// Request to check retrieve status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataCheckRetrieveStatusRequest {
    pub async_process_id: String,
    #[serde(default)]
    pub include_zip: bool,
}

/// Result of a metadata retrieve operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataRetrieveResult {
    pub id: String,
    pub done: bool,
    pub status: String,
    pub success: bool,
    /// Base64-encoded zip file (if include_zip was true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Request to list metadata components of a given type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataListRequest {
    pub metadata_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
}

/// A metadata component entry from list_metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataComponentInfo {
    pub full_name: String,
    pub file_name: String,
    pub component_type: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified_date: Option<String>,
}

/// Response from describe_metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataDescribeResult {
    pub metadata_objects: Vec<MetadataTypeInfo>,
    pub organization_namespace: String,
    pub partial_save_allowed: bool,
    pub test_required: bool,
}

/// Information about a metadata type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataTypeInfo {
    pub xml_name: String,
    pub directory_name: String,
    pub suffix: Option<String>,
    pub in_folder: bool,
    pub meta_file: bool,
    #[serde(default)]
    pub child_xml_names: Vec<String>,
}

// =============================================================================
// REST API: Process Rules & Approvals
// =============================================================================

/// Response from list_process_rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRuleCollection {
    #[serde(default)]
    pub rules: std::collections::HashMap<String, Vec<ProcessRule>>,
}

/// A process rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRule {
    pub id: String,
    pub name: String,
    #[serde(rename = "sobjectType")]
    pub sobject_type: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

/// Request to list process rules for a specific SObject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProcessRulesForSObjectRequest {
    pub sobject: String,
}

/// Request to trigger process rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRuleRequest {
    #[serde(rename = "contextIds")]
    pub context_ids: Vec<String>,
}

/// Result of triggering a process rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRuleResult {
    #[serde(default)]
    pub errors: Vec<SalesforceApiError>,
    pub success: bool,
}

/// Response from list_pending_approvals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApprovalCollection {
    #[serde(default)]
    pub approvals: std::collections::HashMap<String, Vec<PendingApproval>>,
}

/// A pending approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApproval {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(rename = "sortOrder", default)]
    pub sort_order: Option<i32>,
}

/// Request to submit an approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    #[serde(rename = "actionType")]
    pub action_type: String,
    #[serde(rename = "contextId")]
    pub context_id: String,
    #[serde(rename = "contextActorId", skip_serializing_if = "Option::is_none")]
    pub context_actor_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
    #[serde(rename = "nextApproverIds", skip_serializing_if = "Option::is_none")]
    pub next_approver_ids: Option<Vec<String>>,
    #[serde(
        rename = "processDefinitionNameOrId",
        skip_serializing_if = "Option::is_none"
    )]
    pub process_definition_name_or_id: Option<String>,
    #[serde(rename = "skipEntryCriteria", skip_serializing_if = "Option::is_none")]
    pub skip_entry_criteria: Option<bool>,
}

/// Result of an approval submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResult {
    #[serde(rename = "actorIds", default)]
    pub actor_ids: Vec<String>,
    #[serde(rename = "entityId")]
    pub entity_id: String,
    #[serde(default)]
    pub errors: Vec<SalesforceApiError>,
    #[serde(rename = "instanceId")]
    pub instance_id: String,
    #[serde(rename = "instanceStatus")]
    pub instance_status: String,
    #[serde(rename = "newWorkitemIds", default)]
    pub new_workitem_ids: Vec<String>,
    pub success: bool,
}

// =============================================================================
// REST API: List Views
// =============================================================================

/// Request to list views for an SObject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListViewsRequest {
    pub sobject: String,
}

/// Response from list_views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListViewsResult {
    pub done: bool,
    #[serde(rename = "nextRecordsUrl")]
    pub next_records_url: Option<String>,
    #[serde(alias = "listViews", default)]
    pub listviews: Vec<ListView>,
}

/// A list view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListView {
    pub id: String,
    #[serde(rename = "developerName")]
    pub developer_name: String,
    pub label: String,
    #[serde(rename = "describeUrl")]
    pub describe_url: String,
    #[serde(rename = "resultsUrl")]
    pub results_url: String,
    #[serde(rename = "sobjectType")]
    pub sobject_type: String,
}

/// Request to get or describe a list view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListViewRequest {
    pub sobject: String,
    pub list_view_id: String,
}

/// Detailed description of a list view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListViewDescribe {
    pub id: String,
    #[serde(rename = "developerName")]
    pub developer_name: String,
    pub label: String,
    #[serde(rename = "sobjectType")]
    pub sobject_type: String,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub columns: Vec<ListViewColumn>,
    #[serde(rename = "orderBy", default)]
    pub order_by: Vec<serde_json::Value>,
    #[serde(rename = "whereCondition")]
    pub where_condition: Option<serde_json::Value>,
}

/// A column in a list view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListViewColumn {
    #[serde(rename = "fieldNameOrPath")]
    pub field_name_or_path: String,
    pub label: String,
    pub sortable: bool,
    #[serde(rename = "type")]
    pub field_type: String,
}

// =============================================================================
// REST API: Quick Actions
// =============================================================================

/// A quick action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickActionMetadata {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub action_type: String,
}

/// Request to describe a global quick action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeGlobalQuickActionRequest {
    pub action: String,
}

/// Request to list quick actions for an SObject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListQuickActionsRequest {
    pub sobject: String,
}

/// Request to describe a quick action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeQuickActionRequest {
    pub sobject: String,
    pub action: String,
}

/// Detailed description of a quick action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickActionDescribe {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(rename = "targetSobjectType")]
    pub target_sobject_type: Option<String>,
    #[serde(rename = "targetRecordTypeId")]
    pub target_record_type_id: Option<String>,
    #[serde(rename = "targetParentField")]
    pub target_parent_field: Option<String>,
    pub layout: Option<serde_json::Value>,
    #[serde(rename = "defaultValues")]
    pub default_values: Option<serde_json::Value>,
    #[serde(default)]
    pub icons: Vec<serde_json::Value>,
}

/// Request to invoke a quick action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeQuickActionRequest {
    pub sobject: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,
    pub body: serde_json::Value,
}

// =============================================================================
// REST API: Sync (Get Deleted/Updated)
// =============================================================================

/// Request to get deleted records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDeletedRequest {
    pub sobject: String,
    pub start: String,
    pub end: String,
}

/// Response from get_deleted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDeletedResult {
    #[serde(rename = "deletedRecords")]
    pub deleted_records: Vec<DeletedRecord>,
    #[serde(rename = "earliestDateAvailable")]
    pub earliest_date_available: String,
    #[serde(rename = "latestDateCovered")]
    pub latest_date_covered: String,
}

/// A deleted record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletedRecord {
    pub id: String,
    #[serde(rename = "deletedDate")]
    pub deleted_date: String,
}

/// Request to get updated records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUpdatedRequest {
    pub sobject: String,
    pub start: String,
    pub end: String,
}

/// Response from get_updated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUpdatedResult {
    pub ids: Vec<String>,
    #[serde(rename = "latestDateCovered")]
    pub latest_date_covered: String,
}

// =============================================================================
// Priority 2: Invocable Actions, Layouts, Knowledge, Standalone, etc.
// =============================================================================

/// Request to invoke an invocable action (standard or custom).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeActionRequest {
    pub action_name: String,
    pub inputs: Vec<serde_json::Value>,
}

/// Request to invoke a custom action (requires action_type + action_name).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeCustomActionRequest {
    pub action_type: String,
    pub action_name: String,
    pub inputs: Vec<serde_json::Value>,
}

/// Request for describe custom action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeCustomActionRequest {
    pub action_type: String,
    pub action_name: String,
}

/// Request for list custom actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListCustomActionsRequest {
    pub action_type: String,
}

/// Request for describe named layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeNamedLayoutRequest {
    pub sobject: String,
    pub layout_name: String,
}

/// Request for knowledge articles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeArticlesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
}

/// Request for data category groups.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataCategoryGroupsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sobject: Option<String>,
}

/// Request for data categories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataCategoriesRequest {
    pub group: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sobject: Option<String>,
}

/// Request for app menu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMenuRequest {
    pub app_menu_type: String,
}

/// Request for compact layouts (multi-sobject).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactLayoutsMultiRequest {
    pub sobject_list: String,
}

/// Request for platform event schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformEventSchemaRequest {
    pub event_name: String,
}

/// Request for set user password.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetUserPasswordRequest {
    pub user_id: String,
    pub password: String,
}

/// Request for read consent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadConsentRequest {
    pub action: String,
    pub ids: Vec<String>,
}

/// Request for write consent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteConsentRequest {
    pub action: String,
    pub records: Vec<ConsentWriteRecord>,
}

/// Individual consent record for write operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentWriteRecord {
    pub id: String,
    pub result: String,
}

/// Request for read multi consent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadMultiConsentRequest {
    pub actions: Vec<String>,
    pub ids: Vec<String>,
}

/// Request for get blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobRequest {
    pub sobject: String,
    pub id: String,
    pub field: String,
}

/// Response for get blob (base64-encoded).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobResponse {
    pub data_base64: String,
}

/// Request for get rich text image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRichTextImageRequest {
    pub sobject: String,
    pub id: String,
    pub field: String,
    pub content_reference_id: String,
}

/// Response for get rich text image (base64-encoded).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRichTextImageResponse {
    pub data_base64: String,
}

/// Request for get relationship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRelationshipRequest {
    pub sobject: String,
    pub id: String,
    pub relationship_name: String,
}

/// Request for search suggestions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestionsRequest {
    pub query: String,
    pub sobject: String,
}

/// Request for search result layouts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultLayoutsRequest {
    pub sobjects: Vec<String>,
}

// =============================================================================
// Host Function Names (constants for ABI contract)
// =============================================================================

/// Host function name constants.
///
/// These are the names used to register/import host functions across
/// the WASM boundary. Both sf-bridge and sf-guest-sdk use these.
pub mod host_fn_names {
    // REST API
    pub const QUERY: &str = "sf_query";
    pub const QUERY_MORE: &str = "sf_query_more";
    pub const CREATE: &str = "sf_create";
    pub const GET: &str = "sf_get";
    pub const UPDATE: &str = "sf_update";
    pub const DELETE: &str = "sf_delete";
    pub const UPSERT: &str = "sf_upsert";
    pub const DESCRIBE_GLOBAL: &str = "sf_describe_global";
    pub const DESCRIBE_SOBJECT: &str = "sf_describe_sobject";
    pub const SEARCH: &str = "sf_search";
    pub const COMPOSITE: &str = "sf_composite";
    pub const COMPOSITE_BATCH: &str = "sf_composite_batch";
    pub const COMPOSITE_TREE: &str = "sf_composite_tree";
    pub const CREATE_MULTIPLE: &str = "sf_create_multiple";
    pub const UPDATE_MULTIPLE: &str = "sf_update_multiple";
    pub const GET_MULTIPLE: &str = "sf_get_multiple";
    pub const DELETE_MULTIPLE: &str = "sf_delete_multiple";
    pub const LIMITS: &str = "sf_limits";
    pub const VERSIONS: &str = "sf_versions";

    // REST API: Process & Approvals
    pub const LIST_PROCESS_RULES: &str = "sf_list_process_rules";
    pub const LIST_PROCESS_RULES_FOR_SOBJECT: &str = "sf_list_process_rules_for_sobject";
    pub const TRIGGER_PROCESS_RULES: &str = "sf_trigger_process_rules";
    pub const LIST_PENDING_APPROVALS: &str = "sf_list_pending_approvals";
    pub const SUBMIT_APPROVAL: &str = "sf_submit_approval";

    // REST API: List Views
    pub const LIST_VIEWS: &str = "sf_list_views";
    pub const GET_LIST_VIEW: &str = "sf_get_list_view";
    pub const DESCRIBE_LIST_VIEW: &str = "sf_describe_list_view";
    pub const EXECUTE_LIST_VIEW: &str = "sf_execute_list_view";

    // REST API: Quick Actions
    pub const LIST_GLOBAL_QUICK_ACTIONS: &str = "sf_list_global_quick_actions";
    pub const DESCRIBE_GLOBAL_QUICK_ACTION: &str = "sf_describe_global_quick_action";
    pub const LIST_QUICK_ACTIONS: &str = "sf_list_quick_actions";
    pub const DESCRIBE_QUICK_ACTION: &str = "sf_describe_quick_action";
    pub const INVOKE_QUICK_ACTION: &str = "sf_invoke_quick_action";

    // REST API: Sync
    pub const GET_DELETED: &str = "sf_get_deleted";
    pub const GET_UPDATED: &str = "sf_get_updated";

    // Bulk API
    pub const BULK_CREATE_INGEST_JOB: &str = "sf_bulk_create_ingest_job";
    pub const BULK_UPLOAD_JOB_DATA: &str = "sf_bulk_upload_job_data";
    pub const BULK_CLOSE_INGEST_JOB: &str = "sf_bulk_close_ingest_job";
    pub const BULK_ABORT_INGEST_JOB: &str = "sf_bulk_abort_ingest_job";
    pub const BULK_GET_INGEST_JOB: &str = "sf_bulk_get_ingest_job";
    pub const BULK_GET_JOB_RESULTS: &str = "sf_bulk_get_job_results";
    pub const BULK_DELETE_INGEST_JOB: &str = "sf_bulk_delete_ingest_job";
    pub const BULK_GET_ALL_INGEST_JOBS: &str = "sf_bulk_get_all_ingest_jobs";
    pub const BULK_ABORT_QUERY_JOB: &str = "sf_bulk_abort_query_job";
    pub const BULK_GET_QUERY_RESULTS: &str = "sf_bulk_get_query_results";

    // Tooling API
    pub const TOOLING_QUERY: &str = "sf_tooling_query";
    pub const TOOLING_EXECUTE_ANONYMOUS: &str = "sf_tooling_execute_anonymous";
    pub const TOOLING_GET: &str = "sf_tooling_get";
    pub const TOOLING_CREATE: &str = "sf_tooling_create";
    pub const TOOLING_DELETE: &str = "sf_tooling_delete";

    // Metadata API
    pub const METADATA_DEPLOY: &str = "sf_metadata_deploy";
    pub const METADATA_CHECK_DEPLOY_STATUS: &str = "sf_metadata_check_deploy_status";
    pub const METADATA_RETRIEVE: &str = "sf_metadata_retrieve";
    pub const METADATA_CHECK_RETRIEVE_STATUS: &str = "sf_metadata_check_retrieve_status";
    pub const METADATA_LIST: &str = "sf_metadata_list";
    pub const METADATA_DESCRIBE: &str = "sf_metadata_describe";

    // REST API: Invocable Actions
    pub const LIST_STANDARD_ACTIONS: &str = "sf_list_standard_actions";
    pub const LIST_CUSTOM_ACTION_TYPES: &str = "sf_list_custom_action_types";
    pub const LIST_CUSTOM_ACTIONS: &str = "sf_list_custom_actions";
    pub const DESCRIBE_STANDARD_ACTION: &str = "sf_describe_standard_action";
    pub const DESCRIBE_CUSTOM_ACTION: &str = "sf_describe_custom_action";
    pub const INVOKE_STANDARD_ACTION: &str = "sf_invoke_standard_action";
    pub const INVOKE_CUSTOM_ACTION: &str = "sf_invoke_custom_action";

    // REST API: Layouts
    pub const DESCRIBE_LAYOUTS: &str = "sf_describe_layouts";
    pub const DESCRIBE_NAMED_LAYOUT: &str = "sf_describe_named_layout";
    pub const DESCRIBE_APPROVAL_LAYOUTS: &str = "sf_describe_approval_layouts";
    pub const DESCRIBE_COMPACT_LAYOUTS: &str = "sf_describe_compact_layouts";
    pub const DESCRIBE_GLOBAL_PUBLISHER_LAYOUTS: &str = "sf_describe_global_publisher_layouts";

    // REST API: Knowledge
    pub const KNOWLEDGE_SETTINGS: &str = "sf_knowledge_settings";
    pub const KNOWLEDGE_ARTICLES: &str = "sf_knowledge_articles";
    pub const DATA_CATEGORY_GROUPS: &str = "sf_data_category_groups";
    pub const DATA_CATEGORIES: &str = "sf_data_categories";

    // REST API: Standalone
    pub const TABS: &str = "sf_tabs";
    pub const THEME: &str = "sf_theme";
    pub const APP_MENU: &str = "sf_app_menu";
    pub const RECENT_ITEMS: &str = "sf_recent_items";
    pub const RELEVANT_ITEMS: &str = "sf_relevant_items";
    pub const COMPACT_LAYOUTS_MULTI: &str = "sf_compact_layouts_multi";
    pub const PLATFORM_EVENT_SCHEMA: &str = "sf_platform_event_schema";
    pub const LIGHTNING_TOGGLE_METRICS: &str = "sf_lightning_toggle_metrics";
    pub const LIGHTNING_USAGE: &str = "sf_lightning_usage";

    // REST API: User Password
    pub const GET_USER_PASSWORD_STATUS: &str = "sf_get_user_password_status";
    pub const SET_USER_PASSWORD: &str = "sf_set_user_password";
    pub const RESET_USER_PASSWORD: &str = "sf_reset_user_password";

    // REST API: Scheduler
    pub const APPOINTMENT_CANDIDATES: &str = "sf_appointment_candidates";
    pub const APPOINTMENT_SLOTS: &str = "sf_appointment_slots";

    // REST API: Consent
    pub const READ_CONSENT: &str = "sf_read_consent";
    pub const WRITE_CONSENT: &str = "sf_write_consent";
    pub const READ_MULTI_CONSENT: &str = "sf_read_multi_consent";

    // REST API: Binary
    pub const GET_BLOB: &str = "sf_get_blob";
    pub const GET_RICH_TEXT_IMAGE: &str = "sf_get_rich_text_image";
    pub const GET_RELATIONSHIP: &str = "sf_get_relationship";

    // REST API: Embedded Service
    pub const GET_EMBEDDED_SERVICE_CONFIG: &str = "sf_get_embedded_service_config";

    // REST API: Search Enhancements
    pub const PARAMETERIZED_SEARCH: &str = "sf_parameterized_search";
    pub const SEARCH_SUGGESTIONS: &str = "sf_search_suggestions";
    pub const SEARCH_SCOPE_ORDER: &str = "sf_search_scope_order";
    pub const SEARCH_RESULT_LAYOUTS: &str = "sf_search_result_layouts";

    // REST API: Composite Enhancement
    pub const COMPOSITE_GRAPH: &str = "sf_composite_graph";
}

/// The Extism namespace used for all bridge host functions.
pub const BRIDGE_NAMESPACE: &str = "busbar";

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // BridgeResult & BridgeError
    // =========================================================================

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
    fn test_bridge_result_err_with_fields() {
        let result: BridgeResult<String> =
            BridgeResult::err_with_fields("FIELD_ERR", "bad field", vec!["Name".to_string()]);
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Name"));
        match serde_json::from_str::<BridgeResult<String>>(&json).unwrap() {
            BridgeResult::Err(e) => {
                assert_eq!(e.fields, vec!["Name"]);
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
    fn test_bridge_result_is_ok_is_err() {
        let ok: BridgeResult<u32> = BridgeResult::ok(42);
        assert!(ok.is_ok());
        assert!(!ok.is_err());

        let err: BridgeResult<u32> = BridgeResult::err("FAIL", "failed");
        assert!(err.is_err());
        assert!(!err.is_ok());
    }

    #[test]
    fn test_bridge_result_from_into() {
        let ok: BridgeResult<u32> = BridgeResult::ok(42);
        let result: Result<u32, BridgeError> = ok.into();
        assert_eq!(result.unwrap(), 42);

        let err: BridgeResult<u32> = BridgeResult::err("FAIL", "failed");
        let result: Result<u32, BridgeError> = err.into();
        assert!(result.is_err());
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
    fn test_bridge_error_is_error_trait() {
        let err = BridgeError {
            code: "TEST".to_string(),
            message: "test error".to_string(),
            fields: vec![],
        };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_bridge_error_fields_skipped_when_empty() {
        let err = BridgeError {
            code: "X".to_string(),
            message: "y".to_string(),
            fields: vec![],
        };
        let json = serde_json::to_value(&err).unwrap();
        assert!(json.get("fields").is_none());
    }

    #[test]
    fn test_bridge_error_fields_present_when_nonempty() {
        let err = BridgeError {
            code: "X".to_string(),
            message: "y".to_string(),
            fields: vec!["f1".to_string()],
        };
        let json = serde_json::to_value(&err).unwrap();
        assert!(json.get("fields").is_some());
    }

    // =========================================================================
    // SalesforceApiError
    // =========================================================================

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

    // =========================================================================
    // REST API: Query
    // =========================================================================

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
    fn test_query_request_include_deleted_defaults_false() {
        let json = serde_json::json!({"soql": "SELECT Id FROM Account"});
        let req: QueryRequest = serde_json::from_value(json).unwrap();
        assert!(!req.include_deleted);
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
    fn test_query_more_request_roundtrip() {
        let req = QueryMoreRequest {
            next_records_url: "/services/data/v62.0/query/01gxx-2000".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: QueryMoreRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.next_records_url, req.next_records_url);
    }

    // =========================================================================
    // REST API: CRUD
    // =========================================================================

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
    fn test_update_request_roundtrip() {
        let req = UpdateRequest {
            sobject: "Account".to_string(),
            id: "001xx000003DgAAAS".to_string(),
            record: serde_json::json!({"Name": "Updated"}),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: UpdateRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.sobject, "Account");
        assert_eq!(d.id, "001xx000003DgAAAS");
    }

    #[test]
    fn test_delete_request_roundtrip() {
        let req = DeleteRequest {
            sobject: "Account".to_string(),
            id: "001xx000003DgAAAS".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: DeleteRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.sobject, "Account");
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

    // =========================================================================
    // REST API: Describe & Search
    // =========================================================================

    #[test]
    fn test_describe_sobject_request_roundtrip() {
        let req = DescribeSObjectRequest {
            sobject: "Account".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: DescribeSObjectRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.sobject, "Account");
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
    fn test_search_response_roundtrip() {
        let resp = SearchResponse {
            search_records: vec![serde_json::json!({"Id": "001xx", "Name": "Acme"})],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let d: SearchResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(d.search_records.len(), 1);
    }

    // =========================================================================
    // REST API: Composite
    // =========================================================================

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
            "responses": [{
                "body": {"id": "001xx", "success": true},
                "http_status_code": 201,
                "reference_id": "NewAccount"
            }]
        });
        let resp: CompositeResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.responses.len(), 1);
        assert_eq!(resp.responses[0].http_status_code, 201);
    }

    #[test]
    fn test_composite_batch_request_roundtrip() {
        let req = CompositeBatchRequest {
            halt_on_error: true,
            subrequests: vec![CompositeBatchSubrequest {
                method: "GET".to_string(),
                url: "/services/data/v62.0/sobjects/Account/001xx".to_string(),
                rich_input: None,
            }],
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: CompositeBatchRequest = serde_json::from_str(&json).unwrap();
        assert!(d.halt_on_error);
        assert_eq!(d.subrequests.len(), 1);
    }

    #[test]
    fn test_composite_batch_response_roundtrip() {
        let resp = CompositeBatchResponse {
            has_errors: false,
            results: vec![CompositeBatchSubresponse {
                status_code: 200,
                result: serde_json::json!({"Id": "001xx"}),
            }],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let d: CompositeBatchResponse = serde_json::from_str(&json).unwrap();
        assert!(!d.has_errors);
        assert_eq!(d.results[0].status_code, 200);
    }

    #[test]
    fn test_composite_tree_request_roundtrip() {
        let req = CompositeTreeRequest {
            sobject: "Account".to_string(),
            records: vec![serde_json::json!({
                "attributes": {"type": "Account"},
                "referenceId": "ref1",
                "Name": "Parent"
            })],
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: CompositeTreeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.sobject, "Account");
        assert_eq!(d.records.len(), 1);
    }

    #[test]
    fn test_composite_tree_response_roundtrip() {
        let resp = CompositeTreeResponse {
            has_errors: false,
            results: vec![CompositeTreeResult {
                reference_id: "ref1".to_string(),
                id: Some("001xx".to_string()),
                errors: vec![],
            }],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let d: CompositeTreeResponse = serde_json::from_str(&json).unwrap();
        assert!(!d.has_errors);
        assert_eq!(d.results[0].id, Some("001xx".to_string()));
    }

    // =========================================================================
    // REST API: Collections
    // =========================================================================

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
    fn test_update_multiple_request_roundtrip() {
        let req = UpdateMultipleRequest {
            sobject: "Account".to_string(),
            records: vec![UpdateMultipleRecord {
                id: "001xx000003DgAAAS".to_string(),
                fields: serde_json::json!({"Name": "Updated"}),
            }],
            all_or_none: true,
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: UpdateMultipleRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.records.len(), 1);
        assert_eq!(d.records[0].id, "001xx000003DgAAAS");
    }

    #[test]
    fn test_get_multiple_request_roundtrip() {
        let req = GetMultipleRequest {
            sobject: "Account".to_string(),
            ids: vec!["001xx1".to_string(), "001xx2".to_string()],
            fields: vec!["Id".to_string(), "Name".to_string()],
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: GetMultipleRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.ids.len(), 2);
        assert_eq!(d.fields.len(), 2);
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
    fn test_delete_multiple_request() {
        let req = DeleteMultipleRequest {
            ids: vec!["001xx000003Dg1".to_string(), "001xx000003Dg2".to_string()],
            all_or_none: true,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["ids"].as_array().unwrap().len(), 2);
    }

    // =========================================================================
    // REST API: Versions
    // =========================================================================

    #[test]
    fn test_api_version_roundtrip() {
        let v = ApiVersion {
            label: "Winter '25".to_string(),
            url: "/services/data/v62.0".to_string(),
            version: "62.0".to_string(),
        };
        let json = serde_json::to_string(&v).unwrap();
        let d: ApiVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(d.version, "62.0");
    }

    // =========================================================================
    // Bulk API 2.0
    // =========================================================================

    #[test]
    fn test_bulk_create_ingest_job_request_roundtrip() {
        let req = BulkCreateIngestJobRequest {
            sobject: "Account".to_string(),
            operation: "insert".to_string(),
            external_id_field: None,
            column_delimiter: "COMMA".to_string(),
            line_ending: "LF".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: BulkCreateIngestJobRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.sobject, "Account");
        assert_eq!(d.operation, "insert");
    }

    #[test]
    fn test_bulk_create_ingest_job_defaults() {
        let json = serde_json::json!({"sobject": "Account", "operation": "insert"});
        let req: BulkCreateIngestJobRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.column_delimiter, "COMMA");
        assert_eq!(req.line_ending, "LF");
    }

    #[test]
    fn test_bulk_job_response_roundtrip() {
        let resp = BulkJobResponse {
            id: "750xx".to_string(),
            state: "JobComplete".to_string(),
            object: "Account".to_string(),
            operation: "insert".to_string(),
            number_records_processed: 100,
            number_records_failed: 2,
            created_date: Some("2024-01-15T10:30:00.000Z".to_string()),
            system_modstamp: None,
            error_message: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let d: BulkJobResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(d.id, "750xx");
        assert_eq!(d.number_records_processed, 100);
    }

    #[test]
    fn test_bulk_job_response_defaults() {
        let json = serde_json::json!({
            "id": "750xx", "state": "Open", "object": "Account", "operation": "insert"
        });
        let resp: BulkJobResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.number_records_processed, 0);
        assert_eq!(resp.number_records_failed, 0);
    }

    #[test]
    fn test_bulk_upload_job_data_request_roundtrip() {
        let req = BulkUploadJobDataRequest {
            job_id: "750xx".to_string(),
            csv_data: "Name\nAcme\nWidget".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: BulkUploadJobDataRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.job_id, "750xx");
        assert!(d.csv_data.contains("Acme"));
    }

    #[test]
    fn test_bulk_job_id_request_roundtrip() {
        let req = BulkJobIdRequest {
            job_id: "750xx".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: BulkJobIdRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.job_id, "750xx");
    }

    #[test]
    fn test_bulk_job_results_request_roundtrip() {
        let req = BulkJobResultsRequest {
            job_id: "750xx".to_string(),
            result_type: "successful".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: BulkJobResultsRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.result_type, "successful");
    }

    #[test]
    fn test_bulk_job_results_response_roundtrip() {
        let resp = BulkJobResultsResponse {
            csv_data: "sf__Id,Name\n001xx,Acme".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let d: BulkJobResultsResponse = serde_json::from_str(&json).unwrap();
        assert!(d.csv_data.contains("sf__Id"));
    }

    #[test]
    fn test_bulk_job_list_response_roundtrip() {
        let resp = BulkJobListResponse {
            records: vec![BulkJobResponse {
                id: "750a".to_string(),
                state: "JobComplete".to_string(),
                object: "Account".to_string(),
                operation: "insert".to_string(),
                number_records_processed: 50,
                number_records_failed: 0,
                created_date: None,
                system_modstamp: None,
                error_message: None,
            }],
            done: true,
            next_records_url: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let d: BulkJobListResponse = serde_json::from_str(&json).unwrap();
        assert!(d.done);
        assert_eq!(d.records.len(), 1);
    }

    #[test]
    fn test_bulk_query_results_request_roundtrip() {
        let req = BulkQueryResultsRequest {
            job_id: "750xx".to_string(),
            locator: Some("abc123".to_string()),
            max_records: Some(1000),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: BulkQueryResultsRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.locator, Some("abc123".to_string()));
        assert_eq!(d.max_records, Some(1000));
    }

    #[test]
    fn test_bulk_query_results_request_minimal() {
        let json = serde_json::json!({"job_id": "750xx"});
        let req: BulkQueryResultsRequest = serde_json::from_value(json).unwrap();
        assert!(req.locator.is_none());
        assert!(req.max_records.is_none());
    }

    #[test]
    fn test_bulk_query_results_response_roundtrip() {
        let resp = BulkQueryResultsResponse {
            csv_data: "Id,Name\n001xx,Acme".to_string(),
            locator: Some("next_page".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let d: BulkQueryResultsResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(d.locator, Some("next_page".to_string()));
    }

    // =========================================================================
    // Tooling API
    // =========================================================================

    #[test]
    fn test_tooling_query_request_roundtrip() {
        let req = ToolingQueryRequest {
            soql: "SELECT Id, Name FROM ApexClass".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: ToolingQueryRequest = serde_json::from_str(&json).unwrap();
        assert!(d.soql.contains("ApexClass"));
    }

    #[test]
    fn test_execute_anonymous_request_roundtrip() {
        let req = ExecuteAnonymousRequest {
            apex_code: "System.debug('hello');".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: ExecuteAnonymousRequest = serde_json::from_str(&json).unwrap();
        assert!(d.apex_code.contains("System.debug"));
    }

    #[test]
    fn test_execute_anonymous_response_success() {
        let resp = ExecuteAnonymousResponse {
            compiled: true,
            success: true,
            compile_problem: None,
            exception_message: None,
            exception_stack_trace: None,
            line: None,
            column: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let d: ExecuteAnonymousResponse = serde_json::from_str(&json).unwrap();
        assert!(d.compiled);
        assert!(d.success);
    }

    #[test]
    fn test_execute_anonymous_response_compile_error() {
        let resp = ExecuteAnonymousResponse {
            compiled: false,
            success: false,
            compile_problem: Some("unexpected token".to_string()),
            exception_message: None,
            exception_stack_trace: None,
            line: Some(1),
            column: Some(5),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let d: ExecuteAnonymousResponse = serde_json::from_str(&json).unwrap();
        assert!(!d.compiled);
        assert_eq!(d.line, Some(1));
        assert_eq!(d.column, Some(5));
    }

    #[test]
    fn test_execute_anonymous_response_runtime_error() {
        let resp = ExecuteAnonymousResponse {
            compiled: true,
            success: false,
            compile_problem: None,
            exception_message: Some("List index out of bounds".to_string()),
            exception_stack_trace: Some("AnonymousBlock: line 3".to_string()),
            line: None,
            column: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let d: ExecuteAnonymousResponse = serde_json::from_str(&json).unwrap();
        assert!(d.compiled);
        assert!(!d.success);
        assert!(d.exception_message.is_some());
    }

    #[test]
    fn test_tooling_get_request_roundtrip() {
        let req = ToolingGetRequest {
            sobject: "ApexClass".to_string(),
            id: "01pxx".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: ToolingGetRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.sobject, "ApexClass");
    }

    #[test]
    fn test_tooling_create_request_roundtrip() {
        let req = ToolingCreateRequest {
            sobject: "ApexClass".to_string(),
            record: serde_json::json!({"Body": "public class Foo {}"}),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: ToolingCreateRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.record["Body"], "public class Foo {}");
    }

    #[test]
    fn test_tooling_delete_request_roundtrip() {
        let req = ToolingDeleteRequest {
            sobject: "ApexClass".to_string(),
            id: "01pxx".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: ToolingDeleteRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.id, "01pxx");
    }

    // =========================================================================
    // Metadata API
    // =========================================================================

    #[test]
    fn test_metadata_deploy_request_roundtrip() {
        let req = MetadataDeployRequest {
            zip_base64: "UEsDBBQ...".to_string(),
            options: MetadataDeployOptions {
                check_only: true,
                test_level: Some("RunLocalTests".to_string()),
                run_tests: vec![],
                rollback_on_error: true,
            },
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: MetadataDeployRequest = serde_json::from_str(&json).unwrap();
        assert!(d.options.check_only);
    }

    #[test]
    fn test_metadata_deploy_options_defaults() {
        let json = serde_json::json!({});
        let opts: MetadataDeployOptions = serde_json::from_value(json).unwrap();
        assert!(!opts.check_only);
        assert!(opts.test_level.is_none());
        assert!(opts.run_tests.is_empty());
        assert!(opts.rollback_on_error);
    }

    #[test]
    fn test_metadata_deploy_response_roundtrip() {
        let resp = MetadataDeployResponse {
            async_process_id: "0Af1234".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let d: MetadataDeployResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(d.async_process_id, "0Af1234");
    }

    #[test]
    fn test_metadata_check_deploy_status_request_roundtrip() {
        let req = MetadataCheckDeployStatusRequest {
            async_process_id: "0Af1234".to_string(),
            include_details: true,
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: MetadataCheckDeployStatusRequest = serde_json::from_str(&json).unwrap();
        assert!(d.include_details);
    }

    #[test]
    fn test_metadata_deploy_result_roundtrip() {
        let result = MetadataDeployResult {
            id: "0Af1234".to_string(),
            done: true,
            status: "Succeeded".to_string(),
            success: true,
            error_message: None,
            number_component_errors: 0,
            number_components_deployed: 5,
            number_components_total: 5,
            number_test_errors: 0,
            number_tests_completed: 10,
            number_tests_total: 10,
        };
        let json = serde_json::to_string(&result).unwrap();
        let d: MetadataDeployResult = serde_json::from_str(&json).unwrap();
        assert!(d.done);
        assert!(d.success);
        assert_eq!(d.number_components_deployed, 5);
    }

    #[test]
    fn test_metadata_retrieve_request_unpackaged() {
        let req = MetadataRetrieveRequest {
            is_packaged: false,
            package_name: None,
            types: vec![MetadataPackageType {
                name: "ApexClass".to_string(),
                members: vec!["*".to_string()],
            }],
            api_version: "62.0".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: MetadataRetrieveRequest = serde_json::from_str(&json).unwrap();
        assert!(!d.is_packaged);
        assert_eq!(d.types.len(), 1);
        assert_eq!(d.types[0].name, "ApexClass");
    }

    #[test]
    fn test_metadata_retrieve_request_packaged() {
        let req = MetadataRetrieveRequest {
            is_packaged: true,
            package_name: Some("MyPackage".to_string()),
            types: vec![],
            api_version: "62.0".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: MetadataRetrieveRequest = serde_json::from_str(&json).unwrap();
        assert!(d.is_packaged);
        assert_eq!(d.package_name, Some("MyPackage".to_string()));
    }

    #[test]
    fn test_metadata_retrieve_request_defaults() {
        let json = serde_json::json!({});
        let req: MetadataRetrieveRequest = serde_json::from_value(json).unwrap();
        assert!(!req.is_packaged);
        assert!(req.types.is_empty());
        assert_eq!(req.api_version, "65.0");
    }

    #[test]
    fn test_metadata_package_type_roundtrip() {
        let pt = MetadataPackageType {
            name: "ApexTrigger".to_string(),
            members: vec!["AccountTrigger".to_string(), "ContactTrigger".to_string()],
        };
        let json = serde_json::to_string(&pt).unwrap();
        let d: MetadataPackageType = serde_json::from_str(&json).unwrap();
        assert_eq!(d.name, "ApexTrigger");
        assert_eq!(d.members.len(), 2);
    }

    #[test]
    fn test_metadata_retrieve_response_roundtrip() {
        let resp = MetadataRetrieveResponse {
            async_process_id: "09S1234".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let d: MetadataRetrieveResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(d.async_process_id, "09S1234");
    }

    #[test]
    fn test_metadata_check_retrieve_status_request_roundtrip() {
        let req = MetadataCheckRetrieveStatusRequest {
            async_process_id: "09S1234".to_string(),
            include_zip: true,
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: MetadataCheckRetrieveStatusRequest = serde_json::from_str(&json).unwrap();
        assert!(d.include_zip);
    }

    #[test]
    fn test_metadata_retrieve_result_roundtrip() {
        let result = MetadataRetrieveResult {
            id: "09S1234".to_string(),
            done: true,
            status: "Succeeded".to_string(),
            success: true,
            zip_base64: Some("UEsDBBQ...".to_string()),
            error_message: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let d: MetadataRetrieveResult = serde_json::from_str(&json).unwrap();
        assert!(d.zip_base64.is_some());
    }

    #[test]
    fn test_metadata_list_request_roundtrip() {
        let req = MetadataListRequest {
            metadata_type: "ApexClass".to_string(),
            folder: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let d: MetadataListRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(d.metadata_type, "ApexClass");
    }

    #[test]
    fn test_metadata_list_request_with_folder() {
        let req = MetadataListRequest {
            metadata_type: "Report".to_string(),
            folder: Some("MyReports".to_string()),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["folder"], "MyReports");
    }

    #[test]
    fn test_metadata_component_info_roundtrip() {
        let comp = MetadataComponentInfo {
            full_name: "MyClass".to_string(),
            file_name: "classes/MyClass.cls".to_string(),
            component_type: "ApexClass".to_string(),
            id: "01pxx".to_string(),
            namespace_prefix: None,
            last_modified_date: Some("2024-01-15".to_string()),
        };
        let json = serde_json::to_string(&comp).unwrap();
        let d: MetadataComponentInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(d.full_name, "MyClass");
    }

    #[test]
    fn test_metadata_describe_result_roundtrip() {
        let result = MetadataDescribeResult {
            metadata_objects: vec![MetadataTypeInfo {
                xml_name: "ApexClass".to_string(),
                directory_name: "classes".to_string(),
                suffix: Some("cls".to_string()),
                in_folder: false,
                meta_file: true,
                child_xml_names: vec![],
            }],
            organization_namespace: "".to_string(),
            partial_save_allowed: true,
            test_required: false,
        };
        let json = serde_json::to_string(&result).unwrap();
        let d: MetadataDescribeResult = serde_json::from_str(&json).unwrap();
        assert_eq!(d.metadata_objects.len(), 1);
        assert_eq!(d.metadata_objects[0].xml_name, "ApexClass");
    }

    // =========================================================================
    // Host Function Names
    // =========================================================================

    #[test]
    fn test_host_fn_names_are_unique() {
        use host_fn_names::*;
        let names = [
            QUERY,
            QUERY_MORE,
            CREATE,
            GET,
            UPDATE,
            DELETE,
            UPSERT,
            DESCRIBE_GLOBAL,
            DESCRIBE_SOBJECT,
            SEARCH,
            COMPOSITE,
            COMPOSITE_BATCH,
            COMPOSITE_TREE,
            CREATE_MULTIPLE,
            UPDATE_MULTIPLE,
            GET_MULTIPLE,
            DELETE_MULTIPLE,
            LIMITS,
            VERSIONS,
            LIST_PROCESS_RULES,
            LIST_PROCESS_RULES_FOR_SOBJECT,
            TRIGGER_PROCESS_RULES,
            LIST_PENDING_APPROVALS,
            SUBMIT_APPROVAL,
            LIST_VIEWS,
            GET_LIST_VIEW,
            DESCRIBE_LIST_VIEW,
            EXECUTE_LIST_VIEW,
            LIST_GLOBAL_QUICK_ACTIONS,
            DESCRIBE_GLOBAL_QUICK_ACTION,
            LIST_QUICK_ACTIONS,
            DESCRIBE_QUICK_ACTION,
            INVOKE_QUICK_ACTION,
            GET_DELETED,
            GET_UPDATED,
            BULK_CREATE_INGEST_JOB,
            BULK_UPLOAD_JOB_DATA,
            BULK_CLOSE_INGEST_JOB,
            BULK_ABORT_INGEST_JOB,
            BULK_GET_INGEST_JOB,
            BULK_GET_JOB_RESULTS,
            BULK_DELETE_INGEST_JOB,
            BULK_GET_ALL_INGEST_JOBS,
            BULK_ABORT_QUERY_JOB,
            BULK_GET_QUERY_RESULTS,
            TOOLING_QUERY,
            TOOLING_EXECUTE_ANONYMOUS,
            TOOLING_GET,
            TOOLING_CREATE,
            TOOLING_DELETE,
            METADATA_DEPLOY,
            METADATA_CHECK_DEPLOY_STATUS,
            METADATA_RETRIEVE,
            METADATA_CHECK_RETRIEVE_STATUS,
            METADATA_LIST,
            METADATA_DESCRIBE,
            // Priority 2
            LIST_STANDARD_ACTIONS,
            LIST_CUSTOM_ACTION_TYPES,
            LIST_CUSTOM_ACTIONS,
            DESCRIBE_STANDARD_ACTION,
            DESCRIBE_CUSTOM_ACTION,
            INVOKE_STANDARD_ACTION,
            INVOKE_CUSTOM_ACTION,
            DESCRIBE_LAYOUTS,
            DESCRIBE_NAMED_LAYOUT,
            DESCRIBE_APPROVAL_LAYOUTS,
            DESCRIBE_COMPACT_LAYOUTS,
            DESCRIBE_GLOBAL_PUBLISHER_LAYOUTS,
            KNOWLEDGE_SETTINGS,
            KNOWLEDGE_ARTICLES,
            DATA_CATEGORY_GROUPS,
            DATA_CATEGORIES,
            TABS,
            THEME,
            APP_MENU,
            RECENT_ITEMS,
            RELEVANT_ITEMS,
            COMPACT_LAYOUTS_MULTI,
            PLATFORM_EVENT_SCHEMA,
            LIGHTNING_TOGGLE_METRICS,
            LIGHTNING_USAGE,
            GET_USER_PASSWORD_STATUS,
            SET_USER_PASSWORD,
            RESET_USER_PASSWORD,
            APPOINTMENT_CANDIDATES,
            APPOINTMENT_SLOTS,
            READ_CONSENT,
            WRITE_CONSENT,
            READ_MULTI_CONSENT,
            GET_BLOB,
            GET_RICH_TEXT_IMAGE,
            GET_RELATIONSHIP,
            GET_EMBEDDED_SERVICE_CONFIG,
            PARAMETERIZED_SEARCH,
            SEARCH_SUGGESTIONS,
            SEARCH_SCOPE_ORDER,
            SEARCH_RESULT_LAYOUTS,
            COMPOSITE_GRAPH,
        ];
        let mut unique = std::collections::HashSet::new();
        for name in &names {
            assert!(unique.insert(name), "duplicate host function name: {name}");
        }
        assert_eq!(unique.len(), 98);
    }

    #[test]
    fn test_host_fn_names_all_prefixed() {
        use host_fn_names::*;
        let names = [
            QUERY,
            QUERY_MORE,
            CREATE,
            GET,
            UPDATE,
            DELETE,
            UPSERT,
            DESCRIBE_GLOBAL,
            DESCRIBE_SOBJECT,
            SEARCH,
            COMPOSITE,
            COMPOSITE_BATCH,
            COMPOSITE_TREE,
            CREATE_MULTIPLE,
            UPDATE_MULTIPLE,
            GET_MULTIPLE,
            DELETE_MULTIPLE,
            LIMITS,
            VERSIONS,
            LIST_PROCESS_RULES,
            LIST_PROCESS_RULES_FOR_SOBJECT,
            TRIGGER_PROCESS_RULES,
            LIST_PENDING_APPROVALS,
            SUBMIT_APPROVAL,
            LIST_VIEWS,
            GET_LIST_VIEW,
            DESCRIBE_LIST_VIEW,
            EXECUTE_LIST_VIEW,
            LIST_GLOBAL_QUICK_ACTIONS,
            DESCRIBE_GLOBAL_QUICK_ACTION,
            LIST_QUICK_ACTIONS,
            DESCRIBE_QUICK_ACTION,
            INVOKE_QUICK_ACTION,
            GET_DELETED,
            GET_UPDATED,
            BULK_CREATE_INGEST_JOB,
            BULK_UPLOAD_JOB_DATA,
            BULK_CLOSE_INGEST_JOB,
            BULK_ABORT_INGEST_JOB,
            BULK_GET_INGEST_JOB,
            BULK_GET_JOB_RESULTS,
            BULK_DELETE_INGEST_JOB,
            BULK_GET_ALL_INGEST_JOBS,
            BULK_ABORT_QUERY_JOB,
            BULK_GET_QUERY_RESULTS,
            TOOLING_QUERY,
            TOOLING_EXECUTE_ANONYMOUS,
            TOOLING_GET,
            TOOLING_CREATE,
            TOOLING_DELETE,
            METADATA_DEPLOY,
            METADATA_CHECK_DEPLOY_STATUS,
            METADATA_RETRIEVE,
            METADATA_CHECK_RETRIEVE_STATUS,
            METADATA_LIST,
            METADATA_DESCRIBE,
            // Priority 2
            LIST_STANDARD_ACTIONS,
            LIST_CUSTOM_ACTION_TYPES,
            LIST_CUSTOM_ACTIONS,
            DESCRIBE_STANDARD_ACTION,
            DESCRIBE_CUSTOM_ACTION,
            INVOKE_STANDARD_ACTION,
            INVOKE_CUSTOM_ACTION,
            DESCRIBE_LAYOUTS,
            DESCRIBE_NAMED_LAYOUT,
            DESCRIBE_APPROVAL_LAYOUTS,
            DESCRIBE_COMPACT_LAYOUTS,
            DESCRIBE_GLOBAL_PUBLISHER_LAYOUTS,
            KNOWLEDGE_SETTINGS,
            KNOWLEDGE_ARTICLES,
            DATA_CATEGORY_GROUPS,
            DATA_CATEGORIES,
            TABS,
            THEME,
            APP_MENU,
            RECENT_ITEMS,
            RELEVANT_ITEMS,
            COMPACT_LAYOUTS_MULTI,
            PLATFORM_EVENT_SCHEMA,
            LIGHTNING_TOGGLE_METRICS,
            LIGHTNING_USAGE,
            GET_USER_PASSWORD_STATUS,
            SET_USER_PASSWORD,
            RESET_USER_PASSWORD,
            APPOINTMENT_CANDIDATES,
            APPOINTMENT_SLOTS,
            READ_CONSENT,
            WRITE_CONSENT,
            READ_MULTI_CONSENT,
            GET_BLOB,
            GET_RICH_TEXT_IMAGE,
            GET_RELATIONSHIP,
            GET_EMBEDDED_SERVICE_CONFIG,
            PARAMETERIZED_SEARCH,
            SEARCH_SUGGESTIONS,
            SEARCH_SCOPE_ORDER,
            SEARCH_RESULT_LAYOUTS,
            COMPOSITE_GRAPH,
        ];
        for name in &names {
            assert!(name.starts_with("sf_"), "{name} must start with sf_");
        }
    }
}
