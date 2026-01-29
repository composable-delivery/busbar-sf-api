//! Types for Bulk API 2.0.

use serde::{Deserialize, Deserializer, Serialize};

/// Deserialize API version that can be either a float (59.0) or string ("59.0").
pub(crate) fn deserialize_api_version<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ApiVersion {
        Float(f64),
        String(String),
    }

    Option::<ApiVersion>::deserialize(deserializer).map(|opt| {
        opt.map(|v| match v {
            ApiVersion::Float(f) => format!("{:.1}", f),
            ApiVersion::String(s) => s,
        })
    })
}

/// Bulk API 2.0 job states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobState {
    /// Job is open and accepting data
    Open,
    /// Upload is complete, job is ready for processing
    UploadComplete,
    /// Job is processing
    InProgress,
    /// Job was aborted
    Aborted,
    /// Job completed successfully
    JobComplete,
    /// Job failed
    Failed,
}

impl JobState {
    /// Check if job is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            JobState::Aborted | JobState::JobComplete | JobState::Failed
        )
    }

    /// Check if job completed successfully.
    pub fn is_success(&self) -> bool {
        matches!(self, JobState::JobComplete)
    }
}

/// Bulk API 2.0 operation types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BulkOperation {
    /// Insert new records
    Insert,
    /// Update existing records
    Update,
    /// Upsert based on external ID
    Upsert,
    /// Delete records (soft delete)
    Delete,
    /// Hard delete records (permanent)
    #[serde(rename = "hardDelete")]
    HardDelete,
    /// Query records
    Query,
    /// Query all records including deleted
    #[serde(rename = "queryAll")]
    QueryAll,
}

impl BulkOperation {
    /// Get the API string for this operation.
    pub fn api_name(&self) -> &'static str {
        match self {
            BulkOperation::Insert => "insert",
            BulkOperation::Update => "update",
            BulkOperation::Upsert => "upsert",
            BulkOperation::Delete => "delete",
            BulkOperation::HardDelete => "hardDelete",
            BulkOperation::Query => "query",
            BulkOperation::QueryAll => "queryAll",
        }
    }

    /// Check if this is a query operation.
    pub fn is_query(&self) -> bool {
        matches!(self, BulkOperation::Query | BulkOperation::QueryAll)
    }

    /// Check if this is an ingest operation.
    pub fn is_ingest(&self) -> bool {
        !self.is_query()
    }
}

/// Content type for Bulk API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ContentType {
    #[default]
    #[serde(rename = "CSV")]
    Csv,
}

/// Line ending style for Bulk API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LineEnding {
    /// Unix-style (LF)
    #[default]
    #[serde(rename = "LF")]
    Lf,
    /// Windows-style (CRLF)
    #[serde(rename = "CRLF")]
    Crlf,
}

/// Column delimiter for Bulk API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ColumnDelimiter {
    #[default]
    #[serde(rename = "COMMA")]
    Comma,
    #[serde(rename = "TAB")]
    Tab,
    #[serde(rename = "SEMICOLON")]
    Semicolon,
    #[serde(rename = "PIPE")]
    Pipe,
    #[serde(rename = "BACKQUOTE")]
    Backquote,
    #[serde(rename = "CARET")]
    Caret,
}

impl ColumnDelimiter {
    /// Get the API string for this delimiter.
    pub fn api_name(&self) -> &'static str {
        match self {
            ColumnDelimiter::Comma => "COMMA",
            ColumnDelimiter::Tab => "TAB",
            ColumnDelimiter::Semicolon => "SEMICOLON",
            ColumnDelimiter::Pipe => "PIPE",
            ColumnDelimiter::Backquote => "BACKQUOTE",
            ColumnDelimiter::Caret => "CARET",
        }
    }

    /// Get the actual delimiter character.
    pub fn char(&self) -> char {
        match self {
            ColumnDelimiter::Comma => ',',
            ColumnDelimiter::Tab => '\t',
            ColumnDelimiter::Semicolon => ';',
            ColumnDelimiter::Pipe => '|',
            ColumnDelimiter::Backquote => '`',
            ColumnDelimiter::Caret => '^',
        }
    }
}

// =============================================================================
// Request Types
// =============================================================================

/// Request to update job state (close or abort).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateJobStateRequest {
    /// New state for the job
    pub state: JobState,
}

impl UpdateJobStateRequest {
    /// Create request to mark job as upload complete.
    pub fn upload_complete() -> Self {
        Self {
            state: JobState::UploadComplete,
        }
    }

    /// Create request to abort a job.
    pub fn abort() -> Self {
        Self {
            state: JobState::Aborted,
        }
    }
}

