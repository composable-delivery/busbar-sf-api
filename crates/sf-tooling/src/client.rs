//! Salesforce Tooling API client.
//!
//! This client wraps `SalesforceClient` from `sf-client` and provides
//! typed methods for Tooling API operations.

use serde::de::DeserializeOwned;
use tracing::instrument;

use busbar_sf_client::security::{soql, url as url_security};
use busbar_sf_client::{ClientConfig, QueryResult, SalesforceClient};

use crate::error::{Error, ErrorKind, Result};
use crate::types::*;

/// Salesforce Tooling API client.
///
/// Provides typed methods for Tooling API operations:
/// - Execute anonymous Apex
/// - Query Apex classes, triggers, and logs
/// - Manage debug logs and trace flags
/// - Code coverage information
///
/// # Example
///
/// ```rust,ignore
/// use sf_tooling::ToolingClient;
///
/// let client = ToolingClient::new(
///     "https://myorg.my.salesforce.com",
///     "access_token_here",
/// )?;
///
/// // Execute anonymous Apex
/// let result = client.execute_anonymous("System.debug('Hello');").await?;
///
/// // Query Apex classes
/// let classes: Vec<ApexClass> = client
///     .query_all("SELECT Id, Name FROM ApexClass")
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct ToolingClient {
    client: SalesforceClient,
}

impl ToolingClient {
    /// Create a new Tooling API client with the given instance URL and access token.
    pub fn new(
        instance_url: impl Into<String>,
        access_token: impl Into<String>,
    ) -> Result<Self> {
        let client = SalesforceClient::new(instance_url, access_token)?;
        Ok(Self { client })
    }

    /// Create a new Tooling API client with custom HTTP configuration.
    pub fn with_config(
        instance_url: impl Into<String>,
        access_token: impl Into<String>,
        config: ClientConfig,
    ) -> Result<Self> {
        let client = SalesforceClient::with_config(instance_url, access_token, config)?;
        Ok(Self { client })
    }

    /// Create a Tooling client from an existing SalesforceClient.
    pub fn from_client(client: SalesforceClient) -> Self {
        Self { client }
    }

    /// Get the underlying SalesforceClient.
    pub fn inner(&self) -> &SalesforceClient {
        &self.client
    }

    /// Get the instance URL.
    pub fn instance_url(&self) -> &str {
        self.client.instance_url()
    }

    /// Get the API version.
    pub fn api_version(&self) -> &str {
        self.client.api_version()
    }

