//! Types for Salesforce Tooling API.

use serde::{Deserialize, Deserializer, Serialize};

/// Deserialize a value that may be `null` or missing as `T::default()`.
///
/// Serde's `#[serde(default)]` only handles missing keys. This also handles
/// explicit `null` values from the Salesforce API (e.g., `"parameters": null`
/// instead of `"parameters": []`).
fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

// ============================================================================
// Search Types
// ============================================================================

/// Result of a SOSL search query.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResult<T> {
    /// The search results.
    #[serde(rename = "searchRecords")]
    pub search_records: Vec<T>,
}

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
///
/// The Salesforce `runTestsAsynchronous` endpoint accepts comma-separated
/// strings for `classids`, `classNames`, `suiteids`, and `suiteNames`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunTestsAsyncRequest {
    /// Comma-separated list of test class IDs to run.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "classids")]
    pub class_ids: Option<String>,

    /// Comma-separated list of test class names to run (alternative to class_ids).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "classNames")]
    pub class_names: Option<String>,

    /// Comma-separated list of test suite IDs to run.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "suiteids")]
    pub suite_ids: Option<String>,

    /// Comma-separated list of test suite names to run (alternative to suite_ids).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "suiteNames")]
    pub suite_names: Option<String>,

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

/// A test class descriptor for synchronous test execution.
///
/// Salesforce `runTestsSynchronous` expects objects with `className` and
/// optional `testMethods`, not plain class name strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTestItem {
    /// The Apex test class name (required).
    #[serde(rename = "className")]
    pub class_name: String,

    /// Specific test methods to run. If `None`, all test methods are executed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "testMethods")]
    pub test_methods: Option<Vec<String>>,

    /// Namespace prefix (for managed packages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

/// Request for running tests synchronously.
///
/// The `tests` field is an array of [`SyncTestItem`] objects containing
/// `className` and optional `testMethods`. Only one test class is allowed
/// per synchronous request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunTestsSyncRequest {
    /// Test class descriptors to run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests: Option<Vec<SyncTestItem>>,

    /// Maximum number of failed tests before stopping (-1 for no limit).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "maxFailedTests")]
    pub max_failed_tests: Option<i32>,

    /// Whether to skip code coverage calculation.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "skipCodeCoverage")]
    pub skip_code_coverage: Option<bool>,
}

/// Result from running tests synchronously.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct RunTestsSyncResult {
    #[serde(alias = "numTestsRun", default)]
    pub num_tests_run: u32,

    #[serde(alias = "numFailures", default)]
    pub num_failures: u32,

    #[serde(alias = "totalTime", default)]
    pub total_time: f64,

    #[serde(default)]
    pub successes: Vec<TestSuccess>,

    #[serde(default)]
    pub failures: Vec<TestFailure>,

    #[serde(alias = "codeCoverage", default)]
    pub code_coverage: Vec<CodeCoverageResult>,

    #[serde(alias = "codeCoverageWarnings", default)]
    pub code_coverage_warnings: Vec<CodeCoverageWarning>,
}

/// Successful test result.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct TestSuccess {
    #[serde(alias = "Id", alias = "id", default)]
    pub id: String,

    #[serde(alias = "MethodName", alias = "methodName", default)]
    pub method_name: String,

    #[serde(alias = "Name", alias = "name", default)]
    pub name: String,

    #[serde(alias = "NamespacePrefix", alias = "namespacePrefix")]
    pub namespace_prefix: Option<String>,

    #[serde(alias = "Time", alias = "time", default)]
    pub time: f64,
}

/// Failed test result.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct TestFailure {
    #[serde(alias = "Id", alias = "id", default)]
    pub id: String,

    #[serde(alias = "MethodName", alias = "methodName", default)]
    pub method_name: String,

    #[serde(alias = "Name", alias = "name", default)]
    pub name: String,

    #[serde(alias = "NamespacePrefix", alias = "namespacePrefix")]
    pub namespace_prefix: Option<String>,

    #[serde(alias = "Time", alias = "time", default)]
    pub time: f64,

    #[serde(alias = "Message", alias = "message", default)]
    pub message: String,

    #[serde(alias = "StackTrace", alias = "stackTrace")]
    pub stack_trace: Option<String>,

    #[serde(alias = "Type", rename = "type")]
    pub failure_type: Option<String>,
}

