//! Deploy operations.

use crate::types::{ComponentSuccess, TestFailure, TestLevel};
use serde::{Deserialize, Serialize};

/// Options for deployment.
#[derive(Debug, Clone)]
pub struct DeployOptions {
    /// Allow references to missing files in the zip.
    pub allow_missing_files: bool,
    /// Automatically update the package manifest.
    pub auto_update_package: bool,
    /// Validate only, don't actually deploy.
    pub check_only: bool,
    /// Ignore warnings during deployment.
    pub ignore_warnings: bool,
    /// Retrieve metadata after deploy.
    pub perform_retrieve: bool,
    /// Hard delete components (only in sandbox/DE orgs).
    pub purge_on_delete: bool,
    /// Rollback all changes if any component fails.
    pub rollback_on_error: bool,
    /// Run all Apex tests.
    pub run_all_tests: bool,
    /// Deploy as a single package.
    pub single_package: bool,
    /// Test level for deployment.
    pub test_level: Option<TestLevel>,
    /// Specific tests to run (when test_level is RunSpecifiedTests).
    pub run_tests: Vec<String>,
}

impl Default for DeployOptions {
    fn default() -> Self {
        Self {
            allow_missing_files: false,
            auto_update_package: false,
            check_only: false,
            ignore_warnings: true,
            perform_retrieve: false,
            purge_on_delete: false,
            rollback_on_error: true,
            run_all_tests: false,
            single_package: true,
            test_level: None,
            run_tests: vec![],
        }
    }
}

/// Deployment status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeployStatus {
    Pending,
    InProgress,
    Succeeded,
    SucceededPartial,
    Failed,
    Canceling,
    Canceled,
}

impl std::str::FromStr for DeployStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Pending" => Ok(DeployStatus::Pending),
            "InProgress" => Ok(DeployStatus::InProgress),
            "Succeeded" => Ok(DeployStatus::Succeeded),
            "SucceededPartial" => Ok(DeployStatus::SucceededPartial),
            "Failed" => Ok(DeployStatus::Failed),
            "Canceling" => Ok(DeployStatus::Canceling),
            "Canceled" => Ok(DeployStatus::Canceled),
            _ => Err(format!("Unknown deploy status: {}", s)),
        }
    }
}

/// Result of a deployment.
#[derive(Debug, Clone)]
pub struct DeployResult {
    /// Async process ID.
    pub id: String,
    /// Whether the operation is complete.
    pub done: bool,
    /// Current status.
    pub status: DeployStatus,
    /// Whether the deployment succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error_message: Option<String>,
    /// Number of components deployed.
    pub number_components_deployed: u32,
    /// Number of components with errors.
    pub number_components_errors: u32,
    /// Total number of components.
    pub number_components_total: u32,
    /// Number of tests completed.
    pub number_tests_completed: u32,
    /// Number of tests with errors.
    pub number_tests_errors: u32,
    /// Total number of tests.
    pub number_tests_total: u32,
    /// Component failures.
    pub component_failures: Vec<ComponentFailure>,
    /// Component successes.
    pub component_successes: Vec<ComponentSuccess>,
    /// Test failures.
    pub test_failures: Vec<TestFailure>,
    /// State detail message.
    pub state_detail: Option<String>,
}

/// A component failure in deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentFailure {
    pub component_type: Option<String>,
    pub file_name: Option<String>,
    pub full_name: Option<String>,
    pub line_number: Option<u32>,
    pub column_number: Option<u32>,
    pub problem: String,
    pub problem_type: String,
    pub created: bool,
    pub deleted: bool,
}

/// Result of canceling a deployment.
///
/// Returned by `cancel_deploy()`. The `done` field indicates whether the cancellation
/// has completed. Note that canceling is asynchronous — you must poll `check_deploy_status()`
/// to see when the deployment reaches `Canceled` or `Canceling` status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelDeployResult {
    /// The async process ID of the deployment being canceled.
    pub id: String,
    /// Whether the cancel operation has completed.
    pub done: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deploy_options_default() {
        let opts = DeployOptions::default();
        assert!(!opts.allow_missing_files);
        assert!(!opts.check_only);
        assert!(opts.ignore_warnings);
        assert!(opts.rollback_on_error);
        assert!(opts.single_package);
    }

    #[test]
    fn test_deploy_status_parse() {
        assert_eq!(
            "Pending".parse::<DeployStatus>().unwrap(),
            DeployStatus::Pending
        );
        assert_eq!(
            "InProgress".parse::<DeployStatus>().unwrap(),
            DeployStatus::InProgress
        );
        assert_eq!(
            "Succeeded".parse::<DeployStatus>().unwrap(),
            DeployStatus::Succeeded
        );
        assert_eq!(
            "Failed".parse::<DeployStatus>().unwrap(),
            DeployStatus::Failed
        );
    }

    #[test]
    fn test_cancel_deploy_result() {
        use crate::deploy::CancelDeployResult;

        let result = CancelDeployResult {
            id: "0Af123456789ABC".to_string(),
            done: true,
        };

        assert_eq!(result.id, "0Af123456789ABC");
        assert!(result.done);
    }
}