    /// Set the API version.
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.client = self.client.with_api_version(version);
        self
    }

    // =========================================================================
    // Query Operations
    // =========================================================================

    /// Execute a SOQL query against the Tooling API.
    ///
    /// Returns the first page of results. Use `query_all` for automatic pagination.
    ///
    /// # Security
    ///
    /// **IMPORTANT**: If you are including user-provided values in the WHERE clause,
    /// you MUST escape them to prevent SOQL injection attacks:
    ///
    /// ```rust,ignore
    /// use busbar_sf_client::security::soql;
    ///
    /// // CORRECT - properly escaped:
    /// let safe_name = soql::escape_string(user_input);
    /// let query = format!("SELECT Id FROM ApexClass WHERE Name = '{}'", safe_name);
    /// ```
    #[instrument(skip(self))]
    pub async fn query<T: DeserializeOwned>(&self, soql: &str) -> Result<QueryResult<T>> {
        self.client.tooling_query(soql).await.map_err(Into::into)
    }

    /// Execute a SOQL query and return all results (automatic pagination).
    ///
    /// # Security
    ///
    /// **IMPORTANT**: Escape user-provided values with `busbar_sf_client::security::soql::escape_string()`
    /// to prevent SOQL injection attacks. See `query()` for examples.
    #[instrument(skip(self))]
    pub async fn query_all<T: DeserializeOwned + Clone>(&self, soql: &str) -> Result<Vec<T>> {
        self.client.tooling_query_all(soql).await.map_err(Into::into)
    }

    // =========================================================================
    // Execute Anonymous
    // =========================================================================

    /// Execute anonymous Apex code.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = client.execute_anonymous("System.debug('Hello World');").await?;
    /// if result.success {
    ///     println!("Execution successful");
    /// } else if let Some(err) = result.compile_problem {
    ///     println!("Compilation error: {}", err);
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn execute_anonymous(&self, apex_code: &str) -> Result<ExecuteAnonymousResult> {
        let encoded = urlencoding::encode(apex_code);
        let url = format!(
            "{}/services/data/v{}/tooling/executeAnonymous/?anonymousBody={}",
            self.client.instance_url(),
            self.client.api_version(),
            encoded
        );

        let result: ExecuteAnonymousResult = self.client.get_json(&url).await?;

        // Check for compilation or execution errors
        if !result.compiled {
            if let Some(ref problem) = result.compile_problem {
                return Err(Error::new(ErrorKind::ApexCompilation(problem.clone())));
            }
        }

        if !result.success {
            if let Some(ref message) = result.exception_message {
                return Err(Error::new(ErrorKind::ApexExecution(message.clone())));
            }
        }

        Ok(result)
    }

    // =========================================================================
    // Apex Class Operations
    // =========================================================================

    /// Get all Apex classes in the org.
    #[instrument(skip(self))]
    pub async fn get_apex_classes(&self) -> Result<Vec<ApexClass>> {
        self.query_all("SELECT Id, Name, Body, Status, IsValid, ApiVersion, NamespacePrefix, CreatedDate, LastModifiedDate FROM ApexClass")
            .await
    }

    /// Get an Apex class by name.
    #[instrument(skip(self))]
    pub async fn get_apex_class_by_name(&self, name: &str) -> Result<Option<ApexClass>> {
        let safe_name = soql::escape_string(name);
        let soql = format!(
            "SELECT Id, Name, Body, Status, IsValid, ApiVersion, NamespacePrefix, CreatedDate, LastModifiedDate FROM ApexClass WHERE Name = '{}'",
            safe_name
        );
        let mut classes: Vec<ApexClass> = self.query_all(&soql).await?;
        Ok(classes.pop())
    }

    /// Get an Apex class by ID.
    #[instrument(skip(self))]
    pub async fn get_apex_class(&self, id: &str) -> Result<ApexClass> {
        if !url_security::is_valid_salesforce_id(id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: format!("Invalid Salesforce ID format"),
            }));
        }
        let path = format!("sobjects/ApexClass/{}", id);
        self.client.tooling_get(&path).await.map_err(Into::into)
    }

    // =========================================================================
    // Apex Trigger Operations
    // =========================================================================

    /// Get all Apex triggers in the org.
    #[instrument(skip(self))]
    pub async fn get_apex_triggers(&self) -> Result<Vec<ApexTrigger>> {
        self.query_all("SELECT Id, Name, Body, Status, IsValid, ApiVersion, TableEnumOrId FROM ApexTrigger")
            .await
    }

    /// Get an Apex trigger by name.
    #[instrument(skip(self))]
    pub async fn get_apex_trigger_by_name(&self, name: &str) -> Result<Option<ApexTrigger>> {
        let safe_name = soql::escape_string(name);
        let soql = format!(
            "SELECT Id, Name, Body, Status, IsValid, ApiVersion, TableEnumOrId FROM ApexTrigger WHERE Name = '{}'",
            safe_name
        );
        let mut triggers: Vec<ApexTrigger> = self.query_all(&soql).await?;
        Ok(triggers.pop())
    }

    // =========================================================================
    // Debug Log Operations
    // =========================================================================

    /// Get recent Apex logs.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of logs to return (defaults to 20)
    #[instrument(skip(self))]
    pub async fn get_apex_logs(&self, limit: Option<u32>) -> Result<Vec<ApexLog>> {
        let limit = limit.unwrap_or(20);
        let soql = format!(
            "SELECT Id, LogUserId, LogUser.Name, LogLength, LastModifiedDate, StartTime, Status, Operation, Request, Application, DurationMilliseconds, Location FROM ApexLog ORDER BY LastModifiedDate DESC LIMIT {}",
            limit
        );
        self.query_all(&soql).await
    }

    /// Get the body of a specific Apex log.
    #[instrument(skip(self))]
    pub async fn get_apex_log_body(&self, log_id: &str) -> Result<String> {
        if !url_security::is_valid_salesforce_id(log_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: format!("Invalid Salesforce ID format"),
            }));
        }
        let url = format!(
            "{}/services/data/v{}/tooling/sobjects/ApexLog/{}/Body",
            self.client.instance_url(),
            self.client.api_version(),
            log_id
        );

        let request = self.client.get(&url);
        let response = self.client.execute(request).await?;
        response.text().await.map_err(Into::into)
    }

    /// Delete an Apex log.
    #[instrument(skip(self))]
    pub async fn delete_apex_log(&self, log_id: &str) -> Result<()> {
        if !url_security::is_valid_salesforce_id(log_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: format!("Invalid Salesforce ID format"),
            }));
        }
        let url = format!(
            "{}/services/data/v{}/tooling/sobjects/ApexLog/{}",
            self.client.instance_url(),
            self.client.api_version(),
            log_id
        );

        let request = self.client.delete(&url);
        let response = self.client.execute(request).await?;

        if response.status() == 204 || response.is_success() {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Salesforce {
                error_code: "DELETE_FAILED".to_string(),
                message: format!("Failed to delete log: status {}", response.status()),
            }))
        }
    }

    /// Delete all Apex logs for the current user.
    #[instrument(skip(self))]
    pub async fn delete_all_apex_logs(&self) -> Result<u32> {
        let logs = self.get_apex_logs(Some(200)).await?;
        let count = logs.len() as u32;

        for log in logs {
            self.delete_apex_log(&log.id).await?;
        }

        Ok(count)
    }

    // =========================================================================
    // Code Coverage Operations
    // =========================================================================

    /// Get code coverage for all Apex classes and triggers.
    #[instrument(skip(self))]
    pub async fn get_code_coverage(&self) -> Result<Vec<ApexCodeCoverageAggregate>> {
        self.query_all(
            "SELECT Id, ApexClassOrTriggerId, ApexClassOrTrigger.Name, NumLinesCovered, NumLinesUncovered, Coverage FROM ApexCodeCoverageAggregate"
        ).await
    }

    /// Get overall org-wide code coverage percentage.
    #[instrument(skip(self))]
    pub async fn get_org_wide_coverage(&self) -> Result<f64> {
        let coverage = self.get_code_coverage().await?;

        let mut total_covered = 0i64;
        let mut total_uncovered = 0i64;

        for item in coverage {
            total_covered += item.num_lines_covered as i64;
            total_uncovered += item.num_lines_uncovered as i64;
        }

        let total_lines = total_covered + total_uncovered;
        if total_lines == 0 {
            return Ok(0.0);
        }

        Ok((total_covered as f64 / total_lines as f64) * 100.0)
    }

    // =========================================================================
    // Trace Flag Operations
    // =========================================================================

    /// Get all active trace flags.
    #[instrument(skip(self))]
    pub async fn get_trace_flags(&self) -> Result<Vec<TraceFlag>> {
        self.query_all(
            "SELECT Id, TracedEntityId, LogType, DebugLevelId, StartDate, ExpirationDate FROM TraceFlag"
        ).await
    }

    /// Get all debug levels.
    #[instrument(skip(self))]
    pub async fn get_debug_levels(&self) -> Result<Vec<DebugLevel>> {
        self.query_all(
            "SELECT Id, DeveloperName, MasterLabel, ApexCode, ApexProfiling, Callout, Database, System, Validation, Visualforce, Workflow FROM DebugLevel"
        ).await
    }

    // =========================================================================
    // Generic SObject Operations (Tooling)
    // =========================================================================

    /// Get a Tooling API SObject by ID.
    #[instrument(skip(self))]
    pub async fn get<T: DeserializeOwned>(&self, sobject: &str, id: &str) -> Result<T> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: format!("Invalid SObject name"),
            }));
        }
        if !url_security::is_valid_salesforce_id(id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: format!("Invalid Salesforce ID format"),
            }));
        }
        let path = format!("sobjects/{}/{}", sobject, id);
        self.client.tooling_get(&path).await.map_err(Into::into)
    }

    /// Create a Tooling API SObject.
    #[instrument(skip(self, record))]
    pub async fn create<T: serde::Serialize>(
        &self,
        sobject: &str,
        record: &T,
    ) -> Result<String> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: format!("Invalid SObject name"),
            }));
        }
        let path = format!("sobjects/{}", sobject);
        let result: CreateResponse = self.client.tooling_post(&path, record).await?;

        if result.success {
            Ok(result.id)
        } else {
            Err(Error::new(ErrorKind::Salesforce {
                error_code: "CREATE_FAILED".to_string(),
                message: result
                    .errors
                    .into_iter()
                    .map(|e| e.message)
                    .collect::<Vec<_>>()
                    .join("; "),
            }))
        }
    }

    /// Delete a Tooling API SObject.
    #[instrument(skip(self))]
    pub async fn delete(&self, sobject: &str, id: &str) -> Result<()> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: format!("Invalid SObject name"),
            }));
        }
        if !url_security::is_valid_salesforce_id(id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: format!("Invalid Salesforce ID format"),
            }));
        }
        let url = format!(
            "{}/services/data/v{}/tooling/sobjects/{}/{}",
            self.client.instance_url(),
            self.client.api_version(),
            sobject,
            id
        );

        let request = self.client.delete(&url);
        let response = self.client.execute(request).await?;

        if response.status() == 204 || response.is_success() {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Salesforce {
                error_code: "DELETE_FAILED".to_string(),
                message: format!("Failed to delete {}: status {}", sobject, response.status()),
            }))
        }
    }
}

/// Response from create operations.
#[derive(Debug, Clone, serde::Deserialize)]
struct CreateResponse {
    id: String,
    success: bool,
    #[serde(default)]
    errors: Vec<CreateError>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct CreateError {
    message: String,
    #[serde(rename = "statusCode")]
    status_code: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = ToolingClient::new(
            "https://na1.salesforce.com",
            "token123",
        ).unwrap();

        assert_eq!(client.instance_url(), "https://na1.salesforce.com");
        assert_eq!(client.api_version(), "62.0");
    }

    #[test]
    fn test_api_version_override() {
        let client = ToolingClient::new(
            "https://na1.salesforce.com",
            "token",
        ).unwrap().with_api_version("60.0");

        assert_eq!(client.api_version(), "60.0");
    }
}
