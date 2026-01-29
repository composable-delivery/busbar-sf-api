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
    pub fn new(instance_url: impl Into<String>, access_token: impl Into<String>) -> Result<Self> {
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
        self.client
            .tooling_query_all(soql)
            .await
            .map_err(Into::into)
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
                message: "Invalid Salesforce ID format".to_string(),
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
        self.query_all(
            "SELECT Id, Name, Body, Status, IsValid, ApiVersion, TableEnumOrId FROM ApexTrigger",
        )
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
                message: "Invalid Salesforce ID format".to_string(),
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
                message: "Invalid Salesforce ID format".to_string(),
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
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !url_security::is_valid_salesforce_id(id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        let path = format!("sobjects/{}/{}", sobject, id);
        self.client.tooling_get(&path).await.map_err(Into::into)
    }

    /// Create a Tooling API SObject.
    #[instrument(skip(self, record))]
    pub async fn create<T: serde::Serialize>(&self, sobject: &str, record: &T) -> Result<String> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
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
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !url_security::is_valid_salesforce_id(id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
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

    // =========================================================================
    // Composite API (Tooling)
    // =========================================================================

    /// Execute a Tooling API composite request with multiple subrequests.
    ///
    /// The Tooling API composite endpoint allows up to 25 subrequests in a single API call.
    /// Subrequests can reference results from earlier subrequests using `@{referenceId}`.
    ///
    /// Available since API v40.0.
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
    /// let safe_type = soql::escape_string(user_input);
    /// let filter = format!("MetadataComponentType = '{}'", safe_type);
    /// let deps = client.get_metadata_component_dependencies(Some(&filter)).await?;
    /// ```
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use busbar_sf_tooling::{CompositeRequest, CompositeSubrequest};
    ///
    /// let request = CompositeRequest {
    ///     all_or_none: false,
    ///     collate_subrequests: false,
    ///     subrequests: vec![
    ///         CompositeSubrequest {
    ///             method: "GET".to_string(),
    ///             url: "/services/data/v62.0/tooling/sobjects/ApexClass/01p...".to_string(),
    ///             reference_id: "refApexClass".to_string(),
    ///             body: None,
    ///         },
    ///     ],
    /// };
    ///
    /// let response = client.composite(&request).await?;
    /// ```
    #[instrument(skip(self, request))]
    pub async fn composite(
        &self,
        request: &busbar_sf_rest::CompositeRequest,
    ) -> Result<busbar_sf_rest::CompositeResponse> {
        let url = self.client.tooling_url("composite");
        self.client
            .post_json(&url, request)
            .await
            .map_err(Into::into)
    }

    /// Execute a Tooling API composite batch request with multiple independent subrequests.
    ///
    /// The composite batch API executes up to 25 subrequests independently.
    /// Unlike the standard composite API, subrequests cannot reference each other's results.
    ///
    /// Available since API v40.0.
    #[instrument(skip(self, request))]
    pub async fn composite_batch(
        &self,
        request: &busbar_sf_rest::CompositeBatchRequest,
    ) -> Result<busbar_sf_rest::CompositeBatchResponse> {
        let url = self.client.tooling_url("composite/batch");
        self.client
            .post_json(&url, request)
            .await
            .map_err(Into::into)
    }

    /// Execute a Tooling API composite tree request to create record hierarchies.
    ///
    /// Creates parent records with nested child records in a single request.
    /// Supports up to 200 records total across all levels of the hierarchy.
    ///
    /// Available since API v42.0.
    ///
    /// # Arguments
    /// * `sobject` - The parent SObject type (e.g., "ApexClass", "CustomField")
    /// * `request` - The tree request containing parent records and nested children
    #[instrument(skip(self, request))]
    pub async fn composite_tree(
        &self,
        sobject: &str,
        request: &busbar_sf_rest::CompositeTreeRequest,
    ) -> Result<busbar_sf_rest::CompositeTreeResponse> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let url = self
            .client
            .tooling_url(&format!("composite/tree/{}", sobject));
        self.client
            .post_json(&url, request)
            .await
            .map_err(Into::into)
    }

    // =========================================================================
    // SObject Collections (Tooling)
    // =========================================================================

    /// Get multiple Tooling API records by ID in a single request.
    ///
    /// Uses a Tooling API SOQL query (`WHERE Id IN (...)`) internally.
    /// The Tooling API's SObject Collections GET endpoint is documented but
    /// does not work reliably for most objects, so we use SOQL instead.
    ///
    /// # Arguments
    /// * `sobject` - The SObject type (e.g., "ApexClass", "CustomField")
    /// * `ids` - Array of record IDs to retrieve
    /// * `fields` - Array of field names to return
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let classes: Vec<serde_json::Value> = client
    ///     .get_multiple("ApexClass", &["01p...", "01p..."], &["Id", "Name", "Body"])
    ///     .await?;
    /// ```
    #[instrument(skip(self))]
    pub async fn get_multiple<T: DeserializeOwned + Clone>(
        &self,
        sobject: &str,
        ids: &[&str],
        fields: &[&str],
    ) -> Result<Vec<T>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        for id in ids {
            if !url_security::is_valid_salesforce_id(id) {
                return Err(Error::new(ErrorKind::Salesforce {
                    error_code: "INVALID_ID".to_string(),
                    message: "Invalid Salesforce ID format".to_string(),
                }));
            }
        }
        let safe_fields: Vec<&str> = soql::filter_safe_fields(fields.iter().copied()).collect();
        if safe_fields.is_empty() {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_FIELDS".to_string(),
                message: "No valid field names provided".to_string(),
            }));
        }
        // Build a SOQL query: SELECT fields FROM sobject WHERE Id IN ('id1','id2',...)
        // IDs are already validated by is_valid_salesforce_id (alphanumeric only),
        // so they are safe to embed directly.
        let fields_clause = safe_fields.join(", ");
        let ids_clause: Vec<String> = ids.iter().map(|id| format!("'{id}'")).collect();
        let soql = format!(
            "SELECT {} FROM {} WHERE Id IN ({})",
            fields_clause,
            sobject,
            ids_clause.join(", ")
        );
        self.query_all(&soql).await
    }

    /// Create multiple Tooling API records in a single request (up to 200).
    ///
    /// Available since API v45.0.
    ///
    /// # Arguments
    /// * `sobject` - The SObject type (e.g., "ApexClass", "CustomField")
    /// * `records` - Array of records to create
    /// * `all_or_none` - If true, all records must succeed or all fail
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use serde_json::json;
    ///
    /// let records = vec![
    ///     json!({"Name": "TestClass1", "Body": "public class TestClass1 {}"}),
    ///     json!({"Name": "TestClass2", "Body": "public class TestClass2 {}"}),
    /// ];
    ///
    /// let results = client.create_multiple("ApexClass", &records, false).await?;
    /// ```
    #[instrument(skip(self, records))]
    pub async fn create_multiple<T: serde::Serialize>(
        &self,
        sobject: &str,
        records: &[T],
        all_or_none: bool,
    ) -> Result<Vec<busbar_sf_rest::CollectionResult>> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let request = busbar_sf_rest::CollectionRequest {
            all_or_none,
            records: records
                .iter()
                .map(|r| {
                    let mut value = serde_json::to_value(r).unwrap_or(serde_json::Value::Null);
                    if let serde_json::Value::Object(ref mut map) = value {
                        map.insert(
                            "attributes".to_string(),
                            serde_json::json!({"type": sobject}),
                        );
                    }
                    value
                })
                .collect(),
        };
        let url = self.client.tooling_url("composite/sobjects");
        self.client
            .post_json(&url, &request)
            .await
            .map_err(Into::into)
    }

    /// Update multiple Tooling API records in a single request (up to 200).
    ///
    /// Available since API v45.0.
    ///
    /// # Arguments
    /// * `sobject` - The SObject type (e.g., "ApexClass", "CustomField")
    /// * `records` - Array of (id, record) tuples to update
    /// * `all_or_none` - If true, all records must succeed or all fail
    #[instrument(skip(self, records))]
    pub async fn update_multiple<T: serde::Serialize>(
        &self,
        sobject: &str,
        records: &[(String, T)],
        all_or_none: bool,
    ) -> Result<Vec<busbar_sf_rest::CollectionResult>> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        // Validate all IDs
        for (id, _) in records {
            if !url_security::is_valid_salesforce_id(id) {
                return Err(Error::new(ErrorKind::Salesforce {
                    error_code: "INVALID_ID".to_string(),
                    message: "Invalid Salesforce ID format".to_string(),
                }));
            }
        }
        let request = busbar_sf_rest::CollectionRequest {
            all_or_none,
            records: records
                .iter()
                .map(|(id, r)| {
                    let mut value = serde_json::to_value(r).unwrap_or(serde_json::Value::Null);
                    if let serde_json::Value::Object(ref mut map) = value {
                        map.insert(
                            "attributes".to_string(),
                            serde_json::json!({"type": sobject}),
                        );
                        map.insert("Id".to_string(), serde_json::json!(id));
                    }
                    value
                })
                .collect(),
        };

        let url = self.client.tooling_url("composite/sobjects");
        let request_builder = self.client.patch(&url).json(&request)?;
        let response = self.client.execute(request_builder).await?;
        response.json().await.map_err(Into::into)
    }

    /// Delete multiple Tooling API records in a single request (up to 200).
    ///
    /// Available since API v45.0.
    ///
    /// # Arguments
    /// * `ids` - Array of record IDs to delete
    /// * `all_or_none` - If true, all records must succeed or all fail
    #[instrument(skip(self))]
    pub async fn delete_multiple(
        &self,
        ids: &[&str],
        all_or_none: bool,
    ) -> Result<Vec<busbar_sf_rest::CollectionResult>> {
        // Validate all IDs before proceeding
        for id in ids {
            if !url_security::is_valid_salesforce_id(id) {
                return Err(Error::new(ErrorKind::Salesforce {
                    error_code: "INVALID_ID".to_string(),
                    message: "Invalid Salesforce ID format".to_string(),
                }));
            }
        }
        let ids_param = ids.join(",");
        let url = format!(
            "{}/services/data/v{}/tooling/composite/sobjects?ids={}&allOrNone={}",
            self.client.instance_url(),
            self.client.api_version(),
            ids_param,
            all_or_none
        );
        let request = self.client.delete(&url);
        let response = self.client.execute(request).await?;
        response.json().await.map_err(Into::into)
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
    #[allow(dead_code)]
    status_code: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token123").unwrap();

        assert_eq!(client.instance_url(), "https://na1.salesforce.com");
        assert_eq!(client.api_version(), "62.0");
    }

    #[test]
    fn test_api_version_override() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token")
            .unwrap()
            .with_api_version("60.0");

        assert_eq!(client.api_version(), "60.0");
    }

    #[test]
    fn test_composite_url_construction() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();

        let url = client.client.tooling_url("composite");
        assert_eq!(
            url,
            "https://na1.salesforce.com/services/data/v62.0/tooling/composite"
        );
    }

    #[test]
    fn test_composite_batch_url_construction() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();

        let url = client.client.tooling_url("composite/batch");
        assert_eq!(
            url,
            "https://na1.salesforce.com/services/data/v62.0/tooling/composite/batch"
        );
    }

    #[test]
    fn test_composite_tree_url_construction() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();

        let url = client
            .client
            .tooling_url(&format!("composite/tree/{}", "ApexClass"));
        assert_eq!(
            url,
            "https://na1.salesforce.com/services/data/v62.0/tooling/composite/tree/ApexClass"
        );
    }

    #[test]
    fn test_collections_get_soql_construction() {
        // get_multiple uses a SOQL query internally because the Tooling API
        // SObject Collections GET endpoint does not work reliably.
        let sobject = "ApexClass";
        let ids = ["01p000000000001AAA", "01p000000000002AAA"];
        let fields = ["Id", "Name"];

        let fields_clause = fields.join(", ");
        let ids_clause: Vec<String> = ids.iter().map(|id| format!("'{id}'")).collect();
        let soql = format!(
            "SELECT {} FROM {} WHERE Id IN ({})",
            fields_clause,
            sobject,
            ids_clause.join(", ")
        );

        assert_eq!(
            soql,
            "SELECT Id, Name FROM ApexClass WHERE Id IN ('01p000000000001AAA', '01p000000000002AAA')"
        );
    }

    #[test]
    fn test_collections_create_url_construction() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();

        let url = client.client.tooling_url("composite/sobjects");
        assert_eq!(
            url,
            "https://na1.salesforce.com/services/data/v62.0/tooling/composite/sobjects"
        );
    }

    #[test]
    fn test_collections_delete_url_construction() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();

        let ids = ["01p000000000001AAA", "01p000000000002AAA"];
        let ids_param = ids.join(",");

        let url = format!(
            "{}/services/data/v{}/tooling/composite/sobjects?ids={}&allOrNone={}",
            client.client.instance_url(),
            client.client.api_version(),
            ids_param,
            false
        );

        assert_eq!(
            url,
            "https://na1.salesforce.com/services/data/v62.0/tooling/composite/sobjects?ids=01p000000000001AAA,01p000000000002AAA&allOrNone=false"
        );
    }

    // =========================================================================
    // get_multiple validation tests
    // =========================================================================

    #[tokio::test]
    async fn test_get_multiple_empty_ids_returns_empty() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();
        let result: Vec<serde_json::Value> = client
            .get_multiple("ApexClass", &[], &["Id", "Name"])
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_multiple_invalid_sobject_name() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();
        let result: std::result::Result<Vec<serde_json::Value>, _> = client
            .get_multiple("Robert'; DROP TABLE--", &["01p000000000001AAA"], &["Id"])
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_SOBJECT"),
            "Expected INVALID_SOBJECT error, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_get_multiple_invalid_id_format() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();
        let result: std::result::Result<Vec<serde_json::Value>, _> = client
            .get_multiple("ApexClass", &["not-a-valid-sf-id!"], &["Id"])
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_ID"),
            "Expected INVALID_ID error, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_get_multiple_invalid_fields_filtered() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();
        // All field names are invalid (contain injection attempts)
        let result: std::result::Result<Vec<serde_json::Value>, _> = client
            .get_multiple(
                "ApexClass",
                &["01p000000000001AAA"],
                &["'; DROP TABLE--", "1=1 OR"],
            )
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_FIELDS"),
            "Expected INVALID_FIELDS error, got: {err}"
        );
    }

    #[test]
    fn test_get_multiple_soql_construction_with_many_ids() {
        // Verify the SOQL query is constructed correctly for multiple IDs
        let sobject = "ApexClass";
        let ids = [
            "01p000000000001AAA",
            "01p000000000002AAA",
            "01p000000000003AAA",
        ];
        let fields = ["Id", "Name", "Body"];

        let fields_clause = fields.join(", ");
        let ids_clause: Vec<String> = ids.iter().map(|id| format!("'{id}'")).collect();
        let soql = format!(
            "SELECT {} FROM {} WHERE Id IN ({})",
            fields_clause,
            sobject,
            ids_clause.join(", ")
        );

        assert_eq!(
            soql,
            "SELECT Id, Name, Body FROM ApexClass WHERE Id IN ('01p000000000001AAA', '01p000000000002AAA', '01p000000000003AAA')"
        );
    }
}
