use tracing::instrument;

use crate::error::Result;
use crate::types::{
    RunTestsAsyncRequest, RunTestsRequest, RunTestsResponse, RunTestsSyncRequest,
    RunTestsSyncResult, TestDiscoveryResult,
};

impl super::ToolingClient {
    /// Run tests asynchronously.
    ///
    /// Returns the AsyncApexJob ID as a plain string.
    #[instrument(skip(self, request))]
    pub async fn run_tests_async(&self, request: &RunTestsAsyncRequest) -> Result<String> {
        let url = format!(
            "{}/services/data/v{}/tooling/runTestsAsynchronous/",
            self.client.instance_url(),
            self.client.api_version()
        );

        // Salesforce returns the job ID as a plain quoted JSON string, not an object
        let job_id: String = self.client.post_json(&url, request).await?;
        Ok(job_id)
    }

    /// Run tests synchronously.
    ///
    /// Blocks until completion. Returns full results including successes and failures.
    #[instrument(skip(self, request))]
    pub async fn run_tests_sync(
        &self,
        request: &RunTestsSyncRequest,
    ) -> Result<RunTestsSyncResult> {
        let url = format!(
            "{}/services/data/v{}/tooling/runTestsSynchronous/",
            self.client.instance_url(),
            self.client.api_version()
        );

        self.client
            .post_json(&url, request)
            .await
            .map_err(Into::into)
    }

    /// Discover available tests (Apex and Flow tests).
    ///
    /// **Requires API v65.0 or later.**
    #[instrument(skip(self))]
    pub async fn discover_tests(&self, category: Option<&str>) -> Result<TestDiscoveryResult> {
        let mut url = format!(
            "{}/services/data/v{}/tooling/tests/",
            self.client.instance_url(),
            self.client.api_version()
        );

        if let Some(cat) = category {
            url = format!("{}?category={}", url, cat);
        }

        self.client.get_json(&url).await.map_err(Into::into)
    }

    /// Run tests using the unified Test Runner API (v65.0+).
    #[instrument(skip(self, request))]
    pub async fn run_tests(&self, request: &RunTestsRequest) -> Result<String> {
        let url = format!(
            "{}/services/data/v{}/tooling/tests/",
            self.client.instance_url(),
            self.client.api_version()
        );

        let response: RunTestsResponse = self.client.post_json(&url, request).await?;
        Ok(response.test_run_id)
    }
}

#[cfg(test)]
mod tests {
    use super::super::ToolingClient;

    #[test]
    fn test_run_tests_async_url() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token")
            .unwrap()
            .with_api_version("65.0");

        let expected =
            "https://na1.salesforce.com/services/data/v65.0/tooling/runTestsAsynchronous/";
        let actual = format!(
            "{}/services/data/v{}/tooling/runTestsAsynchronous/",
            client.instance_url(),
            client.api_version()
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_run_tests_sync_url() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token")
            .unwrap()
            .with_api_version("62.0");

        let expected =
            "https://na1.salesforce.com/services/data/v62.0/tooling/runTestsSynchronous/";
        let actual = format!(
            "{}/services/data/v{}/tooling/runTestsSynchronous/",
            client.instance_url(),
            client.api_version()
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_discover_tests_url() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token")
            .unwrap()
            .with_api_version("65.0");

        let url_no_cat = format!(
            "{}/services/data/v{}/tooling/tests/",
            client.instance_url(),
            client.api_version()
        );
        assert_eq!(
            url_no_cat,
            "https://na1.salesforce.com/services/data/v65.0/tooling/tests/"
        );
    }

    #[test]
    fn test_run_tests_url() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token")
            .unwrap()
            .with_api_version("65.0");

        let expected = "https://na1.salesforce.com/services/data/v65.0/tooling/tests/";
        let actual = format!(
            "{}/services/data/v{}/tooling/tests/",
            client.instance_url(),
            client.api_version()
        );
        assert_eq!(actual, expected);
    }
}
