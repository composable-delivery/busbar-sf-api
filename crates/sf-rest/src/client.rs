//! Salesforce REST API client.
//!
//! This client wraps `SalesforceClient` from `sf-client` and provides
//! typed methods for REST API operations including CRUD, Query, Describe,
//! Composite, and Collections.

use serde::{de::DeserializeOwned, Serialize};
use tracing::instrument;

use busbar_sf_client::security::{soql, url as url_security};
use busbar_sf_client::{ClientConfig, SalesforceClient};

use crate::collections::{CollectionRequest, CollectionResult};
use crate::composite::{
    CompositeBatchRequest, CompositeBatchResponse, CompositeRequest, CompositeResponse,
    CompositeTreeRequest, CompositeTreeResponse,
};
use crate::describe::{DescribeGlobalResult, DescribeSObjectResult};
use crate::error::{Error, ErrorKind, Result};
use crate::query::QueryResult;
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
    // Limits
    // =========================================================================

    /// Get API limits for the org.
    #[instrument(skip(self))]
    pub async fn limits(&self) -> Result<serde_json::Value> {
        self.client.rest_get("limits").await.map_err(Into::into)
    }

    // =========================================================================
    // Incremental Sync Operations
    // =========================================================================

    /// Get deleted records in a date range.
    ///
    /// Returns a list of records that were deleted between the start and end dates.
    /// This is useful for incremental data synchronization.
    ///
    /// The date range should be no more than 30 days and no earlier than 30 days ago.
    ///
    /// # Arguments
    ///
    /// * `sobject` - The SObject type (e.g., "Account", "Contact")
    /// * `start` - Start datetime in ISO 8601 format
    /// * `end` - End datetime in ISO 8601 format
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = client.get_deleted("Account", "2024-01-01T00:00:00Z", "2024-01-31T23:59:59Z").await?;
    /// for record in result.deleted_records {
    ///     println!("Deleted: {} at {}", record.id, record.deleted_date);
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn get_deleted(
        &self,
        sobject: &str,
        start: &str,
        end: &str,
    ) -> Result<GetDeletedResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!(
            "sobjects/{}/deleted/?start={}&end={}",
            sobject,
            urlencoding::encode(start),
            urlencoding::encode(end)
        );
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get updated records in a date range.
    ///
    /// Returns a list of IDs for records that were updated between the start and end dates.
    /// This is useful for incremental data synchronization.
    ///
    /// The date range should be no more than 30 days and no earlier than 30 days ago.
    ///
    /// # Arguments
    ///
    /// * `sobject` - The SObject type (e.g., "Account", "Contact")
    /// * `start` - Start datetime in ISO 8601 format
    /// * `end` - End datetime in ISO 8601 format
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = client.get_updated("Account", "2024-01-01T00:00:00Z", "2024-01-31T23:59:59Z").await?;
    /// for id in result.ids {
    ///     println!("Updated: {}", id);
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn get_updated(
        &self,
        sobject: &str,
        start: &str,
        end: &str,
    ) -> Result<GetUpdatedResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!(
            "sobjects/{}/updated/?start={}&end={}",
            sobject,
            urlencoding::encode(start),
            urlencoding::encode(end)
        );
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    // =========================================================================
    // Binary Content Operations
    // =========================================================================

    /// Retrieve binary content from a blob field.
    ///
    /// This retrieves the raw binary data from fields such as:
    /// - Attachment.Body
    /// - Document.Body
    /// - ContentVersion.VersionData
    ///
    /// # Arguments
    ///
    /// * `sobject` - The SObject type (e.g., "Attachment", "ContentVersion")
    /// * `id` - The record ID
    /// * `blob_field` - The blob field name (e.g., "Body", "VersionData")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let bytes = client.get_blob("Attachment", "00P7F00000ABC123", "Body").await?;
    /// std::fs::write("attachment.pdf", &bytes)?;
    /// ```
    #[instrument(skip(self))]
    pub async fn get_blob(&self, sobject: &str, id: &str, blob_field: &str) -> Result<Vec<u8>> {
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
        if !soql::is_safe_field_name(blob_field) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_FIELD".to_string(),
                message: "Invalid blob field name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/{}/{}", sobject, id, blob_field);
        let url = self.client.rest_url(&path);
        let request = self.client.get(&url);
        let response = self.client.execute(request).await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Retrieve a rich text image field.
    ///
    /// Retrieves an image embedded in a rich text area field.
    ///
    /// # Arguments
    ///
    /// * `sobject` - The SObject type
    /// * `id` - The record ID
    /// * `field_name` - The rich text field name
    /// * `content_reference_id` - The content reference ID for the image
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let image = client.get_rich_text_image(
    ///     "Account",
    ///     "001xx000003DGb2AAG",
    ///     "Description",
    ///     "069xx0000000001"
    /// ).await?;
    /// ```
    #[instrument(skip(self))]
    pub async fn get_rich_text_image(
        &self,
        sobject: &str,
        id: &str,
        field_name: &str,
        content_reference_id: &str,
    ) -> Result<Vec<u8>> {
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
        if !soql::is_safe_field_name(field_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_FIELD".to_string(),
                message: "Invalid field name".to_string(),
            }));
        }
        // Content reference IDs are typically numeric, but validate as a field name for safety
        if !soql::is_safe_field_name(content_reference_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_CONTENT_REFERENCE_ID".to_string(),
                message: "Invalid content reference ID".to_string(),
            }));
        }
        let path = format!(
            "sobjects/{}/{}/richTextImageFields/{}/{}",
            sobject, id, field_name, content_reference_id
        );
        let url = self.client.rest_url(&path);
        let request = self.client.get(&url);
        let response = self.client.execute(request).await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    // =========================================================================
    // Relationship Traversal
    // =========================================================================

    /// Traverse an SObject relationship by path.
    ///
    /// Navigate child relationships or lookup relationships directly through the REST API.
    /// For child relationships, this returns a QueryResult. For lookup relationships,
    /// this returns the related record.
    ///
    /// # Arguments
    ///
    /// * `sobject` - The SObject type
    /// * `id` - The record ID
    /// * `relationship_name` - The relationship name (e.g., "Contacts", "Account")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Get child contacts for an account
    /// let contacts: QueryResult<serde_json::Value> = client
    ///     .get_relationship("Account", "001xx000003DGb2AAG", "Contacts")
    ///     .await?;
    ///
    /// // Get parent account for a contact
    /// let account: serde_json::Value = client
    ///     .get_relationship("Contact", "003xx000004TmiQAAS", "Account")
    ///     .await?;
    /// ```
    #[instrument(skip(self))]
    pub async fn get_relationship<T: DeserializeOwned>(
        &self,
        sobject: &str,
        id: &str,
        relationship_name: &str,
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
        if !soql::is_safe_field_name(relationship_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_RELATIONSHIP".to_string(),
                message: "Invalid relationship name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/{}/{}", sobject, id, relationship_name);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    // =========================================================================
    // SObject Metadata
    // =========================================================================

    /// Get basic information about an SObject.
    ///
    /// Returns recent items and metadata URLs for the SObject.
    /// This is different from `describe_sobject` which returns full metadata.
    ///
    /// # Arguments
    ///
    /// * `sobject` - The SObject type (e.g., "Account", "Contact")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let info = client.get_sobject_basic_info("Account").await?;
    /// println!("Recent items: {}", info.recent_items.len());
    /// ```
    #[instrument(skip(self))]
    pub async fn get_sobject_basic_info(&self, sobject: &str) -> Result<SObjectInfo> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}", sobject);
        self.client.rest_get(&path).await.map_err(Into::into)
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

