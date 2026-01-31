//! Salesforce REST API client.
//!
//! This client wraps `SalesforceClient` from `sf-client` and provides
//! typed methods for REST API operations including CRUD, Query, Describe,
//! Composite, and Collections.

use serde::{de::DeserializeOwned, Serialize};
use tracing::instrument;

use busbar_sf_client::security::{soql, url as url_security};
use busbar_sf_client::{ClientConfig, SalesforceClient};

use crate::actions::{
    InvocableActionCollection, InvocableActionDescribe, InvocableActionRequest,
    InvocableActionResult,
};
use crate::collections::{CollectionRequest, CollectionResult};
use crate::composite::{
    CompositeBatchRequest, CompositeBatchResponse, CompositeRequest, CompositeResponse,
    CompositeTreeRequest, CompositeTreeResponse,
};
use crate::describe::{DescribeGlobalResult, DescribeSObjectResult};
use crate::error::{Error, ErrorKind, Result};
use crate::list_views::{ListView, ListViewCollection, ListViewDescribe, ListViewResult};
use crate::process::{
    ApprovalRequest, ApprovalResult, PendingApprovalCollection, ProcessRuleCollection,
    ProcessRuleRequest, ProcessRuleResult,
};
use crate::query::QueryResult;
use crate::quick_actions::{QuickAction, QuickActionDescribe, QuickActionResult};
use crate::sobject::{CreateResult, UpsertResult};

/// Salesforce REST API client.
///
/// Provides typed methods for all REST API operations:
/// - CRUD operations on SObjects
/// - SOQL queries with automatic pagination
/// - SOSL search
/// - Describe operations
/// - Composite API
/// - SObject Collections
///
/// # Example
///
/// ```rust,ignore
/// use sf_rest::SalesforceRestClient;
///
/// let client = SalesforceRestClient::new(
///     "https://myorg.my.salesforce.com",
///     "access_token_here",
/// )?;
///
/// // Query
/// let accounts: Vec<Account> = client.query_all("SELECT Id, Name FROM Account").await?;
///
/// // Create
/// let id = client.create("Account", &json!({"Name": "New Account"})).await?;
///
/// // Update
/// client.update("Account", &id, &json!({"Name": "Updated"})).await?;
///
/// // Delete
/// client.delete("Account", &id).await?;
/// ```
#[derive(Debug, Clone)]
pub struct SalesforceRestClient {
    client: SalesforceClient,
}

impl SalesforceRestClient {
    /// Create a new REST client with the given instance URL and access token.
    pub fn new(instance_url: impl Into<String>, access_token: impl Into<String>) -> Result<Self> {
        let client = SalesforceClient::new(instance_url, access_token)?;
        Ok(Self { client })
    }

    /// Create a new REST client with custom HTTP configuration.
    pub fn with_config(
        instance_url: impl Into<String>,
        access_token: impl Into<String>,
        config: ClientConfig,
    ) -> Result<Self> {
        let client = SalesforceClient::with_config(instance_url, access_token, config)?;
        Ok(Self { client })
    }

    /// Create a REST client from an existing SalesforceClient.
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
    // Describe Operations
    // =========================================================================

    /// Get a list of all SObjects available in the org.
    ///
    /// This is equivalent to calling `/services/data/vXX.0/sobjects/`.
    #[instrument(skip(self))]
    pub async fn describe_global(&self) -> Result<DescribeGlobalResult> {
        self.client.rest_get("sobjects").await.map_err(Into::into)
    }

    /// Get detailed metadata for a specific SObject.
    ///
    /// This is equivalent to calling `/services/data/vXX.0/sobjects/{sobject}/describe`.
    #[instrument(skip(self))]
    pub async fn describe_sobject(&self, sobject: &str) -> Result<DescribeSObjectResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/describe", sobject);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    // =========================================================================
    // Layout Operations
    // =========================================================================

