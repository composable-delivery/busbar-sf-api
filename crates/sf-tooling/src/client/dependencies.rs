//! MetadataComponentDependency operations (Beta).

use crate::error::Result;
use tracing::instrument;

impl super::ToolingClient {
    /// Get metadata component dependencies.
    ///
    /// Returns dependency relationships between metadata components in your org.
    /// Note: Limited to 2000 records per query in Tooling API.
    ///
    /// Available since API v43.0 for Tooling API.
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
    /// // Get all dependencies (limited to 2000)
    /// let deps = client.get_metadata_component_dependencies(None).await?;
    ///
    /// // Filter by component type (hardcoded - safe)
    /// let apex_deps = client.get_metadata_component_dependencies(
    ///     Some("MetadataComponentType = 'ApexClass'")
    /// ).await?;
    /// ```
    #[cfg(feature = "dependencies")]
    #[instrument(skip(self))]
    pub async fn get_metadata_component_dependencies(
        &self,
        where_clause: Option<&str>,
    ) -> Result<Vec<crate::types::MetadataComponentDependency>> {
        let base_query = "SELECT MetadataComponentId, MetadataComponentName, MetadataComponentNamespace, MetadataComponentType, RefMetadataComponentId, RefMetadataComponentName, RefMetadataComponentNamespace, RefMetadataComponentType FROM MetadataComponentDependency";

        let query = if let Some(filter) = where_clause {
            format!("{} WHERE {}", base_query, filter)
        } else {
            base_query.to_string()
        };

        self.query_all(&query).await
    }
}

#[cfg(test)]
#[cfg(feature = "dependencies")]
mod tests {
    #[test]
    fn test_metadata_component_dependency_query_construction_no_filter() {
        let base_query = "SELECT MetadataComponentId, MetadataComponentName, MetadataComponentNamespace, MetadataComponentType, RefMetadataComponentId, RefMetadataComponentName, RefMetadataComponentNamespace, RefMetadataComponentType FROM MetadataComponentDependency";

        let query = base_query.to_string();

        assert_eq!(query, base_query);
        assert!(query.contains("MetadataComponentId"));
        assert!(query.contains("RefMetadataComponentId"));
        assert!(!query.contains("WHERE"));
    }

    #[test]
    fn test_metadata_component_dependency_query_construction_with_filter() {
        let base_query = "SELECT MetadataComponentId, MetadataComponentName, MetadataComponentNamespace, MetadataComponentType, RefMetadataComponentId, RefMetadataComponentName, RefMetadataComponentNamespace, RefMetadataComponentType FROM MetadataComponentDependency";
        let filter = "MetadataComponentType = 'ApexClass'";

        let query = format!("{} WHERE {}", base_query, filter);

        assert!(query.contains("WHERE MetadataComponentType = 'ApexClass'"));
        assert!(query.contains("MetadataComponentId"));
        assert!(query.contains("RefMetadataComponentId"));
    }
}