/// Request to create an ingest job.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIngestJobRequest {
    /// SObject API name
    pub object: String,
    /// Operation type
    pub operation: BulkOperation,
    /// External ID field for upsert
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id_field_name: Option<String>,
    /// Content type
    pub content_type: ContentType,
    /// Column delimiter
    pub column_delimiter: ColumnDelimiter,
    /// Line ending
    pub line_ending: LineEnding,
}

impl CreateIngestJobRequest {
    /// Create a new ingest job request.
    pub fn new(sobject: impl Into<String>, operation: BulkOperation) -> Self {
        Self {
            object: sobject.into(),
            operation,
            external_id_field_name: None,
            content_type: ContentType::default(),
            column_delimiter: ColumnDelimiter::default(),
            line_ending: LineEnding::default(),
        }
    }

    /// Set the external ID field for upsert operations.
    pub fn with_external_id_field(mut self, field: impl Into<String>) -> Self {
        self.external_id_field_name = Some(field.into());
        self
    }

    /// Set the column delimiter.
    pub fn with_column_delimiter(mut self, delimiter: ColumnDelimiter) -> Self {
        self.column_delimiter = delimiter;
        self
    }

    /// Set the line ending.
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }
}

/// Request to create a query job (internal use only).
///
/// This type is not exposed publicly to prevent bypassing SOQL injection prevention.
/// Use `BulkApiClient::execute_query()` with QueryBuilder instead.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateQueryJobRequest {
    /// SOQL query
    pub query: String,
    /// Operation type (query or queryAll)
    pub operation: BulkOperation,
    /// Column delimiter
    pub column_delimiter: ColumnDelimiter,
    /// Line ending
    pub line_ending: LineEnding,
}

impl CreateQueryJobRequest {
    /// Create a new query job request (internal).
    pub(crate) fn new(soql: impl Into<String>) -> Self {
        Self {
            query: soql.into(),
            operation: BulkOperation::Query,
            column_delimiter: ColumnDelimiter::default(),
            line_ending: LineEnding::default(),
        }
    }

    /// Use queryAll instead of query (includes deleted records).
    #[allow(dead_code)]
    pub(crate) fn with_query_all(mut self) -> Self {
        self.operation = BulkOperation::QueryAll;
        self
    }

    /// Set the column delimiter.
    #[allow(dead_code)]
    pub(crate) fn with_column_delimiter(mut self, delimiter: ColumnDelimiter) -> Self {
        self.column_delimiter = delimiter;
        self
    }
}

// =============================================================================
// Response Types
// =============================================================================

/// Ingest job response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestJob {
    /// Job ID
    pub id: String,
    /// Current state
    pub state: JobState,
    /// SObject API name
    pub object: String,
    /// Operation type
    pub operation: String,
    /// Number of records processed
    #[serde(default)]
    pub number_records_processed: i64,
    /// Number of records failed
    #[serde(default)]
    pub number_records_failed: i64,
    /// Job creation time
    #[serde(default)]
    pub created_date: Option<String>,
    /// Job completion time
    #[serde(default)]
    pub system_modstamp: Option<String>,
    /// Total processing time in milliseconds
    #[serde(default)]
    pub total_processing_time: Option<i64>,
    /// API version (can be float like 59.0 or string like "59.0")
    #[serde(default, deserialize_with = "deserialize_api_version")]
    pub api_version: Option<String>,
    /// Concurrency mode
    #[serde(default)]
    pub concurrency_mode: Option<String>,
    /// Error message if failed
    #[serde(default)]
    pub error_message: Option<String>,
}

/// Query job response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryJob {
    /// Job ID
    pub id: String,
    /// Current state
    pub state: JobState,
    /// SOQL query
    #[serde(default)]
    pub query: Option<String>,
    /// Operation type
    pub operation: String,
    /// Number of records processed
    #[serde(default)]
    pub number_records_processed: i64,
    /// Job creation time
    #[serde(default)]
    pub created_date: Option<String>,
    /// Job completion time
    #[serde(default)]
    pub system_modstamp: Option<String>,
    /// Total processing time in milliseconds
    #[serde(default)]
    pub total_processing_time: Option<i64>,
    /// Error message if failed
    #[serde(default)]
    pub error_message: Option<String>,
}

/// List of ingest jobs response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestJobList {
    /// List of jobs
    pub records: Vec<IngestJob>,
    /// Whether there are more records
    #[serde(default)]
    pub done: bool,
    /// Next records URL (for pagination)
    #[serde(default)]
    pub next_records_url: Option<String>,
}