/// Code coverage result for a single class or trigger.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CodeCoverageResult {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub namespace: Option<String>,

    #[serde(alias = "numLocations", default)]
    pub num_locations: u32,

    #[serde(alias = "numLocationsNotCovered", default)]
    pub num_locations_not_covered: u32,

    #[serde(rename = "type", default)]
    pub coverage_type: String,

    #[serde(alias = "locationsNotCovered", default)]
    pub locations_not_covered: Vec<CodeLocation>,
}

/// Code location (line number and column).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CodeLocation {
    #[serde(default)]
    pub line: u32,
    #[serde(default)]
    pub column: u32,

    #[serde(alias = "numExecutions", default)]
    pub num_executions: u32,

    #[serde(default)]
    pub time: f64,
}

/// Code coverage warning.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CodeCoverageWarning {
    pub message: String,
    pub name: Option<String>,
    pub namespace: Option<String>,
}

// ============================================================================
// Test Discovery Types (v65.0+)
// ============================================================================

/// Result from Test Discovery API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestDiscoveryResult {
    pub tests: Vec<TestItem>,
}

/// A single test item (Apex test or Flow test).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestItem {
    pub id: String,
    pub name: String,

    #[serde(rename = "className")]
    pub class_name: Option<String>,

    pub namespace: Option<String>,
    pub category: String,
}

// ============================================================================
// Test Runner Types (v65.0+)
// ============================================================================

/// Request for the unified Test Runner API (v65.0+).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunTestsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "classIds")]
    pub class_ids: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "suiteIds")]
    pub suite_ids: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "testIds")]
    pub test_ids: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "testLevel")]
    pub test_level: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "skipCodeCoverage")]
    pub skip_code_coverage: Option<bool>,
}

/// Response from the unified Test Runner API (v65.0+).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RunTestsResponse {
    /// Test run ID for tracking results.
    #[serde(rename = "testRunId")]
    pub test_run_id: String,
}

// ============================================================================
// Code Intelligence Types
// ============================================================================

/// Result from the completions endpoint.
///
/// The Salesforce completions API returns `{ "publicDeclarations": { "ClassName": [...], ... } }`
/// where each key is a class/namespace name mapping to its members.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompletionsResult {
    #[serde(rename = "publicDeclarations")]
    pub public_declarations: std::collections::HashMap<String, Vec<CompletionItem>>,
}

/// A single completion item.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CompletionItem {
    pub name: String,

    #[serde(rename = "type")]
    pub symbol_type: Option<String>,

    pub namespace: Option<String>,

    pub signature: Option<String>,

    #[serde(rename = "returnType")]
    pub return_type: Option<String>,

    #[serde(default, deserialize_with = "null_as_default")]
    pub parameters: Vec<Parameter>,

    #[serde(default, deserialize_with = "null_as_default")]
    pub references: Vec<Reference>,
}

/// Method parameter information.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Parameter {
    pub name: String,

    #[serde(rename = "type")]
    pub param_type: Option<String>,
}

/// Reference to documentation or related types.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Reference {
    pub name: String,

    #[serde(rename = "type")]
    pub ref_type: Option<String>,
}

// ============================================================================
// Metadata Component Dependency (Beta)
// ============================================================================