// =========================================================================
// Incremental Sync Types
// =========================================================================

/// Result of a getDeleted request.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GetDeletedResult {
    /// List of deleted records.
    #[serde(rename = "deletedRecords")]
    pub deleted_records: Vec<DeletedRecord>,

    /// Earliest date available for the getDeleted API.
    #[serde(rename = "earliestDateAvailable")]
    pub earliest_date_available: String,

    /// Latest date covered by this query.
    #[serde(rename = "latestDateCovered")]
    pub latest_date_covered: String,
}

/// A deleted record.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DeletedRecord {
    /// Record ID.
    pub id: String,

    /// Date when the record was deleted (ISO 8601 format).
    #[serde(rename = "deletedDate")]
    pub deleted_date: String,
}

/// Result of a getUpdated request.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GetUpdatedResult {
    /// List of IDs of records that were updated.
    pub ids: Vec<String>,

    /// Latest date covered by this query.
    #[serde(rename = "latestDateCovered")]
    pub latest_date_covered: String,
}

// =========================================================================
// SObject Basic Information Types
// =========================================================================

/// Basic information about an SObject (from GET /sobjects/{SObjectType}).
///
/// This is different from the describe endpoint - it returns recent items
/// and metadata URLs rather than full field metadata.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SObjectInfo {
    /// Basic describe information.
    #[serde(rename = "objectDescribe")]
    pub object_describe: SObjectInfoDescribe,

    /// Recently accessed items.
    #[serde(rename = "recentItems")]
    pub recent_items: Vec<serde_json::Value>,
}