/// List of query jobs response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryJobList {
    /// List of jobs
    pub records: Vec<QueryJob>,
    /// Whether there are more records
    #[serde(default)]
    pub done: bool,
    /// Next records URL (for pagination)
    #[serde(default)]
    pub next_records_url: Option<String>,
}

/// Query results with pagination info.
#[derive(Debug, Clone)]
pub struct QueryResults {
    /// CSV data
    pub csv_data: String,
    /// Locator for next page (None if no more pages)
    pub locator: Option<String>,
}

// =============================================================================
// Result Types
// =============================================================================

/// Result of a completed ingest job.
#[derive(Debug, Clone)]
pub struct IngestJobResult {
    /// The completed job
    pub job: IngestJob,
    /// Successful records CSV (if any)
    pub successful_results: Option<String>,
    /// Failed records CSV (if any)
    pub failed_results: Option<String>,
}

impl IngestJobResult {
    /// Check if the job succeeded.
    pub fn is_success(&self) -> bool {
        self.job.state.is_success()
    }

    /// Get the success rate.
    pub fn success_rate(&self) -> f64 {
        let total = self.job.number_records_processed + self.job.number_records_failed;
        if total == 0 {
            return 1.0;
        }
        self.job.number_records_processed as f64 / total as f64
    }

    /// Check if there were any failures.
    pub fn has_failures(&self) -> bool {
        self.job.number_records_failed > 0
    }
}

/// Result of a completed query job.
#[derive(Debug, Clone)]
pub struct QueryJobResult {
    /// The completed job
    pub job: QueryJob,
    /// Query results CSV (if successful)
    pub results: Option<String>,
}

impl QueryJobResult {
    /// Check if the job succeeded.
    pub fn is_success(&self) -> bool {
        self.job.state.is_success()
    }

    /// Get the number of records returned.
    pub fn record_count(&self) -> i64 {
        self.job.number_records_processed
    }
}

// =============================================================================
// Metadata Component Dependency (Beta)
// =============================================================================

