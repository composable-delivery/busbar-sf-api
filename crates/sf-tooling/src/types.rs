//! Types for Salesforce Tooling API.

use serde::{Deserialize, Serialize};

// ============================================================================
// Execute Anonymous Types
// ============================================================================

/// Result of executing anonymous Apex.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecuteAnonymousResult {
    /// Whether the code compiled successfully.
    #[serde(default)]
    pub compiled: bool,

    /// Line number of compilation error (if any).
    #[serde(rename = "compileProblem")]
    pub compile_problem: Option<String>,

    /// Whether the execution was successful.
    #[serde(default)]
    pub success: bool,

    /// Line number where exception occurred.
    #[serde(rename = "exceptionStackTrace")]
    pub exception_stack_trace: Option<String>,

    /// Exception message.
    #[serde(rename = "exceptionMessage")]
    pub exception_message: Option<String>,

    /// The column number of the error.
    pub column: Option<i32>,

    /// The line number of the error.
    pub line: Option<i32>,
}

// ============================================================================
// Apex Class Types
// ============================================================================

/// ApexClass record from Tooling API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApexClass {
    #[serde(rename = "Id")]
    pub id: String,

    #[serde(rename = "Name")]
    pub name: String,

    #[serde(rename = "Body")]
    pub body: Option<String>,

    #[serde(rename = "Status")]
    pub status: Option<String>,

    #[serde(rename = "IsValid")]
    pub is_valid: Option<bool>,

    #[serde(rename = "ApiVersion")]
    pub api_version: Option<f64>,

    #[serde(rename = "LengthWithoutComments")]
    pub length_without_comments: Option<i32>,

    #[serde(rename = "NamespacePrefix")]
    pub namespace_prefix: Option<String>,

    #[serde(rename = "CreatedDate")]
    pub created_date: Option<String>,

    #[serde(rename = "LastModifiedDate")]
    pub last_modified_date: Option<String>,
}

/// ApexTrigger record from Tooling API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApexTrigger {
    #[serde(rename = "Id")]
    pub id: String,

    #[serde(rename = "Name")]
    pub name: String,

    #[serde(rename = "Body")]
    pub body: Option<String>,

    #[serde(rename = "Status")]
    pub status: Option<String>,

    #[serde(rename = "IsValid")]
    pub is_valid: Option<bool>,

    #[serde(rename = "ApiVersion")]
    pub api_version: Option<f64>,

    #[serde(rename = "TableEnumOrId")]
    pub table_enum_or_id: Option<String>,

    #[serde(rename = "UsageBeforeInsert")]
    pub usage_before_insert: Option<bool>,

    #[serde(rename = "UsageAfterInsert")]
    pub usage_after_insert: Option<bool>,

    #[serde(rename = "UsageBeforeUpdate")]
    pub usage_before_update: Option<bool>,

    #[serde(rename = "UsageAfterUpdate")]
    pub usage_after_update: Option<bool>,

    #[serde(rename = "UsageBeforeDelete")]
    pub usage_before_delete: Option<bool>,

    #[serde(rename = "UsageAfterDelete")]
    pub usage_after_delete: Option<bool>,

    #[serde(rename = "UsageAfterUndelete")]
    pub usage_after_undelete: Option<bool>,
}

// ============================================================================
// Debug Log Types
// ============================================================================

/// ApexLog record from Tooling API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApexLog {
    #[serde(rename = "Id")]
    pub id: String,

    #[serde(rename = "LogUser")]
    pub log_user: Option<LogUser>,

    #[serde(rename = "LogUserId")]
    pub log_user_id: Option<String>,

    #[serde(rename = "LogLength")]
    pub log_length: Option<i64>,

    #[serde(rename = "LastModifiedDate")]
    pub last_modified_date: Option<String>,

    #[serde(rename = "StartTime")]
    pub start_time: Option<String>,

    #[serde(rename = "Status")]
    pub status: Option<String>,

    #[serde(rename = "Operation")]
    pub operation: Option<String>,

    #[serde(rename = "Request")]
    pub request: Option<String>,

    #[serde(rename = "Application")]
    pub application: Option<String>,

    #[serde(rename = "DurationMilliseconds")]
    pub duration_milliseconds: Option<i64>,

    #[serde(rename = "Location")]
    pub location: Option<String>,
}

/// LogUser reference.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogUser {
    #[serde(rename = "Name")]
    pub name: Option<String>,
}

// ============================================================================
// Trace Flag Types
// ============================================================================

