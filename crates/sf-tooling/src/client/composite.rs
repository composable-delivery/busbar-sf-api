use busbar_sf_client::security::soql;
use tracing::instrument;

use crate::error::{Error, ErrorKind, Result};

impl super::ToolingClient {
    /// Execute a Tooling API composite request with multiple subrequests.
    ///
    /// The Tooling API composite endpoint allows up to 25 subrequests in a single API call.
    /// Subrequests can reference results from earlier subrequests using `@{referenceId}`.
    ///
    /// Available since API v40.0.
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
}

#[cfg(test)]
mod tests {
    use super::super::ToolingClient;

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
}