#[cfg(feature = "dependencies")]
pub use busbar_sf_client::MetadataComponentDependency;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_state_terminal() {
        assert!(!JobState::Open.is_terminal());
        assert!(!JobState::InProgress.is_terminal());
        assert!(JobState::JobComplete.is_terminal());
        assert!(JobState::Failed.is_terminal());
        assert!(JobState::Aborted.is_terminal());
    }

    #[test]
    fn test_bulk_operation_types() {
        assert!(BulkOperation::Query.is_query());
        assert!(BulkOperation::QueryAll.is_query());
        assert!(BulkOperation::Insert.is_ingest());
        assert!(BulkOperation::Delete.is_ingest());
    }

    #[test]
    fn test_create_ingest_job_request() {
        let request = CreateIngestJobRequest::new("Account", BulkOperation::Insert);
        assert_eq!(request.object, "Account");
        assert_eq!(request.operation, BulkOperation::Insert);
        assert!(request.external_id_field_name.is_none());
    }

    #[test]
    fn test_create_query_job_request() {
        let request = CreateQueryJobRequest::new("SELECT Id, Name FROM Account");
        assert_eq!(request.operation, BulkOperation::Query);
        assert!(request.query.contains("Account"));
    }

    #[test]
    fn test_update_job_state_request() {
        let upload_complete = UpdateJobStateRequest::upload_complete();
        assert_eq!(upload_complete.state, JobState::UploadComplete);

        let abort = UpdateJobStateRequest::abort();
        assert_eq!(abort.state, JobState::Aborted);
    }

    #[test]
    fn test_bulk_operation_serialization() {
        // Test that enum variants serialize correctly to match Salesforce API
        assert_eq!(
            serde_json::to_string(&BulkOperation::Insert).unwrap(),
            "\"insert\""
        );
        assert_eq!(
            serde_json::to_string(&BulkOperation::Update).unwrap(),
            "\"update\""
        );
        assert_eq!(
            serde_json::to_string(&BulkOperation::Upsert).unwrap(),
            "\"upsert\""
        );
        assert_eq!(
            serde_json::to_string(&BulkOperation::Delete).unwrap(),
            "\"delete\""
        );
        assert_eq!(
            serde_json::to_string(&BulkOperation::HardDelete).unwrap(),
            "\"hardDelete\""
        );
        assert_eq!(
            serde_json::to_string(&BulkOperation::Query).unwrap(),
            "\"query\""
        );
        assert_eq!(
            serde_json::to_string(&BulkOperation::QueryAll).unwrap(),
            "\"queryAll\""
        );
    }

    #[test]
    fn test_bulk_operation_api_name() {
        assert_eq!(BulkOperation::Insert.api_name(), "insert");
        assert_eq!(BulkOperation::Update.api_name(), "update");
        assert_eq!(BulkOperation::Upsert.api_name(), "upsert");
        assert_eq!(BulkOperation::Delete.api_name(), "delete");
        assert_eq!(BulkOperation::HardDelete.api_name(), "hardDelete");
        assert_eq!(BulkOperation::Query.api_name(), "query");
        assert_eq!(BulkOperation::QueryAll.api_name(), "queryAll");
    }

    #[test]
    fn test_job_state_success() {
        assert!(JobState::JobComplete.is_success());
        assert!(!JobState::Failed.is_success());
        assert!(!JobState::Aborted.is_success());
        assert!(!JobState::Open.is_success());
        assert!(!JobState::InProgress.is_success());
    }

    #[test]
    fn test_column_delimiter_char() {
        assert_eq!(ColumnDelimiter::Comma.char(), ',');
        assert_eq!(ColumnDelimiter::Tab.char(), '\t');
        assert_eq!(ColumnDelimiter::Semicolon.char(), ';');
        assert_eq!(ColumnDelimiter::Pipe.char(), '|');
        assert_eq!(ColumnDelimiter::Backquote.char(), '`');
        assert_eq!(ColumnDelimiter::Caret.char(), '^');
    }

    #[test]
    fn test_column_delimiter_api_name() {
        assert_eq!(ColumnDelimiter::Comma.api_name(), "COMMA");
        assert_eq!(ColumnDelimiter::Tab.api_name(), "TAB");
        assert_eq!(ColumnDelimiter::Semicolon.api_name(), "SEMICOLON");
        assert_eq!(ColumnDelimiter::Pipe.api_name(), "PIPE");
        assert_eq!(ColumnDelimiter::Backquote.api_name(), "BACKQUOTE");
        assert_eq!(ColumnDelimiter::Caret.api_name(), "CARET");
    }

    #[test]
    fn test_ingest_job_request_with_builder() {
        let request = CreateIngestJobRequest::new("Account", BulkOperation::Upsert)
            .with_external_id_field("External_Id__c")
            .with_column_delimiter(ColumnDelimiter::Pipe)
            .with_line_ending(LineEnding::Crlf);

        assert_eq!(request.object, "Account");
        assert_eq!(request.operation, BulkOperation::Upsert);
        assert_eq!(
            request.external_id_field_name,
            Some("External_Id__c".to_string())
        );
        assert_eq!(request.column_delimiter, ColumnDelimiter::Pipe);
        assert_eq!(request.line_ending, LineEnding::Crlf);
    }

    #[test]
    fn test_ingest_job_deserialization() {
        let json = serde_json::json!({
            "id": "7508000000Gxxx",
            "state": "JobComplete",
            "object": "Account",
            "operation": "insert",
            "numberRecordsProcessed": 100,
            "numberRecordsFailed": 2,
            "createdDate": "2024-01-15T10:30:00.000Z",
            "totalProcessingTime": 5000,
            "apiVersion": 62.0,
            "concurrencyMode": "Parallel"
        });

        let job: IngestJob = serde_json::from_value(json).unwrap();
        assert_eq!(job.id, "7508000000Gxxx");
        assert_eq!(job.state, JobState::JobComplete);
        assert_eq!(job.object, "Account");
        assert_eq!(job.number_records_processed, 100);
        assert_eq!(job.number_records_failed, 2);
        // API version should be deserialized from float to string
        assert_eq!(job.api_version, Some("62.0".to_string()));
    }

    #[test]
    fn test_ingest_job_api_version_as_string() {
        // Some Salesforce responses return apiVersion as a string
        let json = serde_json::json!({
            "id": "750xx",
            "state": "Open",
            "object": "Contact",
            "operation": "update",
            "apiVersion": "62.0"
        });

        let job: IngestJob = serde_json::from_value(json).unwrap();
        assert_eq!(job.api_version, Some("62.0".to_string()));
    }

    #[test]
    fn test_query_job_deserialization() {
        let json = serde_json::json!({
            "id": "750xx",
            "state": "JobComplete",
            "operation": "query",
            "query": "SELECT Id, Name FROM Account",
            "numberRecordsProcessed": 500
        });

        let job: QueryJob = serde_json::from_value(json).unwrap();
        assert_eq!(job.id, "750xx");
        assert_eq!(job.state, JobState::JobComplete);
        assert_eq!(job.query, Some("SELECT Id, Name FROM Account".to_string()));
        assert_eq!(job.number_records_processed, 500);
    }

    #[test]
    fn test_ingest_job_result_success_rate() {
        let result = IngestJobResult {
            job: IngestJob {
                id: "750xx".to_string(),
                state: JobState::JobComplete,
                object: "Account".to_string(),
                operation: "insert".to_string(),
                number_records_processed: 98,
                number_records_failed: 2,
                created_date: None,
                system_modstamp: None,
                total_processing_time: None,
                api_version: None,
                concurrency_mode: None,
                error_message: None,
            },
            successful_results: Some("Id,Name\n001xx,Acme".to_string()),
            failed_results: Some("Id,Error\n,DUPLICATE".to_string()),
        };

        assert!(result.is_success());
        assert!(result.has_failures());
        assert!((result.success_rate() - 0.98).abs() < 0.001);
    }

    #[test]
    fn test_ingest_job_result_no_records() {
        let result = IngestJobResult {
            job: IngestJob {
                id: "750xx".to_string(),
                state: JobState::JobComplete,
                object: "Account".to_string(),
                operation: "insert".to_string(),
                number_records_processed: 0,
                number_records_failed: 0,
                created_date: None,
                system_modstamp: None,
                total_processing_time: None,
                api_version: None,
                concurrency_mode: None,
                error_message: None,
            },
            successful_results: None,
            failed_results: None,
        };

        // When no records processed, success_rate should be 1.0 (not NaN/panic)
        assert!((result.success_rate() - 1.0).abs() < 0.001);
        assert!(!result.has_failures());
    }

    #[test]
    fn test_query_job_result() {
        let result = QueryJobResult {
            job: QueryJob {
                id: "750xx".to_string(),
                state: JobState::JobComplete,
                operation: "query".to_string(),
                query: Some("SELECT Id FROM Account".to_string()),
                number_records_processed: 42,
                created_date: None,
                system_modstamp: None,
                total_processing_time: None,
                error_message: None,
            },
            results: Some("Id\n001xx\n002xx".to_string()),
        };

        assert!(result.is_success());
        assert_eq!(result.record_count(), 42);
    }

    #[test]
    fn test_ingest_job_list_deserialization() {
        let json = serde_json::json!({
            "records": [
                {"id": "750a", "state": "JobComplete", "object": "Account", "operation": "insert"},
                {"id": "750b", "state": "Failed", "object": "Contact", "operation": "update", "errorMessage": "Invalid data"}
            ],
            "done": true
        });

        let list: IngestJobList = serde_json::from_value(json).unwrap();
        assert!(list.done);
        assert_eq!(list.records.len(), 2);
        assert_eq!(list.records[0].state, JobState::JobComplete);
        assert_eq!(list.records[1].state, JobState::Failed);
        assert_eq!(
            list.records[1].error_message,
            Some("Invalid data".to_string())
        );
    }

    #[test]
    fn test_content_type_serialization() {
        assert_eq!(serde_json::to_string(&ContentType::Csv).unwrap(), "\"CSV\"");
    }

    #[test]
    fn test_line_ending_serialization() {
        assert_eq!(serde_json::to_string(&LineEnding::Lf).unwrap(), "\"LF\"");
        assert_eq!(
            serde_json::to_string(&LineEnding::Crlf).unwrap(),
            "\"CRLF\""
        );
    }

    #[test]
    fn test_column_delimiter_serialization() {
        assert_eq!(
            serde_json::to_string(&ColumnDelimiter::Comma).unwrap(),
            "\"COMMA\""
        );
        assert_eq!(
            serde_json::to_string(&ColumnDelimiter::Tab).unwrap(),
            "\"TAB\""
        );
        assert_eq!(
            serde_json::to_string(&ColumnDelimiter::Pipe).unwrap(),
            "\"PIPE\""
        );
    }

    #[test]
    fn test_ingest_job_request_serialization() {
        let request = CreateIngestJobRequest::new("Account", BulkOperation::Insert);
        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["object"], "Account");
        assert_eq!(json["operation"], "insert");
        assert_eq!(json["contentType"], "CSV");
        assert_eq!(json["columnDelimiter"], "COMMA");
        assert_eq!(json["lineEnding"], "LF");
        // external_id_field_name should be omitted when None
        assert!(json.get("externalIdFieldName").is_none());
    }

    #[test]
    fn test_upsert_job_request_serialization() {
        let request = CreateIngestJobRequest::new("Account", BulkOperation::Upsert)
            .with_external_id_field("External_Id__c");
        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["operation"], "upsert");
        assert_eq!(json["externalIdFieldName"], "External_Id__c");
    }
}