/// TraceFlag record from Tooling API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TraceFlag {
    #[serde(rename = "Id")]
    pub id: Option<String>,

    #[serde(rename = "TracedEntityId")]
    pub traced_entity_id: String,

    #[serde(rename = "LogType")]
    pub log_type: String,

    #[serde(rename = "DebugLevelId")]
    pub debug_level_id: String,

    #[serde(rename = "StartDate")]
    pub start_date: Option<String>,

    #[serde(rename = "ExpirationDate")]
    pub expiration_date: Option<String>,
}

/// DebugLevel record from Tooling API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DebugLevel {
    #[serde(rename = "Id")]
    pub id: Option<String>,

    #[serde(rename = "DeveloperName")]
    pub developer_name: String,

    #[serde(rename = "MasterLabel")]
    pub master_label: String,

    #[serde(rename = "ApexCode")]
    pub apex_code: Option<String>,

    #[serde(rename = "ApexProfiling")]
    pub apex_profiling: Option<String>,

    #[serde(rename = "Callout")]
    pub callout: Option<String>,

    #[serde(rename = "Database")]
    pub database: Option<String>,

    #[serde(rename = "System")]
    pub system: Option<String>,

    #[serde(rename = "Validation")]
    pub validation: Option<String>,

    #[serde(rename = "Visualforce")]
    pub visualforce: Option<String>,

    #[serde(rename = "Workflow")]
    pub workflow: Option<String>,
}

/// Debug log level options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    None,
    Error,
    Warn,
    Info,
    Debug,
    Fine,
    Finer,
    Finest,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::None => write!(f, "NONE"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Fine => write!(f, "FINE"),
            LogLevel::Finer => write!(f, "FINER"),
            LogLevel::Finest => write!(f, "FINEST"),
        }
    }
}

// ============================================================================
// Code Coverage Types
// ============================================================================

/// ApexCodeCoverage record from Tooling API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApexCodeCoverage {
    #[serde(rename = "Id")]
    pub id: String,

    #[serde(rename = "ApexClassOrTriggerId")]
    pub apex_class_or_trigger_id: String,

    #[serde(rename = "ApexClassOrTrigger")]
    pub apex_class_or_trigger: Option<ApexClassOrTriggerRef>,

    #[serde(rename = "TestMethodName")]
    pub test_method_name: Option<String>,

    #[serde(rename = "NumLinesCovered")]
    pub num_lines_covered: Option<i32>,

    #[serde(rename = "NumLinesUncovered")]
    pub num_lines_uncovered: Option<i32>,

    #[serde(rename = "Coverage")]
    pub coverage: Option<CoverageDetail>,
}

/// Reference to ApexClass or ApexTrigger.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApexClassOrTriggerRef {
    #[serde(rename = "Name")]
    pub name: Option<String>,
}

/// Coverage detail with covered and uncovered lines.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CoverageDetail {
    #[serde(rename = "coveredLines", default)]
    pub covered_lines: Vec<i32>,

    #[serde(rename = "uncoveredLines", default)]
    pub uncovered_lines: Vec<i32>,
}

/// ApexCodeCoverageAggregate record.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApexCodeCoverageAggregate {
    #[serde(rename = "Id")]
    pub id: String,

    #[serde(rename = "ApexClassOrTriggerId")]
    pub apex_class_or_trigger_id: String,

    #[serde(rename = "ApexClassOrTrigger")]
    pub apex_class_or_trigger: Option<ApexClassOrTriggerRef>,

    #[serde(rename = "NumLinesCovered")]
    pub num_lines_covered: i32,

    #[serde(rename = "NumLinesUncovered")]
    pub num_lines_uncovered: i32,

    #[serde(rename = "Coverage")]
    pub coverage: Option<CoverageDetail>,
}

// ============================================================================
// Test Execution Types
// ============================================================================

/// Request for running tests asynchronously.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunTestsAsyncRequest {
    /// Test class IDs to run.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "classids")]
    pub class_ids: Option<Vec<String>>,

    /// Test class names to run (alternative to class_ids).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "classNames")]
    pub class_names: Option<Vec<String>>,

    /// Test suite IDs to run.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "suiteids")]
    pub suite_ids: Option<Vec<String>>,

    /// Test suite names to run (alternative to suite_ids).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "suiteNames")]
    pub suite_names: Option<Vec<String>>,

    /// Maximum number of failed tests before stopping (-1 for no limit).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "maxFailedTests")]
    pub max_failed_tests: Option<i32>,

    /// Test level: RunSpecifiedTests, RunLocalTests, RunAllTestsInOrg, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "testLevel")]
    pub test_level: Option<String>,

    /// Whether to skip code coverage calculation.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "skipCodeCoverage")]
    pub skip_code_coverage: Option<bool>,
}

/// Response from running tests asynchronously (just the job ID).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RunTestsAsyncResponse {
    /// The AsyncApexJob ID.
    #[serde(rename = "jobId")]
    pub job_id: String,
}