/// Basic describe information from SObject info endpoint.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SObjectInfoDescribe {
    /// SObject name.
    pub name: String,

    /// Display label.
    pub label: String,

    /// Key prefix for IDs.
    #[serde(rename = "keyPrefix")]
    pub key_prefix: Option<String>,

    /// URL map with links to describe, sobject, and other resources.
    pub urls: std::collections::HashMap<String, String>,

    /// Whether this is a custom object.
    pub custom: bool,

    /// CRUD and other capability flags.
    pub createable: bool,
    pub updateable: bool,
    pub deletable: bool,
    pub queryable: bool,
    pub searchable: bool,
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

    #[test]
    fn test_get_deleted_result_deserialization() {
        let json = r#"{
            "deletedRecords": [
                {
                    "id": "001xx000003DGb2AAG",
                    "deletedDate": "2024-01-15T10:30:00.000+0000"
                }
            ],
            "earliestDateAvailable": "2024-01-01T00:00:00.000+0000",
            "latestDateCovered": "2024-01-31T23:59:59.000+0000"
        }"#;

        let result: GetDeletedResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.deleted_records.len(), 1);
        assert_eq!(result.deleted_records[0].id, "001xx000003DGb2AAG");
        assert_eq!(
            result.deleted_records[0].deleted_date,
            "2024-01-15T10:30:00.000+0000"
        );
        assert_eq!(
            result.earliest_date_available,
            "2024-01-01T00:00:00.000+0000"
        );
        assert_eq!(result.latest_date_covered, "2024-01-31T23:59:59.000+0000");
    }

    #[test]
    fn test_get_updated_result_deserialization() {
        let json = r#"{
            "ids": [
                "001xx000003DGb2AAG",
                "001xx000003DGb3AAG"
            ],
            "latestDateCovered": "2024-01-31T23:59:59.000+0000"
        }"#;

        let result: GetUpdatedResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.ids.len(), 2);
        assert_eq!(result.ids[0], "001xx000003DGb2AAG");
        assert_eq!(result.ids[1], "001xx000003DGb3AAG");
        assert_eq!(result.latest_date_covered, "2024-01-31T23:59:59.000+0000");
    }

    #[test]
    fn test_sobject_info_deserialization() {
        let json = r#"{
            "objectDescribe": {
                "name": "Account",
                "label": "Account",
                "keyPrefix": "001",
                "urls": {
                    "sobject": "/services/data/v62.0/sobjects/Account",
                    "describe": "/services/data/v62.0/sobjects/Account/describe"
                },
                "custom": false,
                "createable": true,
                "updateable": true,
                "deletable": true,
                "queryable": true,
                "searchable": true
            },
            "recentItems": [
                {
                    "Id": "001xx000003DGb2AAG",
                    "Name": "Test Account"
                }
            ]
        }"#;

        let result: SObjectInfo = serde_json::from_str(json).unwrap();
        assert_eq!(result.object_describe.name, "Account");
        assert_eq!(result.object_describe.label, "Account");
        assert_eq!(result.object_describe.key_prefix, Some("001".to_string()));
        assert!(!result.object_describe.custom);
        assert_eq!(result.recent_items.len(), 1);
    }

    // =========================================================================
    // Wiremock HTTP Tests
    // =========================================================================

    #[tokio::test]
    async fn test_get_deleted_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "deletedRecords": [
                {"id": "001xx000003DGb2AAG", "deletedDate": "2024-01-15T10:30:00.000+0000"}
            ],
            "earliestDateAvailable": "2024-01-01T00:00:00.000+0000",
            "latestDateCovered": "2024-01-31T23:59:59.000+0000"
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account/deleted/"))
            "layouts": [{"id": "00h000000000001", "name": "Account Layout"}],
            "recordTypeMappings": []
        });

    async fn test_describe_layouts_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account/describe/layouts$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .get_deleted("Account", "2024-01-01T00:00:00Z", "2024-01-31T23:59:59Z")
            .await
            .expect("get_deleted should succeed");

        assert_eq!(result.deleted_records.len(), 1);
        assert_eq!(result.deleted_records[0].id, "001xx000003DGb2AAG");
    }

    #[tokio::test]
    async fn test_get_deleted_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .get_deleted(
                "Bad'; DROP--",
                "2024-01-01T00:00:00Z",
                "2024-01-31T23:59:59Z",
            )
            .await;
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
    async fn test_get_updated_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "ids": ["001xx000003DGb2AAG", "001xx000003DGb3AAG"],
            "latestDateCovered": "2024-01-31T23:59:59.000+0000"
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account/updated/"))
            "layouts": [{"detailLayoutSections": [], "editLayoutSections": []}]
        });
    async fn test_describe_named_layout_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(
                ".*/sobjects/Account/describe/namedLayouts/MyLayout",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .get_updated("Account", "2024-01-01T00:00:00Z", "2024-01-31T23:59:59Z")
            .await
            .expect("get_updated should succeed");

        assert_eq!(result.ids.len(), 2);
        assert_eq!(result.ids[0], "001xx000003DGb2AAG");
    }

    #[tokio::test]
    async fn test_get_updated_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .get_updated(
                "Bad'; DROP--",
                "2024-01-01T00:00:00Z",
                "2024-01-31T23:59:59Z",
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_get_blob_wiremock() {
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
        let binary_data = vec![0x89, 0x50, 0x4E, 0x47]; // PNG header bytes

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Attachment/001xx000003DGb2AAG/Body"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(binary_data.clone()))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .get_blob("Attachment", "001xx000003DGb2AAG", "Body")
            .await
            .expect("get_blob should succeed");

        assert_eq!(result, binary_data);
    }

    #[tokio::test]
    async fn test_get_blob_invalid_id() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.get_blob("Attachment", "bad-id!", "Body").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_ID"));
    }

    #[tokio::test]
    async fn test_get_blob_invalid_field() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .get_blob("Attachment", "001xx000003DGb2AAG", "Bad;Field")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_FIELD"));
    }

    #[tokio::test]
    async fn test_get_rich_text_image_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let image_data = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG header

        Mock::given(method("GET"))
            .and(path_regex(
                ".*/sobjects/Account/001xx000003DGb2AAG/richTextImageFields/Description/refId001",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(image_data.clone()))

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
            .get_rich_text_image("Account", "001xx000003DGb2AAG", "Description", "refId001")
            .await
            .expect("get_rich_text_image should succeed");

        assert_eq!(result, image_data);
    }

    #[tokio::test]
    async fn test_get_rich_text_image_invalid_id() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .get_rich_text_image("Account", "bad!", "Description", "refId001")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_ID"));
    }

    #[tokio::test]
    async fn test_get_relationship_wiremock() {
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
            "totalSize": 2,
            "done": true,
            "records": [
                {"Id": "003xx1", "Name": "Contact 1"},
                {"Id": "003xx2", "Name": "Contact 2"}
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(
                ".*/sobjects/Account/001xx000003DGb2AAG/Contacts",
            ))
            "compactLayouts": [{"id": "0AH000000000001", "name": "System Default"}],
            "defaultCompactLayoutId": "0AH000000000001"
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account/describe/compactLayouts$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result: serde_json::Value = client
            .get_relationship("Account", "001xx000003DGb2AAG", "Contacts")
            .await
            .expect("get_relationship should succeed");

        assert_eq!(result["totalSize"], 2);
        assert_eq!(result["records"][0]["Name"], "Contact 1");
    }

    #[tokio::test]
    async fn test_get_relationship_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result: std::result::Result<serde_json::Value, _> = client
            .get_relationship("Bad'; DROP--", "001xx000003DGb2AAG", "Contacts")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_get_sobject_basic_info_wiremock() {
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
            "objectDescribe": {
                "name": "Account",
                "label": "Account",
                "keyPrefix": "001",
                "urls": {"sobject": "/services/data/v62.0/sobjects/Account"},
                "custom": false,
                "createable": true,
                "updateable": true,
                "deletable": true,
                "queryable": true,
                "searchable": true
            },
            "recentItems": []
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account$"))
            "layouts": [{"id": "00h000000000002", "name": "Global Layout"}]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Global/describe/layouts$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .get_sobject_basic_info("Account")
            .await
            .expect("get_sobject_basic_info should succeed");

        assert_eq!(result.object_describe.name, "Account");
        assert!(result.object_describe.queryable);
        assert!(result.recent_items.is_empty());
    }

    #[tokio::test]
    async fn test_get_sobject_basic_info_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.get_sobject_basic_info("Bad'; DROP--").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
            .describe_global_publisher_layouts()
            .await
            .expect("describe_global_publisher_layouts should succeed");

        assert!(result["layouts"].is_array());
        assert_eq!(result["layouts"][0]["name"], "Global Layout");
    }
}
