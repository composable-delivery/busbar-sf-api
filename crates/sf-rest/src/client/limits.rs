use tracing::instrument;

use crate::error::Result;

impl super::SalesforceRestClient {
    /// Get API limits for the org.
    #[instrument(skip(self))]
    pub async fn limits(&self) -> Result<serde_json::Value> {
        self.client.rest_get("limits").await.map_err(Into::into)
    }

    /// Get available API versions.
    #[instrument(skip(self))]
    pub async fn versions(&self) -> Result<Vec<super::ApiVersion>> {
        let url = format!("{}/services/data", self.client.instance_url());
        self.client.get_json(&url).await.map_err(Into::into)
    }
}
