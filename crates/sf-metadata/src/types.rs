//! Common types for Metadata API.

use serde::{Deserialize, Serialize};

/// Default Metadata API version.
pub const DEFAULT_API_VERSION: &str = "62.0";

/// Test level for deployments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TestLevel {
    /// No tests run.
    NoTestRun,
    /// Run local tests only.
    #[default]
    RunLocalTests,
    /// Run all tests in org.
    RunAllTestsInOrg,
    /// Run specified tests.
    RunSpecifiedTests,
}

impl std::fmt::Display for TestLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestLevel::NoTestRun => write!(f, "NoTestRun"),
            TestLevel::RunLocalTests => write!(f, "RunLocalTests"),
            TestLevel::RunAllTestsInOrg => write!(f, "RunAllTestsInOrg"),
            TestLevel::RunSpecifiedTests => write!(f, "RunSpecifiedTests"),
        }
    }
}

/// SOAP Fault from the Metadata API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoapFault {
    pub fault_code: String,
    pub fault_string: String,
}

impl std::fmt::Display for SoapFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SOAP Fault: {} - {}", self.fault_code, self.fault_string)
    }
}

impl std::error::Error for SoapFault {}

/// A component deployment success.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSuccess {
    pub component_type: Option<String>,
    pub file_name: Option<String>,
    pub full_name: Option<String>,
    pub created: bool,
    pub deleted: bool,
}

/// A test failure during deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFailure {
    pub name: Option<String>,
    pub method_name: Option<String>,
    pub message: Option<String>,
    pub stack_trace: Option<String>,
    pub namespace: Option<String>,
}

/// Properties of a file in a retrieve result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileProperties {
    pub created_by_id: String,
    pub created_by_name: String,
    pub created_date: String,
    pub file_name: String,
    pub full_name: String,
    pub id: String,
    pub last_modified_by_id: String,
    pub last_modified_by_name: String,
    pub last_modified_date: String,
    pub manageable_state: Option<String>,
    pub namespace_prefix: Option<String>,
    pub component_type: String,
}

/// Error information returned from Metadata API CRUD operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataError {
    /// Status code identifying the error type.
    pub status_code: String,
    /// Descriptive error message.
    pub message: String,
    /// Field names associated with the error.
    pub fields: Vec<String>,
}

/// Result of a save operation (create/update/rename).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveResult {
    /// Full name of the metadata component.
    pub full_name: String,
    /// Whether the operation succeeded.
    pub success: bool,
    /// Errors that occurred during the operation.
    pub errors: Vec<MetadataError>,
}

/// Result of an upsert operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertResult {
    /// Full name of the metadata component.
    pub full_name: String,
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether the component was created (true) or updated (false).
    pub created: bool,
    /// Errors that occurred during the operation.
    pub errors: Vec<MetadataError>,
}

/// Result of a delete operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResult {
    /// Full name of the metadata component.
    pub full_name: String,
    /// Whether the operation succeeded.
    pub success: bool,
    /// Errors that occurred during the operation.
    pub errors: Vec<MetadataError>,
}

/// Result of a read operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResult {
    /// The metadata components that were read.
    /// Each element is a metadata object with type-specific fields.
    pub records: Vec<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_level_display() {
        assert_eq!(TestLevel::NoTestRun.to_string(), "NoTestRun");
        assert_eq!(TestLevel::RunLocalTests.to_string(), "RunLocalTests");
        assert_eq!(TestLevel::RunAllTestsInOrg.to_string(), "RunAllTestsInOrg");
        assert_eq!(
            TestLevel::RunSpecifiedTests.to_string(),
            "RunSpecifiedTests"
        );
    }

    #[test]
    fn test_soap_fault_display() {
        let fault = SoapFault {
            fault_code: "sf:INVALID_SESSION_ID".to_string(),
            fault_string: "Invalid Session ID".to_string(),
        };
        assert!(fault.to_string().contains("INVALID_SESSION_ID"));
    }
}
