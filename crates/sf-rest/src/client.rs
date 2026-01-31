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
    // API Versions
    // =========================================================================

    /// Get available API versions.
    #[instrument(skip(self))]
    pub async fn versions(&self) -> Result<Vec<ApiVersion>> {
        let url = format!("{}/services/data", self.client.instance_url());
        self.client.get_json(&url).await.map_err(Into::into)
    }

    // =========================================================================
    // Consent API
    // =========================================================================

    /// Read consent status for records for a specific action.
    ///
    /// # Arguments
    ///
    /// * `action` - The consent action name
    /// * `ids` - Comma-separated list of record IDs
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let response = client.read_consent("marketing_email", "001xx000003DHP0AAO,001xx000003DHP1AAO").await?;
    /// ```
    #[instrument(skip(self))]
    pub async fn read_consent(
        &self,
        action: &str,
        ids: &str,
    ) -> Result<crate::consent::ConsentResponse> {
        // Validate action name is safe
        if !soql::is_safe_field_name(action) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION".to_string(),
                message: "Invalid consent action name".to_string(),
            }));
        }
        let encoded_ids = url_security::encode_param(ids);
        let path = format!("consent/action/{}?ids={}", action, encoded_ids);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Write consent for records for a specific action.
    ///
    /// # Arguments
    ///
    /// * `action` - The consent action name
    /// * `request` - The consent write request with record IDs and consent values
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use sf_rest::consent::{ConsentWriteRequest, ConsentWriteRecord};
    ///
    /// let request = ConsentWriteRequest {
    ///     consents: vec![
    ///         ConsentWriteRecord {
    ///             id: "001xx000003DHP0AAO".to_string(),
    ///             consent: true,
    ///         }
    ///     ]
    /// };
    /// let response = client.write_consent("marketing_email", &request).await?;
    /// ```
    #[instrument(skip(self, request))]
    pub async fn write_consent(
        &self,
        action: &str,
        request: &crate::consent::ConsentWriteRequest,
    ) -> Result<crate::consent::ConsentResponse> {
        // Validate action name is safe
        if !soql::is_safe_field_name(action) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION".to_string(),
                message: "Invalid consent action name".to_string(),
            }));
        }
        let path = format!("consent/action/{}", action);
        self.client
            .rest_post(&path, request)
            .await
            .map_err(Into::into)
    }

    /// Read multiple consent actions for records.
    ///
    /// # Arguments
    ///
    /// * `actions` - Comma-separated list of consent action names
    /// * `ids` - Comma-separated list of record IDs
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let response = client.read_multi_consent("marketing_email,sms", "001xx000003DHP0AAO").await?;
    /// ```
    #[instrument(skip(self))]
    pub async fn read_multi_consent(
        &self,
        actions: &str,
        ids: &str,
    ) -> Result<crate::consent::MultiConsentResponse> {
        let encoded_actions = url_security::encode_param(actions);
        let encoded_ids = url_security::encode_param(ids);
        let path = format!(
            "consent/multiaction?actions={}&ids={}",
            encoded_actions, encoded_ids
        );
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    // =========================================================================
    // Knowledge Management
    // =========================================================================

    /// Get Knowledge Management settings for the org.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let settings = client.knowledge_management_settings().await?;
    /// println!("Knowledge enabled: {}", settings.is_enabled);
    /// ```
    #[instrument(skip(self))]
    pub async fn knowledge_management_settings(
        &self,
    ) -> Result<crate::knowledge::KnowledgeSettings> {
        self.client
            .rest_get("knowledgeManagement/settings")
            .await
            .map_err(Into::into)
    }

    /// List knowledge articles.
    ///
    /// Returns a list of available knowledge articles based on search criteria.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let articles = client.list_knowledge_articles().await?;
    /// ```
    #[instrument(skip(self))]
    pub async fn list_knowledge_articles(
        &self,
    ) -> Result<crate::knowledge::KnowledgeArticlesResponse> {
        self.client
            .rest_get("support/knowledgeArticles")
            .await
            .map_err(Into::into)
    }

    /// Get a specific knowledge article.
    ///
    /// # Arguments
    ///
    /// * `article_id` - The knowledge article ID
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let article = client.get_knowledge_article("kA0xx0000000001").await?;
    /// ```
    #[instrument(skip(self))]
    pub async fn get_knowledge_article(
        &self,
        article_id: &str,
    ) -> Result<crate::knowledge::KnowledgeArticle> {
        if !url_security::is_valid_salesforce_id(article_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid article ID format".to_string(),
            }));
        }
        let path = format!("support/knowledgeArticles/{}", article_id);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// List data category groups.
    ///
    /// Returns available data category groups for organizing knowledge articles.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let groups = client.list_data_category_groups().await?;
    /// ```
    #[instrument(skip(self))]
    pub async fn list_data_category_groups(
        &self,
    ) -> Result<crate::knowledge::DataCategoryGroupsResponse> {
        self.client
            .rest_get("support/dataCategoryGroups")
            .await
            .map_err(Into::into)
    }

    /// List data categories within a specific group.
    ///
    /// # Arguments
    ///
    /// * `group` - The data category group name
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let categories = client.list_data_categories("Products").await?;
    /// ```
    #[instrument(skip(self))]
    pub async fn list_data_categories(
        &self,
        group: &str,
    ) -> Result<crate::knowledge::DataCategoriesResponse> {
        // Validate group name is safe
        if !soql::is_safe_field_name(group) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_GROUP".to_string(),
                message: "Invalid data category group name".to_string(),
            }));
        }
        let path = format!("support/dataCategoryGroups/{}/dataCategories", group);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    // =========================================================================
    // User Password Management
    // =========================================================================

    /// Get password expiration status for a user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The User record ID
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let status = client.get_user_password_status("005xx000001X8Uz").await?;
    /// println!("Password expired: {}", status.is_expired);
    /// ```
    #[instrument(skip(self))]
    pub async fn get_user_password_status(
        &self,
        user_id: &str,
    ) -> Result<crate::user_password::UserPasswordStatus> {
        if !url_security::is_valid_salesforce_id(user_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid User ID format".to_string(),
            }));
        }
        let path = format!("sobjects/User/{}/password", user_id);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Set a user's password.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The User record ID
    /// * `new_password` - The new password to set
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use sf_rest::user_password::SetPasswordRequest;
    ///
    /// let request = SetPasswordRequest {
    ///     new_password: "NewSecurePassword123!".to_string(),
    /// };
    /// client.set_user_password("005xx000001X8Uz", &request).await?;
    /// ```
    #[instrument(skip(self, request))]
    pub async fn set_user_password(
        &self,
        user_id: &str,
        request: &crate::user_password::SetPasswordRequest,
    ) -> Result<crate::user_password::SetPasswordResponse> {
        if !url_security::is_valid_salesforce_id(user_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid User ID format".to_string(),
            }));
        }
        let path = format!("sobjects/User/{}/password", user_id);
        self.client
            .rest_post(&path, request)
            .await
            .map_err(Into::into)
    }

    /// Reset a user's password (generates a new password).
    ///
    /// # Arguments
    ///
    /// * `user_id` - The User record ID
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let response = client.reset_user_password("005xx000001X8Uz").await?;
    /// println!("New password: {:?}", response.new_password);
    /// ```
    #[instrument(skip(self))]
    pub async fn reset_user_password(
        &self,
        user_id: &str,
    ) -> Result<crate::user_password::SetPasswordResponse> {
        if !url_security::is_valid_salesforce_id(user_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid User ID format".to_string(),
            }));
        }
        let path = format!("sobjects/User/{}/password", user_id);
        let url = self.client.rest_url(&path);
        let request = self.client.delete(&url);
        let response = self.client.execute(request).await?;
        response.json().await.map_err(Into::into)
    }

    // =========================================================================
    // Suggested Articles & Platform Actions
    // =========================================================================

    /// Get suggested knowledge articles for a case or other SObject.
    ///
    /// # Arguments
    ///
    /// * `sobject_type` - The SObject type (e.g., "Case")
    /// * `subject` - Optional subject text for article suggestions
    /// * `description` - Optional description text for article suggestions
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let articles = client.get_suggested_articles(
    ///     "Case",
    ///     Some("How to reset password"),
    ///     Some("User cannot log in")
    /// ).await?;
    /// ```
    #[instrument(skip(self))]
    pub async fn get_suggested_articles(
        &self,
        sobject_type: &str,
        subject: Option<&str>,
        description: Option<&str>,
    ) -> Result<crate::actions::SuggestedArticlesResponse> {
        if !soql::is_safe_sobject_name(sobject_type) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let mut path = format!("sobjects/{}/suggestedArticles", sobject_type);
        let mut params = Vec::new();
        if let Some(s) = subject {
            params.push(format!("subject={}", url_security::encode_param(s)));
        }
        if let Some(d) = description {
            params.push(format!("description={}", url_security::encode_param(d)));
        }
        if !params.is_empty() {
            path.push('?');
            path.push_str(&params.join("&"));
        }
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get platform actions available for an SObject type.
    ///
    /// # Arguments
    ///
    /// * `sobject_type` - The SObject type (e.g., "Account", "Contact")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let actions = client.get_platform_actions("Account").await?;
    /// ```
    #[instrument(skip(self))]
    pub async fn get_platform_actions(
        &self,
        sobject_type: &str,
    ) -> Result<crate::actions::PlatformActionsResponse> {
        if !soql::is_safe_sobject_name(sobject_type) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/platformAction", sobject_type);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    // =========================================================================
    // Salesforce Scheduler
    // =========================================================================

    /// Get available appointment slots.
    ///
    /// Returns available time slots for scheduling appointments.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let slots = client.get_appointment_slots().await?;
    /// ```
    #[instrument(skip(self))]
    pub async fn get_appointment_slots(&self) -> Result<serde_json::Value> {
        self.client
            .rest_get("scheduling/getAppointmentSlots")
            .await
            .map_err(Into::into)
    }

    /// Get appointment candidates based on scheduling criteria.
    ///
    /// # Arguments
    ///
    /// * `request` - The appointment candidates request with scheduling parameters
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use sf_rest::scheduler::AppointmentCandidatesRequest;
    ///
    /// let request = AppointmentCandidatesRequest {
    ///     scheduling_policy_id: Some("0VsB000000001".to_string()),
    ///     work_type_id: Some("08qB000000001".to_string()),
    ///     account_id: Some("001B000000001".to_string()),
    ///     additional: std::collections::HashMap::new(),
    /// };
    /// let candidates = client.get_appointment_candidates(&request).await?;
    /// ```
    #[instrument(skip(self, request))]
    pub async fn get_appointment_candidates(
        &self,
        request: &crate::scheduler::AppointmentCandidatesRequest,
    ) -> Result<crate::scheduler::AppointmentCandidatesResponse> {
        self.client
            .rest_post("scheduling/getAppointmentCandidates", request)
            .await
            .map_err(Into::into)
    }

    // =========================================================================
    // Embedded Service
    // =========================================================================

    /// Get embedded service configuration.
    ///
    /// # Arguments
    ///
    /// * `config_id` - The embedded service configuration ID
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = client.get_embedded_service_config("0ESxx000000001").await?;
    /// ```
    #[instrument(skip(self))]
    pub async fn get_embedded_service_config(
        &self,
        config_id: &str,
    ) -> Result<crate::embedded_service::EmbeddedServiceConfig> {
        if !url_security::is_valid_salesforce_id(config_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid configuration ID format".to_string(),
            }));
        }
        let path = format!("support/embeddedservice/configuration/{}", config_id);
        self.client.rest_get(&path).await.map_err(Into::into)
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

    async fn test_read_consent_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "consents": [
                {"id": "001xx000003DHP0AAO", "consent": true},
                {"id": "001xx000003DHP1AAO", "consent": false, "error": "No consent found"}
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/consent/action/marketing_email"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .read_consent("marketing_email", "001xx000003DHP0AAO,001xx000003DHP1AAO")
            .await
            .expect("should succeed");
        assert_eq!(result.consents.len(), 2);
        assert!(result.consents[0].consent);
        assert!(!result.consents[1].consent);
        assert_eq!(
            result.consents[1].error.as_deref(),
            Some("No consent found")
        );
    }

    #[tokio::test]
    async fn test_read_consent_invalid_action() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        let result = client
            .read_consent("Robert'; DROP TABLE--", "001xx000003DHP0AAO")
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_ACTION"),
            "Expected INVALID_ACTION, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_write_consent_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let response_body = serde_json::json!({
            "consents": [
                {"id": "001xx000003DHP0AAO", "consent": true}
            ]
        });

        Mock::given(method("POST"))
            .and(path_regex(".*/consent/action/marketing_email"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let request = crate::consent::ConsentWriteRequest {
            consents: vec![crate::consent::ConsentWriteRecord {
                id: "001xx000003DHP0AAO".to_string(),
                consent: true,
            }],
        };
        let result = client
            .write_consent("marketing_email", &request)
            .await
            .expect("should succeed");
        assert_eq!(result.consents.len(), 1);
        assert!(result.consents[0].consent);
    }

    #[tokio::test]
    async fn test_read_multi_consent_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "marketing_email": {
                "consents": [{"id": "001xx000003DHP0AAO", "consent": true}]
            },
            "sms": {
                "consents": [{"id": "001xx000003DHP0AAO", "consent": false}]
            }
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/consent/multiaction"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .read_multi_consent("marketing_email,sms", "001xx000003DHP0AAO")
            .await
            .expect("should succeed");
        assert!(result.actions.contains_key("marketing_email"));
        assert!(result.actions.contains_key("sms"));
        assert!(result.actions["marketing_email"].consents[0].consent);
        assert!(!result.actions["sms"].consents[0].consent);
    }

    // =========================================================================
    // Wiremock HTTP tests — Knowledge Management
    // =========================================================================

    #[tokio::test]
    async fn test_knowledge_management_settings_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "isEnabled": true,
            "defaultLanguage": "en_US"
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/knowledgeManagement/settings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .knowledge_management_settings()
            .await
            .expect("should succeed");
        assert!(result.is_enabled);
        assert_eq!(result.default_language.as_deref(), Some("en_US"));
    }

    #[tokio::test]
    async fn test_list_knowledge_articles_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "articles": [
                {"id": "kA0xx0000000001AAA", "title": "How to Reset Password"},
                {"id": "kA0xx0000000002AAA", "title": "Getting Started Guide"}
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/support/knowledgeArticles$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .list_knowledge_articles()
            .await
            .expect("should succeed");
        assert_eq!(result.articles.len(), 2);
        assert_eq!(result.articles[0].title, "How to Reset Password");
    }

    #[tokio::test]
    async fn test_get_knowledge_article_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "id": "kA0xx0000000001AAA",
            "title": "How to Reset Password",
            "urlName": "reset-password",
            "articleType": "Knowledge__kav"
        });

        Mock::given(method("GET"))
            .and(path_regex(
                ".*/support/knowledgeArticles/kA0xx0000000001AAA",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .get_knowledge_article("kA0xx0000000001AAA")
            .await
            .expect("should succeed");
        assert_eq!(result.id, "kA0xx0000000001AAA");
        assert_eq!(result.title, "How to Reset Password");
        assert_eq!(result.url_name.as_deref(), Some("reset-password"));
    }

    #[tokio::test]
    async fn test_get_knowledge_article_invalid_id() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        let result = client.get_knowledge_article("not-valid!").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_ID"),
            "Expected INVALID_ID, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_list_data_category_groups_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "categoryGroups": [
                {"name": "Products", "label": "Products", "active": true},
                {"name": "Geography", "label": "Geography", "active": true}
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/support/dataCategoryGroups$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .list_data_category_groups()
            .await
            .expect("should succeed");
        assert_eq!(result.category_groups.len(), 2);
        assert_eq!(result.category_groups[0].name, "Products");
    }

    #[tokio::test]
    async fn test_list_data_categories_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "categories": [
                {"name": "Laptops", "label": "Laptops", "parentName": "Products"},
                {"name": "Phones", "label": "Phones", "parentName": "Products"}
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(
                ".*/support/dataCategoryGroups/Products/dataCategories",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .list_data_categories("Products")
            .await
            .expect("should succeed");
        assert_eq!(result.categories.len(), 2);
        assert_eq!(result.categories[0].name, "Laptops");
    }

    #[tokio::test]
    async fn test_list_data_categories_invalid_group() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        let result = client.list_data_categories("Robert'; DROP TABLE--").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_GROUP"),
            "Expected INVALID_GROUP, got: {err}"
        );
    }

    // =========================================================================
    // Wiremock HTTP tests — User Password Management
    // =========================================================================

    #[tokio::test]
    async fn test_get_user_password_status_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({"isExpired": false});

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/User/005xx000001X8UzAAK/password"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .get_user_password_status("005xx000001X8UzAAK")
            .await
            .expect("should succeed");
        assert!(!result.is_expired);
    }

    #[tokio::test]
    async fn test_get_user_password_status_invalid_id() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        let result = client.get_user_password_status("bad-id!").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_ID"),
            "Expected INVALID_ID, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_set_user_password_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let response_body = serde_json::json!({});

        Mock::given(method("POST"))
            .and(path_regex(".*/sobjects/User/005xx000001X8UzAAK/password"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let request = crate::user_password::SetPasswordRequest {
            new_password: "NewSecurePassword123!".to_string(),
        };
        let result = client
            .set_user_password("005xx000001X8UzAAK", &request)
            .await
            .expect("should succeed");
        assert!(result.new_password.is_none());
    }

    #[tokio::test]
    async fn test_reset_user_password_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({"NewPassword": "AutoGen123!"});

        Mock::given(method("DELETE"))
            .and(path_regex(".*/sobjects/User/005xx000001X8UzAAK/password"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .reset_user_password("005xx000001X8UzAAK")
            .await
            .expect("should succeed");
        assert_eq!(result.new_password.as_deref(), Some("AutoGen123!"));
    }

    // =========================================================================
    // Wiremock HTTP tests — Suggested Articles & Platform Actions
    // =========================================================================

    #[tokio::test]
    async fn test_get_suggested_articles_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "articles": [
                {"id": "kA0xx0000000001AAA", "title": "Password Reset Guide", "score": 0.95},
                {"id": "kA0xx0000000002AAA", "title": "Login Troubleshooting", "score": 0.80}
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Case/suggestedArticles"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .get_suggested_articles("Case", Some("Password reset"), Some("Cannot log in"))
            .await
            .expect("should succeed");
        assert_eq!(result.articles.len(), 2);
        assert_eq!(result.articles[0].title, "Password Reset Guide");
        assert_eq!(result.articles[0].score, Some(0.95));
    }

    #[tokio::test]
    async fn test_get_suggested_articles_invalid_sobject() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        let result = client
            .get_suggested_articles("Robert'; DROP TABLE--", None, None)
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_SOBJECT"),
            "Expected INVALID_SOBJECT, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_get_platform_actions_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "actions": [
                {"name": "NewCase", "label": "New Case", "actionType": "QuickAction", "available": true},
                {"name": "SendEmail", "label": "Send Email", "actionType": "QuickAction", "available": true}
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account/platformAction"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .get_platform_actions("Account")
            .await
            .expect("should succeed");
        assert_eq!(result.actions.len(), 2);
        assert_eq!(result.actions[0].name, "NewCase");
        assert_eq!(result.actions[0].label.as_deref(), Some("New Case"));
    }

    #[tokio::test]
    async fn test_get_platform_actions_invalid_sobject() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        let result = client.get_platform_actions("Robert'; DROP TABLE--").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_SOBJECT"),
            "Expected INVALID_SOBJECT, got: {err}"
        );
    }

    // =========================================================================
    // Wiremock HTTP tests — Scheduler
    // =========================================================================

    #[tokio::test]
    async fn test_get_appointment_slots_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "slots": [
                {"startTime": "2026-02-01T09:00:00.000Z", "endTime": "2026-02-01T10:00:00.000Z"},
                {"startTime": "2026-02-01T10:00:00.000Z", "endTime": "2026-02-01T11:00:00.000Z"}
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/scheduling/getAppointmentSlots"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .get_appointment_slots()
            .await
            .expect("should succeed");
        assert!(result.get("slots").is_some());
        assert_eq!(result["slots"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_get_appointment_candidates_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "candidates": [
                {
                    "startTime": "2026-02-01T09:00:00.000Z",
                    "endTime": "2026-02-01T10:00:00.000Z"
                }
            ]
        });

        Mock::given(method("POST"))
            .and(path_regex(".*/scheduling/getAppointmentCandidates"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let request = crate::scheduler::AppointmentCandidatesRequest {
            scheduling_policy_id: Some("0VsB000000001AAA".to_string()),
            work_type_id: Some("08qB000000001AAA".to_string()),
            account_id: Some("001B000000001AAA".to_string()),
            additional: std::collections::HashMap::new(),
        };
        let result = client
            .get_appointment_candidates(&request)
            .await
            .expect("should succeed");
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(
            result.candidates[0].start_time.as_deref(),
            Some("2026-02-01T09:00:00.000Z")
        );
    }

    // =========================================================================
    // Wiremock HTTP tests — Embedded Service
    // =========================================================================

    #[tokio::test]
    async fn test_get_embedded_service_config_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "id": "0ESxx0000000001AAA",
            "name": "My_Embedded_Chat",
            "isEnabled": true,
            "siteUrl": "https://myorg.my.site.com"
        });

        Mock::given(method("GET"))
            .and(path_regex(
                ".*/support/embeddedservice/configuration/0ESxx0000000001AAA",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .get_embedded_service_config("0ESxx0000000001AAA")
            .await
            .expect("should succeed");
        assert_eq!(result.id, "0ESxx0000000001AAA");
        assert_eq!(result.name.as_deref(), Some("My_Embedded_Chat"));
        assert_eq!(result.is_enabled, Some(true));
    }

    #[tokio::test]
    async fn test_get_embedded_service_config_invalid_id() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        let result = client.get_embedded_service_config("bad-id!").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_ID"),
            "Expected INVALID_ID, got: {err}"
        );
    }
}