/// Request for running tests synchronously.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunTestsSyncRequest {
    /// Apex class names or test methods to run.
    /// Format: "ClassName" or "ClassName.methodName"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests: Option<Vec<String>>,

    /// Whether to skip code coverage calculation.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "skipCodeCoverage")]
    pub skip_code_coverage: Option<bool>,
}

/// Result from running tests synchronously.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RunTestsSyncResult {
    /// Number of tests run.
    #[serde(rename = "numTestsRun")]
    pub num_tests_run: u32,

    /// Number of test failures.
    #[serde(rename = "numFailures")]
    pub num_failures: u32,

    /// Total execution time in milliseconds.
    #[serde(rename = "totalTime")]
    pub total_time: f64,

    /// Successful test results.
    #[serde(default)]
    pub successes: Vec<TestSuccess>,

    /// Failed test results.
    #[serde(default)]
    pub failures: Vec<TestFailure>,

    /// Code coverage results.
    #[serde(rename = "codeCoverage", default)]
    pub code_coverage: Vec<CodeCoverageResult>,

    /// Code coverage warnings.
    #[serde(rename = "codeCoverageWarnings", default)]
    pub code_coverage_warnings: Vec<CodeCoverageWarning>,
}

/// Successful test result.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestSuccess {
    /// Test class ID.
    #[serde(rename = "Id")]
    pub id: String,

    /// Test method name.
    #[serde(rename = "MethodName")]
    pub method_name: String,

    /// Test class name.
    #[serde(rename = "Name")]
    pub name: String,

    /// Namespace prefix.
    #[serde(rename = "NamespacePrefix")]
    pub namespace_prefix: Option<String>,

    /// Execution time in milliseconds.
    #[serde(rename = "Time")]
    pub time: f64,
}

/// Failed test result.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestFailure {
    /// Test class ID.
    #[serde(rename = "Id")]
    pub id: String,

    /// Test method name.
    #[serde(rename = "MethodName")]
    pub method_name: String,

    /// Test class name.
    #[serde(rename = "Name")]
    pub name: String,

    /// Namespace prefix.
    #[serde(rename = "NamespacePrefix")]
    pub namespace_prefix: Option<String>,

    /// Execution time in milliseconds.
    #[serde(rename = "Time")]
    pub time: f64,

    /// Error message.
    #[serde(rename = "Message")]
    pub message: String,

    /// Stack trace.
    #[serde(rename = "StackTrace")]
    pub stack_trace: Option<String>,

    /// Type of failure (e.g., "Class", "Method").
    #[serde(rename = "Type")]
    pub failure_type: Option<String>,
}

/// Code coverage result for a single class or trigger.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CodeCoverageResult {
    /// ID of the class or trigger.
    #[serde(rename = "id")]
    pub id: String,

    /// Name of the class or trigger.
    #[serde(rename = "name")]
    pub name: String,

    /// Namespace prefix.
    #[serde(rename = "namespace")]
    pub namespace: Option<String>,

    /// Number of lines covered.
    #[serde(rename = "numLocations")]
    pub num_locations: u32,

    /// Number of lines not covered.
    #[serde(rename = "numLocationsNotCovered")]
    pub num_locations_not_covered: u32,

    /// Type: "Class" or "Trigger".
    #[serde(rename = "type")]
    pub coverage_type: String,

    /// Covered lines.
    #[serde(rename = "locationsNotCovered", default)]
    pub locations_not_covered: Vec<CodeLocation>,
}

/// Code location (line number and column).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CodeLocation {
    /// Line number.
    pub line: u32,

    /// Column number.
    pub column: u32,

    /// Number of times executed (0 for not covered).
    #[serde(rename = "numExecutions")]
    pub num_executions: u32,

    /// Execution time in nanoseconds.
    pub time: f64,
}

/// Code coverage warning.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CodeCoverageWarning {
    /// Warning message.
    pub message: String,

    /// Name of the class or trigger.
    pub name: Option<String>,

    /// Namespace prefix.
    pub namespace: Option<String>,
}

// ============================================================================
// Test Discovery Types (v65.0+)
// ============================================================================

/// Result from Test Discovery API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestDiscoveryResult {
    /// List of available tests.
    pub tests: Vec<TestItem>,
}

/// A single test item (Apex test or Flow test).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestItem {
    /// Test ID.
    pub id: String,

    /// Test method or flow name.
    pub name: String,

    /// Class name (for Apex tests).
    #[serde(rename = "className")]
    pub class_name: Option<String>,

    /// Namespace prefix.
    pub namespace: Option<String>,

    /// Test category: "apex" or "flow".
    pub category: String,
}

// ============================================================================
// Test Runner Types (v65.0+)
// ============================================================================