#[cfg(feature = "dependencies")]
pub use busbar_sf_client::MetadataComponentDependency;

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
            class_names: Some("TestClass1".to_string()),
            test_level: Some("RunSpecifiedTests".to_string()),
            skip_code_coverage: Some(false),
            ..Default::default()
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("classNames"));
        assert!(json.contains("\"TestClass1\""));
        assert!(json.contains("testLevel"));
        assert!(json.contains("skipCodeCoverage"));
        assert!(!json.contains("classids"));
    }

    #[test]
    fn test_run_tests_async_request_multiple_classes() {
        let req = RunTestsAsyncRequest {
            class_names: Some("TestClass1,TestClass2,TestClass3".to_string()),
            test_level: Some("RunSpecifiedTests".to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("TestClass1,TestClass2,TestClass3"));
    }

    #[test]
    fn test_run_tests_sync_request_ser() {
        let req = RunTestsSyncRequest {
            tests: Some(vec![SyncTestItem {
                class_name: "MyTestClass".to_string(),
                test_methods: Some(vec!["testMethod1".to_string()]),
                namespace: None,
            }]),
            ..Default::default()
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("className"));
        assert!(json.contains("MyTestClass"));
        assert!(json.contains("testMethods"));
        assert!(json.contains("testMethod1"));
    }

    #[test]
    fn test_run_tests_sync_result_deser() {
        let json = r#"{
            "numTestsRun": 5,
            "numFailures": 1,
            "totalTime": 1234.5,
            "successes": [
                {
                    "Id": "01p001",
                    "MethodName": "testMethod1",
                    "Name": "TestClass1",
                    "NamespacePrefix": null,
                    "Time": 100.0
                }
            ],
            "failures": [
                {
                    "Id": "01p002",
                    "MethodName": "testFailing",
                    "Name": "TestClass2",
                    "NamespacePrefix": null,
                    "Time": 200.0,
                    "Message": "Assertion failed",
                    "StackTrace": "Class.TestClass2.testFailing: line 5"
                }
            ],
            "codeCoverage": [],
            "codeCoverageWarnings": []
        }"#;

        let result: RunTestsSyncResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.num_tests_run, 5);
        assert_eq!(result.num_failures, 1);
        assert_eq!(result.successes.len(), 1);
        assert_eq!(result.successes[0].method_name, "testMethod1");
        assert_eq!(result.failures.len(), 1);
        assert_eq!(result.failures[0].message, "Assertion failed");
    }

    #[test]
    fn test_test_discovery_result_deser() {
        let json = r#"{
            "tests": [
                {
                    "id": "01pxx00000000001",
                    "name": "testMethod1",
                    "className": "TestClass1",
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

    #[test]
    fn test_completions_result_deser() {
        let json = r#"{
            "publicDeclarations": {
                "System": [
                    {
                        "name": "debug",
                        "type": "Method",
                        "namespace": "System",
                        "returnType": "void",
                        "parameters": [
                            {
                                "name": "message",
                                "type": "Object"
                            }
                        ]
                    }
                ],
                "Database": [
                    {
                        "name": "query",
                        "type": "Method",
                        "namespace": "Database"
                    }
                ]
            }
        }"#;

        let result: CompletionsResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.public_declarations.len(), 2);
        assert!(result.public_declarations.contains_key("System"));
        assert!(result.public_declarations.contains_key("Database"));
        let system_items = &result.public_declarations["System"];
        assert_eq!(system_items.len(), 1);
        assert_eq!(system_items[0].name, "debug");
        assert_eq!(system_items[0].return_type, Some("void".to_string()));
        assert_eq!(system_items[0].parameters.len(), 1);
    }

    #[test]
    fn test_completions_result_null_fields() {
        // Salesforce API can return null for arrays and missing fields
        let json = r#"{
            "publicDeclarations": {
                "System": [
                    {
                        "name": "debug",
                        "parameters": null,
                        "references": null
                    },
                    {
                        "name": "assert"
                    }
                ]
            }
        }"#;

        let result: CompletionsResult = serde_json::from_str(json).unwrap();
        let items = &result.public_declarations["System"];
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "debug");
        assert!(items[0].parameters.is_empty());
        assert!(items[0].references.is_empty());
        assert_eq!(items[1].name, "assert");
        assert!(items[1].parameters.is_empty());
    }
}