    /// Get all page layouts for a specific SObject.
    ///
    /// This returns metadata about all page layouts configured for the SObject,
    /// including sections, rows, items, and field metadata.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let layouts = client.describe_layouts("Account").await?;
    /// println!("Account layouts: {:?}", layouts);
    /// ```
    ///
    /// This is equivalent to calling `/services/data/vXX.0/sobjects/{sobject}/describe/layouts`.
    #[instrument(skip(self))]
    pub async fn describe_layouts(
        &self,
        sobject: &str,
    ) -> Result<crate::layout::DescribeLayoutsResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/describe/layouts", sobject);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get a specific named layout for an SObject.
    ///
    /// This returns the layout metadata for a specific named layout.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let layout = client.describe_named_layout("Account", "MyCustomLayout").await?;
    /// println!("Layout metadata: {:?}", layout);
    /// ```
    ///
    /// This is equivalent to calling `/services/data/vXX.0/sobjects/{sobject}/describe/namedLayouts/{layoutName}`.
    #[instrument(skip(self))]
    pub async fn describe_named_layout(
        &self,
        sobject: &str,
        layout_name: &str,
    ) -> Result<crate::layout::NamedLayoutResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        // URL-encode the layout name to handle special characters
        let encoded_name = url_security::encode_param(layout_name);
        let path = format!(
            "sobjects/{}/describe/namedLayouts/{}",
            sobject, encoded_name
        );
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get approval process layouts for a specific SObject.
    ///
    /// This returns the approval process layout information including
    /// approval steps, actions, and field mappings.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let approval_layouts = client.describe_approval_layouts("Account").await?;
    /// println!("Approval layouts: {:?}", approval_layouts);
    /// ```
    ///
    /// This is equivalent to calling `/services/data/vXX.0/sobjects/{sobject}/describe/approvalLayouts`.
    #[instrument(skip(self))]
    pub async fn describe_approval_layouts(
        &self,
        sobject: &str,
    ) -> Result<crate::layout::ApprovalLayoutsResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/describe/approvalLayouts", sobject);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get compact layouts for a specific SObject.
    ///
    /// Compact layouts are used in the Salesforce mobile app and Lightning Experience
    /// to show a preview of a record in a compact space.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let compact_layouts = client.describe_compact_layouts("Account").await?;
    /// println!("Compact layouts: {:?}", compact_layouts);
    /// ```
    ///
    /// This is equivalent to calling `/services/data/vXX.0/sobjects/{sobject}/describe/compactLayouts`.
    #[instrument(skip(self))]
    pub async fn describe_compact_layouts(
        &self,
        sobject: &str,
    ) -> Result<crate::layout::CompactLayoutsResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/describe/compactLayouts", sobject);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get global publisher layouts (global quick actions).
    ///
    /// This returns global quick actions and publisher layouts that are
    /// available across the entire organization, not tied to a specific SObject.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let global_layouts = client.describe_global_publisher_layouts().await?;
    /// println!("Global layouts: {:?}", global_layouts);
    /// ```
    ///
    /// This is equivalent to calling `/services/data/vXX.0/sobjects/Global/describe/layouts`.
    #[instrument(skip(self))]
    pub async fn describe_global_publisher_layouts(
        &self,
    ) -> Result<crate::layout::GlobalPublisherLayoutsResult> {
        let path = "sobjects/Global/describe/layouts";
        self.client.rest_get(path).await.map_err(Into::into)
    }

    // =========================================================================
    // CRUD Operations
    // =========================================================================

