use serde::de::DeserializeOwned;
use tracing::instrument;

use busbar_sf_client::QueryResult;

use crate::error::Result;

impl super::ToolingClient {
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
}
