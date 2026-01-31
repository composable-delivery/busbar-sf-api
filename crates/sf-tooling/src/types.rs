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
// Code Intelligence Types
// ============================================================================

/// Result from the completions endpoint.
///
/// Contains code completion suggestions for Apex or Visualforce.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompletionsResult {
    /// List of completion items.
    #[serde(rename = "publicDeclarations")]
    pub public_declarations: PublicDeclarations,
}

/// Public declarations structure containing completion suggestions.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PublicDeclarations {
    /// List of public declarations (classes, methods, etc.).
    #[serde(rename = "publicDeclarations")]
    #[serde(default)]
    pub public_declarations: Vec<CompletionItem>,
}

/// A single completion item (class, method, property, etc.).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompletionItem {
    /// Name of the symbol.
    #[serde(rename = "name")]
    pub name: String,

    /// Type of the symbol (e.g., "Class", "Method", "Property").
    #[serde(rename = "type")]
    pub symbol_type: Option<String>,

    /// Namespace of the symbol.
    #[serde(rename = "namespace")]
    pub namespace: Option<String>,

    /// Signature or documentation.
    #[serde(rename = "signature")]
    pub signature: Option<String>,

    /// Return type for methods.
    #[serde(rename = "returnType")]
    pub return_type: Option<String>,

    /// Parameters for methods.
    #[serde(rename = "parameters")]
    #[serde(default)]
    pub parameters: Vec<Parameter>,

    /// References to related types or documentation.
    #[serde(rename = "references")]
    #[serde(default)]
    pub references: Vec<Reference>,
}

/// Method parameter information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Parameter {
    /// Parameter name.
    #[serde(rename = "name")]
    pub name: String,

    /// Parameter type.
    #[serde(rename = "type")]
    pub param_type: String,
}

/// Reference to documentation or related types.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Reference {
    /// Name of the reference.
    #[serde(rename = "name")]
    pub name: String,

    /// Type of reference.
    #[serde(rename = "type")]
    pub ref_type: Option<String>,
}

/// Result from the apex manifest endpoint.
///
/// Contains a complete listing of all Apex classes and triggers in the org,
/// including inner classes and global classes from managed packages.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApexManifestResult {
    /// List of Apex classes.
    #[serde(rename = "classes")]
    #[serde(default)]
    pub classes: Vec<ApexManifestClass>,

    /// List of Apex triggers.
    #[serde(rename = "triggers")]
    #[serde(default)]
    pub triggers: Vec<ApexManifestTrigger>,
}

/// Apex class information from the manifest.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApexManifestClass {
    /// Salesforce ID of the class.
    #[serde(rename = "id")]
    pub id: String,

    /// Name of the class.
    #[serde(rename = "name")]
    pub name: String,

    /// Namespace prefix (if any).
    #[serde(rename = "namespacePrefix")]
    pub namespace_prefix: Option<String>,

    /// Whether the class is valid (compiled successfully).
    #[serde(rename = "isValid")]
    pub is_valid: Option<bool>,

    /// Length of the class without comments.
    #[serde(rename = "lengthWithoutComments")]
    pub length_without_comments: Option<i32>,

    /// API version.
    #[serde(rename = "apiVersion")]
    pub api_version: Option<f64>,
}

/// Apex trigger information from the manifest.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApexManifestTrigger {
    /// Salesforce ID of the trigger.
    #[serde(rename = "id")]
    pub id: String,

    /// Name of the trigger.
    #[serde(rename = "name")]
    pub name: String,

    /// Namespace prefix (if any).
    #[serde(rename = "namespacePrefix")]
    pub namespace_prefix: Option<String>,

    /// Whether the trigger is valid.
    #[serde(rename = "isValid")]
    pub is_valid: Option<bool>,

    /// Entity on which the trigger operates.
    #[serde(rename = "tableEnumOrId")]
    pub table_enum_or_id: Option<String>,

    /// API version.
    #[serde(rename = "apiVersion")]
    pub api_version: Option<f64>,
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
    fn test_completions_result_deser() {
        let json = r#"{
            "publicDeclarations": {
                "publicDeclarations": [
                    {
                        "name": "System",
                        "type": "Class",
                        "namespace": null
                    },
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
                ]
            }
        }"#;

        let result: CompletionsResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.public_declarations.public_declarations.len(), 2);
        assert_eq!(
            result.public_declarations.public_declarations[0].name,
            "System"
        );
        assert_eq!(
            result.public_declarations.public_declarations[0].symbol_type,
            Some("Class".to_string())
        );
        assert_eq!(
            result.public_declarations.public_declarations[1].name,
            "debug"
        );
        assert_eq!(
            result.public_declarations.public_declarations[1].return_type,
            Some("void".to_string())
        );
        assert_eq!(
            result.public_declarations.public_declarations[1]
                .parameters
                .len(),
            1
        );
    }

    #[test]
    fn test_apex_manifest_result_deser() {
        let json = r#"{
            "classes": [
                {
                    "id": "01pxx00000000001AAA",
                    "name": "TestClass",
                    "namespacePrefix": null,
                    "isValid": true,
                    "lengthWithoutComments": 150,
                    "apiVersion": 62.0
                },
                {
                    "id": "01pxx00000000002AAA",
                    "name": "AnotherClass",
                    "namespacePrefix": "myns",
                    "isValid": true,
                    "apiVersion": 61.0
                }
            ],
            "triggers": [
                {
                    "id": "01qxx00000000001AAA",
                    "name": "AccountTrigger",
                    "namespacePrefix": null,
                    "isValid": true,
                    "tableEnumOrId": "Account",
                    "apiVersion": 62.0
                }
            ]
        }"#;

        let result: ApexManifestResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.classes.len(), 2);
        assert_eq!(result.classes[0].name, "TestClass");
        assert_eq!(result.classes[0].id, "01pxx00000000001AAA");
        assert_eq!(result.classes[0].is_valid, Some(true));
        assert_eq!(result.classes[0].length_without_comments, Some(150));
        assert_eq!(result.classes[1].name, "AnotherClass");
        assert_eq!(result.classes[1].namespace_prefix, Some("myns".to_string()));

        assert_eq!(result.triggers.len(), 1);
        assert_eq!(result.triggers[0].name, "AccountTrigger");
        assert_eq!(result.triggers[0].id, "01qxx00000000001AAA");
        assert_eq!(
            result.triggers[0].table_enum_or_id,
            Some("Account".to_string())
        );
    }
}