    /// Create a new record.
    ///
    /// Returns the ID of the created record.
    #[instrument(skip(self, record))]
    pub async fn create<T: Serialize>(&self, sobject: &str, record: &T) -> Result<String> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}", sobject);
        let result: CreateResult = self.client.rest_post(&path, record).await?;

        if result.success {
            Ok(result.id)
        } else {
            let errors: Vec<String> = result.errors.iter().map(|e| e.message.clone()).collect();
            Err(Error::new(ErrorKind::Salesforce {
                error_code: "CREATE_FAILED".to_string(),
                message: errors.join("; "),
            }))
        }
    }

    /// Get a record by ID.
    ///
    /// Optionally specify which fields to retrieve.
    #[instrument(skip(self))]
    pub async fn get<T: DeserializeOwned>(
        &self,
        sobject: &str,
        id: &str,
        fields: Option<&[&str]>,
    ) -> Result<T> {
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
        let path = if let Some(fields) = fields {
            // Validate and filter field names for safety
            let safe_fields: Vec<&str> = soql::filter_safe_fields(fields.iter().copied()).collect();
            if safe_fields.is_empty() {
                return Err(Error::new(ErrorKind::Salesforce {
                    error_code: "INVALID_FIELDS".to_string(),
                    message: "No valid field names provided".to_string(),
                }));
            }
            format!(
                "sobjects/{}/{}?fields={}",
                sobject,
                id,
                safe_fields.join(",")
            )
        } else {
            format!("sobjects/{}/{}", sobject, id)
        };
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Update a record.
    #[instrument(skip(self, record))]
    pub async fn update<T: Serialize>(&self, sobject: &str, id: &str, record: &T) -> Result<()> {
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
        self.client
            .rest_patch(&path, record)
            .await
            .map_err(Into::into)
    }

    /// Delete a record.
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
        let path = format!("sobjects/{}/{}", sobject, id);
        self.client.rest_delete(&path).await.map_err(Into::into)
    }

    /// Upsert a record using an external ID field.
    ///
    /// Creates the record if it doesn't exist, updates it if it does.
    #[instrument(skip(self, record))]
    pub async fn upsert<T: Serialize>(
        &self,
        sobject: &str,
        external_id_field: &str,
        external_id_value: &str,
        record: &T,
    ) -> Result<UpsertResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !soql::is_safe_field_name(external_id_field) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_FIELD".to_string(),
                message: "Invalid external ID field name".to_string(),
            }));
        }
        // URL-encode the external ID value to handle special characters
        let encoded_value = url_security::encode_param(external_id_value);
        let path = format!(
            "sobjects/{}/{}/{}",
            sobject, external_id_field, encoded_value
        );
        let url = self.client.rest_url(&path);
        let request = self.client.patch(&url).json(record)?;
        let response = self.client.execute(request).await?;

        // Upsert returns 201 Created or 204 No Content
        let status = response.status();
        if status == 201 {
            // Created - response has the ID
            let result: UpsertResult = response.json().await?;
            Ok(result)
        } else if status == 204 {
            // Updated - no response body
            Ok(UpsertResult {
                id: external_id_value.to_string(),
                success: true,
                created: false,
                errors: vec![],
            })
        } else {
            Err(Error::new(ErrorKind::Salesforce {
                error_code: "UPSERT_FAILED".to_string(),
                message: format!("Unexpected status: {}", status),
            }))
        }
    }

    // =========================================================================
    // Query Operations
    // =========================================================================

    /// Execute a SOQL query.
    ///
    /// Returns the first page of results. Use `query_all` for automatic pagination.
    ///
    /// # Security
    ///
    /// **IMPORTANT**: If you are including user-provided values in the WHERE clause,
    /// you MUST escape them to prevent SOQL injection attacks. Use the security utilities:
    ///
    /// ```rust,ignore
    /// use busbar_sf_client::security::soql;
    ///
    /// // WRONG - vulnerable to injection:
    /// let query = format!("SELECT Id FROM Account WHERE Name = '{}'", user_input);
    ///
    /// // CORRECT - properly escaped:
    /// let safe_value = soql::escape_string(user_input);
    /// let query = format!("SELECT Id FROM Account WHERE Name = '{}'", safe_value);
    /// ```
    #[instrument(skip(self))]
    pub async fn query<T: DeserializeOwned>(&self, soql: &str) -> Result<QueryResult<T>> {
        self.client.query(soql).await.map_err(Into::into)
    }

    /// Execute a SOQL query and return all results (automatic pagination).
    ///
    /// # Security
    ///
    /// **IMPORTANT**: Escape user-provided values with `busbar_sf_client::security::soql::escape_string()`
    /// to prevent SOQL injection attacks. See `query()` for examples.
    #[instrument(skip(self))]
    pub async fn query_all<T: DeserializeOwned + Clone>(&self, soql: &str) -> Result<Vec<T>> {
        self.client.query_all(soql).await.map_err(Into::into)
    }

    /// Execute a SOQL query including deleted/archived records.
    ///
    /// # Security
    ///
    /// **IMPORTANT**: Escape user-provided values with `busbar_sf_client::security::soql::escape_string()`
    /// to prevent SOQL injection attacks. See `query()` for examples.
    #[instrument(skip(self))]
    pub async fn query_all_including_deleted<T: DeserializeOwned>(
        &self,
        soql: &str,
    ) -> Result<QueryResult<T>> {
        let encoded = urlencoding::encode(soql);
        let url = format!(
            "{}/services/data/v{}/queryAll?q={}",
            self.client.instance_url(),
            self.client.api_version(),
            encoded
        );
        self.client.get_json(&url).await.map_err(Into::into)
    }

    /// Fetch the next page of query results.
    #[instrument(skip(self))]
    pub async fn query_more<T: DeserializeOwned>(
        &self,
        next_records_url: &str,
    ) -> Result<QueryResult<T>> {
        self.client
            .get_json(next_records_url)
            .await
            .map_err(Into::into)
    }

    // =========================================================================
    // Search Operations (SOSL)
    // =========================================================================

    /// Execute a SOSL search.
    ///
    /// # Security
    ///
    /// **IMPORTANT**: If you are including user-provided values in the search term,
    /// you MUST escape them. Use `busbar_sf_client::security::soql::escape_string()`
    /// for string values in SOSL queries.
    #[instrument(skip(self))]
    pub async fn search<T: DeserializeOwned>(&self, sosl: &str) -> Result<SearchResult<T>> {
        let encoded = urlencoding::encode(sosl);
        let url = format!(
            "{}/services/data/v{}/search?q={}",
            self.client.instance_url(),
            self.client.api_version(),
            encoded
        );
        self.client.get_json(&url).await.map_err(Into::into)
    }

    // =========================================================================
    // Composite API
    // =========================================================================

    /// Execute a composite request with multiple subrequests.
    ///
    /// The composite API allows up to 25 subrequests in a single API call.
    /// Subrequests can reference results from earlier subrequests using `@{referenceId}`.
    ///
    /// Available since API v34.0.
    #[instrument(skip(self, request))]
    pub async fn composite(&self, request: &CompositeRequest) -> Result<CompositeResponse> {
        self.client
            .rest_post("composite", request)
            .await
            .map_err(Into::into)
    }

    /// Execute a composite batch request with multiple independent subrequests.
    ///
    /// The composite batch API executes up to 25 subrequests independently.
    /// Unlike the standard composite API, subrequests cannot reference each other's results.
    ///
    /// Available since API v34.0.
    #[instrument(skip(self, request))]
    pub async fn composite_batch(
        &self,
        request: &CompositeBatchRequest,
    ) -> Result<CompositeBatchResponse> {
        self.client
            .rest_post("composite/batch", request)
            .await
            .map_err(Into::into)
    }

    /// Execute a composite tree request to create record hierarchies.
    ///
    /// Creates parent records with nested child records in a single request.
    /// Supports up to 200 records total across all levels of the hierarchy.
    ///
    /// Available since API v42.0.
    ///
    /// # Arguments
    /// * `sobject` - The parent SObject type (e.g., "Account")
    /// * `request` - The tree request containing parent records and nested children
    #[instrument(skip(self, request))]
    pub async fn composite_tree(
        &self,
        sobject: &str,
        request: &CompositeTreeRequest,
    ) -> Result<CompositeTreeResponse> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("composite/tree/{}", sobject);
        self.client
            .rest_post(&path, request)
            .await
            .map_err(Into::into)
    }

    // =========================================================================
    // SObject Collections
    // =========================================================================

    /// Create multiple records in a single request (up to 200).
    #[instrument(skip(self, records))]
    pub async fn create_multiple<T: Serialize>(
        &self,
        sobject: &str,
        records: &[T],
        all_or_none: bool,
    ) -> Result<Vec<CollectionResult>> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let request = CollectionRequest {
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
        self.client
            .rest_post("composite/sobjects", &request)
            .await
            .map_err(Into::into)
    }

    /// Update multiple records in a single request (up to 200).
    #[instrument(skip(self, records))]
    pub async fn update_multiple<T: Serialize>(
        &self,
        sobject: &str,
        records: &[(String, T)], // (id, record)
        all_or_none: bool,
    ) -> Result<Vec<CollectionResult>> {
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
        let request = CollectionRequest {
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

        let url = self.client.rest_url("composite/sobjects");
        let request_builder = self.client.patch(&url).json(&request)?;
        let response = self.client.execute(request_builder).await?;
        response.json().await.map_err(Into::into)
    }

    /// Delete multiple records in a single request (up to 200).
    #[instrument(skip(self))]
    pub async fn delete_multiple(
        &self,
        ids: &[&str],
        all_or_none: bool,
    ) -> Result<Vec<CollectionResult>> {
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
            "{}/services/data/v{}/composite/sobjects?ids={}&allOrNone={}",
            self.client.instance_url(),
            self.client.api_version(),
            ids_param,
            all_or_none
        );
        let request = self.client.delete(&url);
        let response = self.client.execute(request).await?;
        response.json().await.map_err(Into::into)
    }

    /// Get multiple records by ID in a single request (up to 2000).
    #[instrument(skip(self))]
    pub async fn get_multiple<T: DeserializeOwned>(
        &self,
        sobject: &str,
        ids: &[&str],
        fields: &[&str],
    ) -> Result<Vec<T>> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        // Validate all IDs
        for id in ids {
            if !url_security::is_valid_salesforce_id(id) {
                return Err(Error::new(ErrorKind::Salesforce {
                    error_code: "INVALID_ID".to_string(),
                    message: "Invalid Salesforce ID format".to_string(),
                }));
            }
        }
        // Validate and filter field names
        let safe_fields: Vec<&str> = soql::filter_safe_fields(fields.iter().copied()).collect();
        if safe_fields.is_empty() {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_FIELDS".to_string(),
                message: "No valid field names provided".to_string(),
            }));
        }
        let ids_param = ids.join(",");
        let fields_param = safe_fields.join(",");
        let url = format!(
            "{}/services/data/v{}/composite/sobjects/{}?ids={}&fields={}",
            self.client.instance_url(),
            self.client.api_version(),
            sobject,
            ids_param,
            fields_param
        );
        // The SObject Collections GET response is a JSON array that may contain
        // null entries for records that could not be retrieved (deleted, no access, etc.).
        // Deserialize as Vec<Option<T>> and filter out the nulls.
        let results: Vec<Option<T>> = self.client.get_json(&url).await.map_err(Error::from)?;
        Ok(results.into_iter().flatten().collect())
    }

    // =========================================================================
    // Quick Actions
    // =========================================================================

    /// List all quick actions available for a specific SObject.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let actions = client.list_quick_actions("Account").await?;
    /// for action in actions {
    ///     println!("{}: {}", action.name, action.label);
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn list_quick_actions(&self, sobject: &str) -> Result<Vec<QuickAction>> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/quickActions", sobject);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get detailed metadata for a specific quick action.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let action = client.describe_quick_action("Account", "SendEmail").await?;
    /// println!("Action type: {}", action.action_type);
    /// ```
    #[instrument(skip(self))]
    pub async fn describe_quick_action(
        &self,
        sobject: &str,
        action_name: &str,
    ) -> Result<QuickActionDescribe> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !soql::is_safe_sobject_name(action_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION_NAME".to_string(),
                message: "Invalid action name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/quickActions/{}", sobject, action_name);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Invoke a quick action.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use serde_json::json;
    ///
    /// let result = client.invoke_quick_action(
    ///     "Account",
    ///     "SendEmail",
    ///     &json!({"Subject": "Hello", "Body": "Test email"})
    /// ).await?;
    /// if result.success {
    ///     println!("Action invoked successfully");
    /// }
    /// ```
    #[instrument(skip(self, body))]
    pub async fn invoke_quick_action<T: Serialize>(
        &self,
        sobject: &str,
        action_name: &str,
        body: &T,
    ) -> Result<QuickActionResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !soql::is_safe_sobject_name(action_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION_NAME".to_string(),
                message: "Invalid action name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/quickActions/{}", sobject, action_name);
        self.client.rest_post(&path, body).await.map_err(Into::into)
    }

    // =========================================================================
    // List Views
    // =========================================================================

    /// List all list views for a specific SObject.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let list_views = client.list_views("Account").await?;
    /// for view in list_views.listviews {
    ///     println!("{}: {}", view.developer_name, view.label);
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn list_views(&self, sobject: &str) -> Result<ListViewCollection> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/listviews", sobject);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get a specific list view by ID.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let view = client.get_list_view("Account", "00B000000000001AAA").await?;
    /// println!("View: {}", view.label);
    /// ```
    #[instrument(skip(self))]
    pub async fn get_list_view(&self, sobject: &str, list_view_id: &str) -> Result<ListView> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !url_security::is_valid_salesforce_id(list_view_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        let path = format!("sobjects/{}/listviews/{}", sobject, list_view_id);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get detailed metadata for a list view, including columns and filters.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let describe = client.describe_list_view("Account", "00B000000000001AAA").await?;
    /// println!("Query: {}", describe.query);
    /// for column in describe.columns {
    ///     println!("Column: {}", column.field_name_or_path);
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn describe_list_view(
        &self,
        sobject: &str,
        list_view_id: &str,
    ) -> Result<ListViewDescribe> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !url_security::is_valid_salesforce_id(list_view_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        let path = format!("sobjects/{}/listviews/{}/describe", sobject, list_view_id);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Execute a list view and retrieve results.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let results: ListViewResult<serde_json::Value> =
    ///     client.execute_list_view("Account", "00B000000000001AAA").await?;
    /// println!("Found {} records", results.size);
    /// for record in results.records {
    ///     println!("{:?}", record);
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn execute_list_view<T: DeserializeOwned>(
        &self,
        sobject: &str,
        list_view_id: &str,
    ) -> Result<ListViewResult<T>> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !url_security::is_valid_salesforce_id(list_view_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        let path = format!("sobjects/{}/listviews/{}/results", sobject, list_view_id);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    // =========================================================================
    // Process Rules & Approvals
    // =========================================================================

    /// List all process rules available in the org.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let rules = client.list_process_rules().await?;
    /// for (sobject, rule_list) in &rules.rules {
    ///     for rule in rule_list {
    ///         println!("{} - {}: {}", sobject, rule.id, rule.name);
    ///     }
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn list_process_rules(&self) -> Result<ProcessRuleCollection> {
        self.client
            .rest_get("process/rules")
            .await
            .map_err(Into::into)
    }

    /// List process rules for a specific SObject.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let rules = client.list_process_rules_for_sobject("Account").await?;
    /// if let Some(account_rules) = rules.rules.get("Account") {
    ///     for rule in account_rules {
    ///         println!("{}: {}", rule.id, rule.name);
    ///     }
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn list_process_rules_for_sobject(
        &self,
        sobject: &str,
    ) -> Result<ProcessRuleCollection> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("process/rules/{}", sobject);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Trigger process rules for a specific record.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use sf_rest::process::ProcessRuleRequest;
    ///
    /// let request = ProcessRuleRequest {
    ///     context_id: "0015e000001234567".to_string(),
    /// };
    /// let result = client.trigger_process_rules(&request).await?;
    /// if result.success {
    ///     println!("Process rules triggered successfully");
    /// }
    /// ```
    #[instrument(skip(self, request))]
    pub async fn trigger_process_rules(
        &self,
        request: &ProcessRuleRequest,
    ) -> Result<ProcessRuleResult> {
        if !url_security::is_valid_salesforce_id(&request.context_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        self.client
            .rest_post("process/rules", request)
            .await
            .map_err(Into::into)
    }

    /// List pending approval requests.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let approvals = client.list_pending_approvals().await?;
    /// for (entity_type, approval_list) in &approvals.approvals {
    ///     for approval in approval_list {
    ///         println!("{} - Approval {}: Entity {}", entity_type, approval.id, approval.entity_id);
    ///     }
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn list_pending_approvals(&self) -> Result<PendingApprovalCollection> {
        self.client
            .rest_get("process/approvals")
            .await
            .map_err(Into::into)
    }

    /// Submit, approve, or reject an approval request.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use sf_rest::process::{ApprovalRequest, ApprovalActionType};
    ///
    /// let request = ApprovalRequest {
    ///     action_type: ApprovalActionType::Approve,
    ///     context_id: "0015e000001234567".to_string(),
    ///     context_actor_id: None,
    ///     comments: Some("Approved".to_string()),
    ///     next_approver_ids: None,
    ///     process_definition_name_or_id: None,
    ///     skip_entry_criteria: None,
    /// };
    /// let result = client.submit_approval(&request).await?;
    /// if result.success {
    ///     println!("Approval processed: {}", result.instance_status);
    /// }
    /// ```
    #[instrument(skip(self, request))]
    pub async fn submit_approval(&self, request: &ApprovalRequest) -> Result<ApprovalResult> {
        if !url_security::is_valid_salesforce_id(&request.context_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        self.client
            .rest_post("process/approvals", request)
            .await
            .map_err(Into::into)
    }

    // =========================================================================
    // Invocable Actions
    // =========================================================================

    /// List all standard invocable actions available in the org.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let actions = client.list_standard_actions().await?;
    /// for action in actions.actions {
    ///     println!("{}: {}", action.name, action.label);
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn list_standard_actions(&self) -> Result<InvocableActionCollection> {
        self.client
            .rest_get("actions/standard")
            .await
            .map_err(Into::into)
    }

    /// Get detailed metadata for a standard invocable action.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let action = client.describe_standard_action("emailSimple").await?;
    /// println!("Inputs: {:?}", action.inputs);
    /// ```
    #[instrument(skip(self))]
    pub async fn describe_standard_action(
        &self,
        action_name: &str,
    ) -> Result<InvocableActionDescribe> {
        if !soql::is_safe_sobject_name(action_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION_NAME".to_string(),
                message: "Invalid action name".to_string(),
            }));
        }
        let path = format!("actions/standard/{}", action_name);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Invoke a standard invocable action.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use sf_rest::actions::InvocableActionRequest;
    /// use serde_json::json;
    ///
    /// let request = InvocableActionRequest {
    ///     inputs: vec![json!({
    ///         "emailAddresses": "test@example.com",
    ///         "emailSubject": "Hello",
    ///         "emailBody": "Test"
    ///     })],
    /// };
    /// let result = client.invoke_standard_action("emailSimple", &request).await?;
    /// if result.is_success {
    ///     println!("Action invoked successfully");
    /// }
    /// ```
    #[instrument(skip(self, request))]
    pub async fn invoke_standard_action(
        &self,
        action_name: &str,
        request: &InvocableActionRequest,
    ) -> Result<InvocableActionResult> {
        if !soql::is_safe_sobject_name(action_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION_NAME".to_string(),
                message: "Invalid action name".to_string(),
            }));
        }
        let path = format!("actions/standard/{}", action_name);
        self.client
            .rest_post(&path, request)
            .await
            .map_err(Into::into)
    }

    /// List all custom invocable actions (Apex @InvocableMethod) available in the org.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let actions = client.list_custom_actions().await?;
    /// for action in actions.actions {
    ///     println!("{}: {}", action.name, action.label);
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn list_custom_actions(&self) -> Result<InvocableActionCollection> {
        self.client
            .rest_get("actions/custom")
            .await
            .map_err(Into::into)
    }

    /// Get detailed metadata for a custom invocable action.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let action = client.describe_custom_action("MyCustomAction").await?;
    /// println!("Inputs: {:?}", action.inputs);
    /// ```
    #[instrument(skip(self))]
    pub async fn describe_custom_action(
        &self,
        action_name: &str,
    ) -> Result<InvocableActionDescribe> {
        if !soql::is_safe_sobject_name(action_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION_NAME".to_string(),
                message: "Invalid action name".to_string(),
            }));
        }
        let path = format!("actions/custom/{}", action_name);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Invoke a custom invocable action.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use sf_rest::actions::InvocableActionRequest;
    /// use serde_json::json;
    ///
    /// let request = InvocableActionRequest {
    ///     inputs: vec![json!({"recordId": "0015e000001234567"})],
    /// };
    /// let result = client.invoke_custom_action("MyCustomAction", &request).await?;
    /// if result.is_success {
    ///     println!("Action invoked successfully");
    /// }
    /// ```
    #[instrument(skip(self, request))]
    pub async fn invoke_custom_action(
        &self,
        action_name: &str,
        request: &InvocableActionRequest,
    ) -> Result<InvocableActionResult> {
        if !soql::is_safe_sobject_name(action_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION_NAME".to_string(),
                message: "Invalid action name".to_string(),
            }));
        }
        let path = format!("actions/custom/{}", action_name);
        self.client
            .rest_post(&path, request)
            .await
            .map_err(Into::into)
    }

    // =========================================================================
    // Limits
    // =========================================================================

    /// Get API limits for the org.
    #[instrument(skip(self))]
    pub async fn limits(&self) -> Result<serde_json::Value> {
        self.client.rest_get("limits").await.map_err(Into::into)
    }

    // =========================================================================
    // API Versions
    // =========================================================================

    /// Get available API versions.
    #[instrument(skip(self))]
    pub async fn versions(&self) -> Result<Vec<ApiVersion>> {
        let url = format!("{}/services/data", self.client.instance_url());
        self.client.get_json(&url).await.map_err(Into::into)
    }
}