/// Request for the unified Test Runner API (v65.0+).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunTestsRequest {
    /// Test class IDs to run.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "classIds")]
    pub class_ids: Option<Vec<String>>,

    /// Suite IDs to run.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "suiteIds")]
    pub suite_ids: Option<Vec<String>>,

    /// Test IDs to run (from Test Discovery).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "testIds")]
    pub test_ids: Option<Vec<String>>,

    /// Test level: "RunSpecifiedTests", "RunLocalTests", "RunAllTestsInOrg".
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "testLevel")]
    pub test_level: Option<String>,

    /// Whether to skip code coverage calculation.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "skipCodeCoverage")]
    pub skip_code_coverage: Option<bool>,

    /// Maximum number of failed tests before stopping.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "maxFailedTests")]
    pub max_failed_tests: Option<i32>,
}

/// Response from the unified Test Runner API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RunTestsResponse {
    /// Test run ID.
    #[serde(rename = "testRunId")]
    pub test_run_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_anonymous_result_deser() {
        let json = r#"{
            "compiled": true,
            "success": true
        }"#;

        let result: ExecuteAnonymousResult = serde_json::from_str(json).unwrap();
        assert!(result.compiled);
        assert!(result.success);
    }

    #[test]
    fn test_apex_class_deser() {
        let json = r#"{
            "Id": "01pxx00000000001AAA",
            "Name": "TestClass",
            "Status": "Active",
            "IsValid": true,
            "ApiVersion": 62.0
        }"#;

        let class: ApexClass = serde_json::from_str(json).unwrap();
        assert_eq!(class.name, "TestClass");
        assert_eq!(class.is_valid, Some(true));
        assert_eq!(class.api_version, Some(62.0));
    }

    #[test]
    fn test_apex_log_deser() {
        let json = r#"{
            "Id": "07Lxx00000000001AAA",
            "LogLength": 12345,
            "Status": "Success",
            "Operation": "Apex",
            "DurationMilliseconds": 150
        }"#;

        let log: ApexLog = serde_json::from_str(json).unwrap();
        assert_eq!(log.id, "07Lxx00000000001AAA");
        assert_eq!(log.log_length, Some(12345));
        assert_eq!(log.duration_milliseconds, Some(150));
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Debug.to_string(), "DEBUG");
        assert_eq!(LogLevel::Finest.to_string(), "FINEST");
    }

    #[test]
    fn test_run_tests_async_request_ser() {
        let req = RunTestsAsyncRequest {
            class_names: Some(vec!["TestClass1".to_string(), "TestClass2".to_string()]),
            test_level: Some("RunSpecifiedTests".to_string()),
            skip_code_coverage: Some(false),
            ..Default::default()
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("classNames"));
        assert!(json.contains("TestClass1"));
    }

    #[test]
    fn test_run_tests_async_response_deser() {
        let json = r#"{"jobId": "707xx00000000001AAA"}"#;
        let resp: RunTestsAsyncResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.job_id, "707xx00000000001AAA");
    }

    #[test]
    fn test_run_tests_sync_result_deser() {
        let json = r#"{
            "numTestsRun": 5,
            "numFailures": 1,
            "totalTime": 1234.5,
            "successes": [],
            "failures": [{
                "Id": "01pxx00000000001",
                "MethodName": "testMethod1",
                "Name": "TestClass",
                "Time": 100.0,
                "Message": "Assertion failed"
            }],
            "codeCoverage": [],
            "codeCoverageWarnings": []
        }"#;

        let result: RunTestsSyncResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.num_tests_run, 5);
        assert_eq!(result.num_failures, 1);
        assert_eq!(result.failures.len(), 1);
        assert_eq!(result.failures[0].method_name, "testMethod1");
    }

    #[test]
    fn test_test_discovery_result_deser() {
        let json = r#"{
            "tests": [
                {
                    "id": "01pxx00000000001",
                    "name": "testMethod1",
                    "className": "TestClass",
                    "namespace": null,
                    "category": "apex"
                },
                {
                    "id": "300xx00000000001",
                    "name": "MyFlowTest",
                    "namespace": null,
                    "category": "flow"
                }
            ]
        }"#;

        let result: TestDiscoveryResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.tests.len(), 2);
        assert_eq!(result.tests[0].category, "apex");
        assert_eq!(result.tests[1].category, "flow");
    }

    #[test]
    fn test_run_tests_request_ser() {
        let req = RunTestsRequest {
            test_ids: Some(vec!["01pxx00000000001".to_string()]),
            test_level: Some("RunSpecifiedTests".to_string()),
            skip_code_coverage: Some(true),
            ..Default::default()
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("testIds"));
        assert!(json.contains("skipCodeCoverage"));
    }
}
