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
    HardDelete,
    /// Query records
    Query,
    /// Query all records including deleted
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

/// Request to create an ingest job.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIngestJobRequest {
    /// SObject API name
    pub object: String,
    /// Operation type
    pub operation: String,
    /// External ID field for upsert
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id_field_name: Option<String>,
    /// Content type
    pub content_type: String,
    /// Column delimiter
    pub column_delimiter: String,
    /// Line ending
    pub line_ending: String,
}

impl CreateIngestJobRequest {
    /// Create a new ingest job request.
    pub fn new(sobject: impl Into<String>, operation: BulkOperation) -> Self {
        Self {
            object: sobject.into(),
            operation: operation.api_name().to_string(),
            external_id_field_name: None,
            content_type: "CSV".to_string(),
            column_delimiter: ColumnDelimiter::default().api_name().to_string(),
            line_ending: "LF".to_string(),
        }
    }

    /// Set the external ID field for upsert operations.
    pub fn with_external_id_field(mut self, field: impl Into<String>) -> Self {
        self.external_id_field_name = Some(field.into());
        self
    }

    /// Set the column delimiter.
    pub fn with_column_delimiter(mut self, delimiter: ColumnDelimiter) -> Self {
        self.column_delimiter = delimiter.api_name().to_string();
        self
    }

    /// Set the line ending.
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = match line_ending {
            LineEnding::Lf => "LF",
            LineEnding::Crlf => "CRLF",
        }
        .to_string();
        self
    }
}

/// Request to create a query job.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateQueryJobRequest {
    /// SOQL query
    pub query: String,
    /// Operation type (query or queryAll)
    pub operation: String,
    /// Column delimiter
    pub column_delimiter: String,
    /// Line ending
    pub line_ending: String,
}

impl CreateQueryJobRequest {
    /// Create a new query job request.
    pub fn new(soql: impl Into<String>) -> Self {
        Self {
            query: soql.into(),
            operation: BulkOperation::Query.api_name().to_string(),
            column_delimiter: ColumnDelimiter::default().api_name().to_string(),
            line_ending: "LF".to_string(),
        }
    }

    /// Use queryAll instead of query (includes deleted records).
    pub fn with_query_all(mut self) -> Self {
        self.operation = BulkOperation::QueryAll.api_name().to_string();
        self
    }

    /// Set the column delimiter.
    pub fn with_column_delimiter(mut self, delimiter: ColumnDelimiter) -> Self {
        self.column_delimiter = delimiter.api_name().to_string();
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
        assert_eq!(request.operation, "insert");
        assert!(request.external_id_field_name.is_none());
    }

    #[test]
    fn test_create_query_job_request() {
        let request = CreateQueryJobRequest::new("SELECT Id, Name FROM Account");
        assert_eq!(request.operation, "query");
        assert!(request.query.contains("Account"));
    }
}