/// Result of a SOSL search.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SearchResult<T> {
    #[serde(rename = "searchRecords")]
    pub search_records: Vec<T>,
}

/// API version information.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ApiVersion {
    pub version: String,
    pub label: String,
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token123").unwrap();

        assert_eq!(client.instance_url(), "https://na1.salesforce.com");
        assert_eq!(client.api_version(), "62.0");
    }

    #[test]
    fn test_api_version_override() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token")
            .unwrap()
            .with_api_version("60.0");

        assert_eq!(client.api_version(), "60.0");
    }

    // =========================================================================
    // Wiremock HTTP Tests
    // =========================================================================

    #[tokio::test]
    async fn test_describe_layouts_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "layouts": [{"id": "00h000000000001", "name": "Account Layout"}],
            "recordTypeMappings": []
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account/describe/layouts$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .describe_layouts("Account")
            .await
            .expect("describe_layouts should succeed");

        assert!(result["layouts"].is_array());
        assert_eq!(result["layouts"][0]["name"], "Account Layout");
    }

    #[tokio::test]
    async fn test_describe_layouts_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.describe_layouts("Bad'; DROP--").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_describe_named_layout_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "layouts": [{"detailLayoutSections": [], "editLayoutSections": []}]
        });

        Mock::given(method("GET"))
            .and(path_regex(
                ".*/sobjects/Account/describe/namedLayouts/MyLayout",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .describe_named_layout("Account", "MyLayout")
            .await
            .expect("describe_named_layout should succeed");

        assert!(result["layouts"].is_array());
    }

    #[tokio::test]
    async fn test_describe_approval_layouts_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "approvalLayouts": []
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account/describe/approvalLayouts$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .describe_approval_layouts("Account")
            .await
            .expect("describe_approval_layouts should succeed");

        assert!(result["approvalLayouts"].is_array());
    }

    #[tokio::test]
    async fn test_describe_compact_layouts_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "compactLayouts": [{"id": "0AH000000000001", "name": "System Default"}],
            "defaultCompactLayoutId": "0AH000000000001"
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account/describe/compactLayouts$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .describe_compact_layouts("Account")
            .await
            .expect("describe_compact_layouts should succeed");

        assert!(result["compactLayouts"].is_array());
        assert_eq!(result["compactLayouts"][0]["name"], "System Default");
    }

    #[tokio::test]
    async fn test_describe_global_publisher_layouts_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "layouts": [{"id": "00h000000000002", "name": "Global Layout"}]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Global/describe/layouts$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .describe_global_publisher_layouts()
            .await
            .expect("describe_global_publisher_layouts should succeed");

        assert!(result["layouts"].is_array());
        assert_eq!(result["layouts"][0]["name"], "Global Layout");
    }
}
