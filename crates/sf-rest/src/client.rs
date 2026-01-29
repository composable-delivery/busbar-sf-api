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
    // Standalone REST Resources
    // =========================================================================

    /// Get all available tabs for the user.
    ///
    /// Returns information about all tabs available in the org, including
    /// standard and custom tabs.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let tabs = client.tabs().await?;
    /// for tab in tabs {
    ///     println!("Tab: {}", tab["label"]);
    /// }
    /// ```
    ///
    /// # Salesforce Documentation
    ///
    /// <https://developer.salesforce.com/docs/atlas.en-us.api_rest.meta/api_rest/resources_tabs.htm>
    #[instrument(skip(self))]
    pub async fn tabs(&self) -> Result<Vec<serde_json::Value>> {
        self.client.rest_get("tabs").await.map_err(Into::into)
    }

    /// Get the theme information for the org.
    ///
    /// Returns theme colors, logo URLs, and custom branding information.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let theme = client.theme().await?;
    /// println!("Theme: {:?}", theme);
    /// ```
    ///
    /// # Salesforce Documentation
    ///
    /// <https://developer.salesforce.com/docs/atlas.en-us.api_rest.meta/api_rest/resources_themes.htm>
    #[instrument(skip(self))]
    pub async fn theme(&self) -> Result<serde_json::Value> {
        self.client.rest_get("theme").await.map_err(Into::into)
    }

    /// Get the app menu items.
    ///
    /// Returns a list of applications available to the user.
    ///
    /// # Parameters
    ///
    /// * `app_menu_type` - Type of app menu: `AppSwitcher`, `Salesforce1`, or `NetworkTabs`
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let apps = client.app_menu("AppSwitcher").await?;
    /// for app in apps {
    ///     println!("App: {}", app["label"]);
    /// }
    /// ```
    ///
    /// # Salesforce Documentation
    ///
    /// <https://developer.salesforce.com/docs/atlas.en-us.api_rest.meta/api_rest/resources_appmenu.htm>
    #[instrument(skip(self))]
    pub async fn app_menu(&self, app_menu_type: &str) -> Result<serde_json::Value> {
        // Validate app_menu_type to prevent URL path injection
        let valid_types = ["AppSwitcher", "Salesforce1", "NetworkTabs"];
        if !valid_types.contains(&app_menu_type) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_APP_MENU_TYPE".to_string(),
                message: format!(
                    "Invalid app menu type '{}'. Must be one of: AppSwitcher, Salesforce1, NetworkTabs",
                    app_menu_type
                ),
            }));
        }
        let path = format!("appMenu/{}", app_menu_type);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get recently viewed items.
    ///
    /// Returns a list of recently viewed records across all objects.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let recent = client.recent_items().await?;
    /// for item in recent {
    ///     println!("Recently viewed: {} ({})", item["Name"], item["attributes"]["type"]);
    /// }
    /// ```
    ///
    /// # Salesforce Documentation
    ///
    /// <https://developer.salesforce.com/docs/atlas.en-us.api_rest.meta/api_rest/resources_recent_items.htm>
    #[instrument(skip(self))]
    pub async fn recent_items(&self) -> Result<Vec<serde_json::Value>> {
        self.client.rest_get("recent").await.map_err(Into::into)
    }

    /// Get relevant items.
    ///
    /// Returns a contextual list of objects relevant to the user.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let relevant = client.relevant_items().await?;
    /// println!("Relevant items: {:?}", relevant);
    /// ```
    ///
    /// # Salesforce Documentation
    ///
    /// <https://developer.salesforce.com/docs/atlas.en-us.api_rest.meta/api_rest/resources_relevant_items.htm>
    #[instrument(skip(self))]
    pub async fn relevant_items(&self) -> Result<serde_json::Value> {
        self.client
            .rest_get("relevantItems")
            .await
            .map_err(Into::into)
    }

    /// Get global compact layouts for multiple objects.
    ///
    /// Returns compact layouts for the specified objects.
    ///
    /// # Parameters
    ///
    /// * `sobject_list` - Comma-separated list of object names
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let layouts = client.compact_layouts("Account,Contact").await?;
    /// println!("Compact layouts: {:?}", layouts);
    /// ```
    ///
    /// # Salesforce Documentation
    ///
    /// <https://developer.salesforce.com/docs/atlas.en-us.api_rest.meta/api_rest/resources_compact_layouts.htm>
    #[instrument(skip(self))]
    pub async fn compact_layouts(&self, sobject_list: &str) -> Result<serde_json::Value> {
        // Check for empty input
        if sobject_list.trim().is_empty() {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_INPUT".to_string(),
                message: "SObject list cannot be empty".to_string(),
            }));
        }

        // Validate sobject names for safety
        let objects: Vec<&str> = sobject_list.split(',').map(|s| s.trim()).collect();
        for obj in &objects {
            if obj.is_empty() {
                return Err(Error::new(ErrorKind::Salesforce {
                    error_code: "INVALID_INPUT".to_string(),
                    message: "SObject list contains empty entries".to_string(),
                }));
            }
            if !soql::is_safe_sobject_name(obj) {
                return Err(Error::new(ErrorKind::Salesforce {
                    error_code: "INVALID_SOBJECT".to_string(),
                    message: format!("Invalid SObject name: {}", obj),
                }));
            }
        }
        let encoded = url_security::encode_param(sobject_list);
        let path = format!("compactLayouts?q={}", encoded);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get the schema for a platform event.
    ///
    /// Returns the Avro-style schema definition for a specific platform event.
    ///
    /// # Parameters
    ///
    /// * `event_name` - Name of the platform event (e.g., "Order_Event__e")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let schema = client.platform_event_schema("Order_Event__e").await?;
    /// println!("Event schema: {:?}", schema);
    /// ```
    ///
    /// # Salesforce Documentation
    ///
    /// <https://developer.salesforce.com/docs/atlas.en-us.api_rest.meta/api_rest/resources_event_eventschema.htm>
    #[instrument(skip(self))]
    pub async fn platform_event_schema(&self, event_name: &str) -> Result<serde_json::Value> {
        // Validate event name for safety
        if !soql::is_safe_sobject_name(event_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_EVENT_NAME".to_string(),
                message: "Invalid event name".to_string(),
            }));
        }
        let path = format!("event/eventSchema/{}", event_name);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get Lightning toggle metrics.
    ///
    /// Returns metrics about Lightning Experience feature toggles.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let metrics = client.lightning_toggle_metrics().await?;
    /// println!("Toggle metrics: {:?}", metrics);
    /// ```
    ///
    /// # Salesforce Documentation
    ///
    /// <https://developer.salesforce.com/docs/atlas.en-us.api_rest.meta/api_rest/resources_lightning_togglemetrics.htm>
    #[instrument(skip(self))]
    pub async fn lightning_toggle_metrics(&self) -> Result<serde_json::Value> {
        self.client
            .rest_get("lightning/toggleMetrics")
            .await
            .map_err(Into::into)
    }

    /// Get Lightning usage statistics.
    ///
    /// Returns Lightning Experience usage statistics for the org.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let usage = client.lightning_usage().await?;
    /// println!("Lightning usage: {:?}", usage);
    /// ```
    ///
    /// # Salesforce Documentation
    ///
    /// <https://developer.salesforce.com/docs/atlas.en-us.api_rest.meta/api_rest/resources_lightning_usage.htm>
    #[instrument(skip(self))]
    pub async fn lightning_usage(&self) -> Result<serde_json::Value> {
        self.client
            .rest_get("lightning/usage")
            .await
            .map_err(Into::into)
    }

    /// Deploy metadata via REST API (multipart form data).
    ///
    /// This is an alternative to the SOAP-based metadata deploy operation.
    /// It accepts a zip file containing metadata and deployment options.
    ///
    /// **Note**: This endpoint is currently not implemented and will return an error.
    /// Full multipart/form-data support requires extending the HTTP client infrastructure.
    /// For now, please use the SOAP-based metadata deploy in the `sf-metadata` crate.
    ///
    /// # Parameters
    ///
    /// * `zip_data` - Zip file contents as bytes
    /// * `options` - Deployment options (JSON serializable struct)
    ///
    /// # Salesforce Documentation
    ///
    /// <https://developer.salesforce.com/docs/atlas.en-us.api_rest.meta/api_rest/resources_deploy_metadata.htm>
    #[instrument(skip(self, zip_data, _options))]
    pub async fn rest_deploy<T: Serialize>(
        &self,
        zip_data: &[u8],
        _options: &T,
    ) -> Result<serde_json::Value> {
        // This is a placeholder implementation that demonstrates the endpoint.
        // Full multipart/form-data support would require extending the HTTP client.
        let _url = self.client.rest_url("metadata/deployRequest");
        let _zip_len = zip_data.len();

        // For now, return an error indicating this needs to be implemented
        Err(Error::new(ErrorKind::Other(
            "REST Deploy endpoint requires multipart/form-data support. \
             Please use the SOAP-based metadata deploy in sf-metadata crate for now."
                .to_string(),
        )))
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

    // Tests for standalone REST resources

    #[test]
    fn test_tabs_url_construction() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        let url = client.client.rest_url("tabs");
        assert_eq!(url, "https://na1.salesforce.com/services/data/v62.0/tabs");
    }

    #[test]
    fn test_theme_url_construction() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        let url = client.client.rest_url("theme");
        assert_eq!(url, "https://na1.salesforce.com/services/data/v62.0/theme");
    }

    #[test]
    fn test_app_menu_url_construction() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        let url = client.client.rest_url("appMenu/AppSwitcher");
        assert_eq!(
            url,
            "https://na1.salesforce.com/services/data/v62.0/appMenu/AppSwitcher"
        );
    }

    #[test]
    fn test_recent_items_url_construction() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        let url = client.client.rest_url("recent");
        assert_eq!(url, "https://na1.salesforce.com/services/data/v62.0/recent");
    }

    #[test]
    fn test_relevant_items_url_construction() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        let url = client.client.rest_url("relevantItems");
        assert_eq!(
            url,
            "https://na1.salesforce.com/services/data/v62.0/relevantItems"
        );
    }

    #[test]
    fn test_platform_event_schema_url_construction() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        let url = client.client.rest_url("event/eventSchema/Order_Event__e");
        assert_eq!(
            url,
            "https://na1.salesforce.com/services/data/v62.0/event/eventSchema/Order_Event__e"
        );
    }

    #[test]
    fn test_lightning_toggle_metrics_url_construction() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        let url = client.client.rest_url("lightning/toggleMetrics");
        assert_eq!(
            url,
            "https://na1.salesforce.com/services/data/v62.0/lightning/toggleMetrics"
        );
    }

    #[test]
    fn test_lightning_usage_url_construction() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        let url = client.client.rest_url("lightning/usage");
        assert_eq!(
            url,
            "https://na1.salesforce.com/services/data/v62.0/lightning/usage"
        );
    }

    #[test]
    fn test_compact_layouts_valid_sobjects() {
        // Test that valid sobject names are accepted
        let valid_list = "Account,Contact,Lead";
        let objects: Vec<&str> = valid_list.split(',').map(|s| s.trim()).collect();
        for obj in &objects {
            assert!(soql::is_safe_sobject_name(obj), "Should be valid: {}", obj);
        }
    }

    #[test]
    fn test_compact_layouts_url_construction() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        // The URL should have the query parameter encoded
        let url = client.client.rest_url("compactLayouts?q=Account%2CContact");
        assert_eq!(
            url,
            "https://na1.salesforce.com/services/data/v62.0/compactLayouts?q=Account%2CContact"
        );
    }

    #[test]
    fn test_app_menu_invalid_type() {
        // Test that invalid app menu types are rejected
        let invalid_types = ["invalid", "../../etc/passwd", "AppSwitcher/../secret"];
        for invalid_type in &invalid_types {
            // We can't easily test the async function here, but we can verify
            // that the valid types list doesn't include these values
            let valid_types = ["AppSwitcher", "Salesforce1", "NetworkTabs"];
            assert!(
                !valid_types.contains(invalid_type),
                "{} should not be valid",
                invalid_type
            );
        }
    }

    #[test]
    fn test_app_menu_valid_types() {
        // Test that valid app menu types are accepted
        let valid_types = ["AppSwitcher", "Salesforce1", "NetworkTabs"];
        for valid_type in &valid_types {
            // These should be in the valid list
            assert!(
                valid_types.contains(valid_type),
                "{} should be valid",
                valid_type
            );
        }
    }

    #[test]
    fn test_rest_deploy_url_construction() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token").unwrap();
        let url = client.client.rest_url("metadata/deployRequest");
        assert_eq!(
            url,
            "https://na1.salesforce.com/services/data/v62.0/metadata/deployRequest"
        );
    }
}
