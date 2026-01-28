//! Bulk API 2.0 client.
//!
//! Provides a high-level interface to Salesforce Bulk API 2.0 for
//! efficient large-scale data operations.

use std::time::Duration;
use tokio::time::sleep;
use tracing::instrument;

use busbar_sf_client::{ClientConfig, SalesforceClient};

use crate::error::{Error, ErrorKind, Result};
use crate::types::*;

/// Default polling interval for job status checks.
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Default maximum wait time for job completion.
const DEFAULT_MAX_WAIT: Duration = Duration::from_secs(3600); // 1 hour

/// Salesforce Bulk API 2.0 client.
///
/// Provides low-level API operations for Bulk API 2.0:
/// - Create, monitor, and manage ingest jobs
/// - Create, monitor, and manage query jobs
/// - Upload data and retrieve results
///
/// # Example
///
/// ```rust,ignore
/// use busbar_sf_bulk::{BulkApiClient, BulkOperation, CreateIngestJobRequest};
///
/// let client = BulkApiClient::new(
///     "https://myorg.my.salesforce.com",
///     "access_token_here",
/// )?;
///
/// // Create ingest job
/// let request = CreateIngestJobRequest::new("Account", BulkOperation::Insert);
/// let job = client.create_ingest_job(request).await?;
///
/// // Upload data
/// client.upload_job_data(&job.id, "Name\nTest Account 1\nTest Account 2").await?;
///
/// // Close and wait for completion
/// client.close_ingest_job(&job.id).await?;
/// let completed = client.wait_for_ingest_job(&job.id).await?;
/// ```
#[derive(Debug, Clone)]
pub struct BulkApiClient {
    client: SalesforceClient,
    poll_interval: Duration,
    max_wait: Duration,
}

impl BulkApiClient {
    /// Create a new Bulk API client.
    pub fn new(instance_url: impl Into<String>, access_token: impl Into<String>) -> Result<Self> {
        let client = SalesforceClient::new(instance_url, access_token)?;
        Ok(Self {
            client,
            poll_interval: DEFAULT_POLL_INTERVAL,
            max_wait: DEFAULT_MAX_WAIT,
        })
    }

    /// Create a new Bulk API client with custom HTTP configuration.
    pub fn with_config(
        instance_url: impl Into<String>,
        access_token: impl Into<String>,
        config: ClientConfig,
    ) -> Result<Self> {
        let client = SalesforceClient::with_config(instance_url, access_token, config)?;
        Ok(Self {
            client,
            poll_interval: DEFAULT_POLL_INTERVAL,
            max_wait: DEFAULT_MAX_WAIT,
        })
    }

    /// Create a Bulk API client from an existing SalesforceClient.
    pub fn from_client(client: SalesforceClient) -> Self {
        Self {
            client,
            poll_interval: DEFAULT_POLL_INTERVAL,
            max_wait: DEFAULT_MAX_WAIT,
        }
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

    /// Set the polling interval for job status checks.
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Set the maximum wait time for job completion.
    pub fn with_max_wait(mut self, max_wait: Duration) -> Self {
        self.max_wait = max_wait;
        self
    }

    // =========================================================================
    // Ingest Job Operations
    // =========================================================================

    /// Create a new ingest job.
    #[instrument(skip(self, request))]
    pub async fn create_ingest_job(&self, request: CreateIngestJobRequest) -> Result<IngestJob> {
        let url = self.client.bulk_url("ingest");
        let job: IngestJob = self.client.post_json(&url, &request).await?;
        Ok(job)
    }

    /// Upload CSV data to an ingest job.
    #[instrument(skip(self, csv_data))]
    pub async fn upload_job_data(&self, job_id: &str, csv_data: &str) -> Result<()> {
        let url = format!("{}/{}/batches", self.client.bulk_url("ingest"), job_id);

        let request = self.client.put(&url).csv(csv_data);

        let response = self.client.execute(request).await?;

        if !response.is_success() {
            return Err(Error::new(ErrorKind::Upload(format!(
                "Failed to upload job data: status {}",
                response.status()
            ))));
        }

        Ok(())
    }

    /// Close an ingest job (mark as UploadComplete).
    #[instrument(skip(self))]
    pub async fn close_ingest_job(&self, job_id: &str) -> Result<IngestJob> {
        let url = format!("{}/{}", self.client.bulk_url("ingest"), job_id);
        let request = UpdateJobStateRequest::upload_complete();

        let req = self.client.patch(&url).json(&request)?;
        let response = self.client.execute(req).await?;

        if !response.is_success() {
            return Err(Error::new(ErrorKind::Api(format!(
                "Failed to close job: status {}",
                response.status()
            ))));
        }

        let job: IngestJob = response.json().await?;
        Ok(job)
    }

    /// Abort an ingest job.
    #[instrument(skip(self))]
    pub async fn abort_ingest_job(&self, job_id: &str) -> Result<IngestJob> {
        let url = format!("{}/{}", self.client.bulk_url("ingest"), job_id);
        let request = UpdateJobStateRequest::abort();

        let req = self.client.patch(&url).json(&request)?;
        let response = self.client.execute(req).await?;

        if !response.is_success() {
            return Err(Error::new(ErrorKind::Api(format!(
                "Failed to abort job: status {}",
                response.status()
            ))));
        }

        let job: IngestJob = response.json().await?;
        Ok(job)
    }

    /// Get ingest job status.
    #[instrument(skip(self))]
    pub async fn get_ingest_job(&self, job_id: &str) -> Result<IngestJob> {
        let url = format!("{}/{}", self.client.bulk_url("ingest"), job_id);
        let job: IngestJob = self.client.get_json(&url).await?;
        Ok(job)
    }

    /// Wait for an ingest job to complete.
    #[instrument(skip(self))]
    pub async fn wait_for_ingest_job(&self, job_id: &str) -> Result<IngestJob> {
        let start = std::time::Instant::now();

        loop {
            let job = self.get_ingest_job(job_id).await?;

            if job.state.is_terminal() {
                return Ok(job);
            }

            if start.elapsed() > self.max_wait {
                return Err(Error::new(ErrorKind::Timeout(format!(
                    "Job {} did not complete within {:?}",
                    job_id, self.max_wait
                ))));
            }

            sleep(self.poll_interval).await;
        }
    }

    /// Get successful results from an ingest job (CSV format).
    #[instrument(skip(self))]
    pub async fn get_successful_results(&self, job_id: &str) -> Result<String> {
        let url = format!(
            "{}/{}/successfulResults",
            self.client.bulk_url("ingest"),
            job_id
        );

        let request = self.client.get(&url).header("Accept", "text/csv");

        let response = self.client.execute(request).await?;

        if !response.is_success() {
            return Err(Error::new(ErrorKind::Api(format!(
                "Failed to get successful results: status {}",
                response.status()
            ))));
        }

        response.text().await.map_err(Into::into)
    }

    /// Get failed results from an ingest job (CSV format).
    #[instrument(skip(self))]
    pub async fn get_failed_results(&self, job_id: &str) -> Result<String> {
        let url = format!(
            "{}/{}/failedResults",
            self.client.bulk_url("ingest"),
            job_id
        );

        let request = self.client.get(&url).header("Accept", "text/csv");

        let response = self.client.execute(request).await?;

        if !response.is_success() {
            return Err(Error::new(ErrorKind::Api(format!(
                "Failed to get failed results: status {}",
                response.status()
            ))));
        }

        response.text().await.map_err(Into::into)
    }

    /// Get unprocessed records from an ingest job (CSV format).
    #[instrument(skip(self))]
    pub async fn get_unprocessed_records(&self, job_id: &str) -> Result<String> {
        let url = format!(
            "{}/{}/unprocessedrecords",
            self.client.bulk_url("ingest"),
            job_id
        );

        let request = self.client.get(&url).header("Accept", "text/csv");

        let response = self.client.execute(request).await?;

        if !response.is_success() {
            return Err(Error::new(ErrorKind::Api(format!(
                "Failed to get unprocessed records: status {}",
                response.status()
            ))));
        }

        response.text().await.map_err(Into::into)
    }

    /// Delete an ingest job.
    #[instrument(skip(self))]
    pub async fn delete_ingest_job(&self, job_id: &str) -> Result<()> {
        let url = format!("{}/{}", self.client.bulk_url("ingest"), job_id);

        let request = self.client.delete(&url);
        let response = self.client.execute(request).await?;

        if !response.is_success() {
            return Err(Error::new(ErrorKind::Api(format!(
                "Failed to delete job: status {}",
                response.status()
            ))));
        }

        Ok(())
    }

    /// Get all ingest jobs.
    ///
    /// Returns a list of all ingest jobs in the org.
    #[instrument(skip(self))]
    pub async fn get_all_ingest_jobs(&self) -> Result<IngestJobList> {
        let url = self.client.bulk_url("ingest");
        let jobs: IngestJobList = self.client.get_json(&url).await?;
        Ok(jobs)
    }

    // =========================================================================
    // Query Job Operations
    // =========================================================================

    /// Create a new query job.
    #[instrument(skip(self, request))]
    pub async fn create_query_job(&self, request: CreateQueryJobRequest) -> Result<QueryJob> {
        let url = self.client.bulk_url("query");
        let job: QueryJob = self.client.post_json(&url, &request).await?;
        Ok(job)
    }

    /// Get query job status.
    #[instrument(skip(self))]
    pub async fn get_query_job(&self, job_id: &str) -> Result<QueryJob> {
        let url = format!("{}/{}", self.client.bulk_url("query"), job_id);
        let job: QueryJob = self.client.get_json(&url).await?;
        Ok(job)
    }

    /// Wait for a query job to complete.
    #[instrument(skip(self))]
    pub async fn wait_for_query_job(&self, job_id: &str) -> Result<QueryJob> {
        let start = std::time::Instant::now();

        loop {
            let job = self.get_query_job(job_id).await?;

            if job.state.is_terminal() {
                return Ok(job);
            }

            if start.elapsed() > self.max_wait {
                return Err(Error::new(ErrorKind::Timeout(format!(
                    "Query job {} did not complete within {:?}",
                    job_id, self.max_wait
                ))));
            }

            sleep(self.poll_interval).await;
        }
    }

    /// Abort a query job.
    #[instrument(skip(self))]
    pub async fn abort_query_job(&self, job_id: &str) -> Result<QueryJob> {
        let url = format!("{}/{}", self.client.bulk_url("query"), job_id);
        let request = UpdateJobStateRequest::abort();

        let req = self.client.patch(&url).json(&request)?;
        let response = self.client.execute(req).await?;

        if !response.is_success() {
            return Err(Error::new(ErrorKind::Api(format!(
                "Failed to abort query job: status {}",
                response.status()
            ))));
        }

        let job: QueryJob = response.json().await?;
        Ok(job)
    }

    /// Get query results with pagination.
    ///
    /// Returns results in CSV format. Use `locator` for pagination.
    #[instrument(skip(self))]
    pub async fn get_query_results(
        &self,
        job_id: &str,
        locator: Option<&str>,
        max_records: Option<usize>,
    ) -> Result<QueryResults> {
        let mut url = format!("{}/{}/results", self.client.bulk_url("query"), job_id);

        let mut query_params = vec![];
        if let Some(loc) = locator {
            query_params.push(format!("locator={}", urlencoding::encode(loc)));
        }
        if let Some(max) = max_records {
            query_params.push(format!("maxRecords={}", max));
        }
        if !query_params.is_empty() {
            url = format!("{}?{}", url, query_params.join("&"));
        }

        let request = self.client.get(&url).header("Accept", "text/csv");

        let response = self.client.execute(request).await?;

        if !response.is_success() {
            return Err(Error::new(ErrorKind::Api(format!(
                "Failed to get query results: status {}",
                response.status()
            ))));
        }

        // Get the Sforce-Locator header for pagination
        let next_locator = response
            .sforce_locator()
            .map(|s| s.to_string())
            .filter(|s| s != "null");

        let csv_data = response.text().await?;

        Ok(QueryResults {
            csv_data,
            locator: next_locator,
        })
    }

    /// Get all query results (handles pagination automatically).
    #[instrument(skip(self))]
    pub async fn get_all_query_results(&self, job_id: &str) -> Result<String> {
        let mut all_results = String::new();
        let mut locator: Option<String> = None;
        let mut first_batch = true;

        loop {
            let results = self
                .get_query_results(job_id, locator.as_deref(), None)
                .await?;

            if first_batch {
                all_results = results.csv_data;
                first_batch = false;
            } else {
                // Skip header row for subsequent batches
                let data_without_header = results
                    .csv_data
                    .lines()
                    .skip(1)
                    .collect::<Vec<_>>()
                    .join("\n");
                if !data_without_header.is_empty() {
                    all_results.push('\n');
                    all_results.push_str(&data_without_header);
                }
            }

            match results.locator {
                Some(loc) => locator = Some(loc),
                None => break,
            }
        }

        Ok(all_results)
    }

    /// Delete a query job.
    #[instrument(skip(self))]
    pub async fn delete_query_job(&self, job_id: &str) -> Result<()> {
        let url = format!("{}/{}", self.client.bulk_url("query"), job_id);

        let request = self.client.delete(&url);
        let response = self.client.execute(request).await?;

        if !response.is_success() {
            return Err(Error::new(ErrorKind::Api(format!(
                "Failed to delete query job: status {}",
                response.status()
            ))));
        }

        Ok(())
    }

    /// Get all query jobs.
    ///
    /// Returns a list of all query jobs in the org.
    #[instrument(skip(self))]
    pub async fn get_all_query_jobs(&self) -> Result<QueryJobList> {
        let url = self.client.bulk_url("query");
        let jobs: QueryJobList = self.client.get_json(&url).await?;
        Ok(jobs)
    }

    // =========================================================================
    // High-Level Operations
    // =========================================================================

    /// Execute a complete ingest operation.
    ///
    /// Creates job, uploads data, waits for completion, and returns results.
    #[instrument(skip(self, csv_data))]
    pub async fn execute_ingest(
        &self,
        sobject: &str,
        operation: BulkOperation,
        csv_data: &str,
        external_id_field: Option<&str>,
    ) -> Result<IngestJobResult> {
        // Create job
        let mut request = CreateIngestJobRequest::new(sobject, operation);
        if let Some(ext_id) = external_id_field {
            request = request.with_external_id_field(ext_id);
        }

        let job = self.create_ingest_job(request).await?;

        // Upload data
        self.upload_job_data(&job.id, csv_data).await?;

        // Close job
        self.close_ingest_job(&job.id).await?;

        // Wait for completion
        let completed_job = self.wait_for_ingest_job(&job.id).await?;

        // Get results
        let successful_results = self.get_successful_results(&job.id).await.ok();
        let failed_results = self.get_failed_results(&job.id).await.ok();

        Ok(IngestJobResult {
            job: completed_job,
            successful_results,
            failed_results,
        })
    }

    /// Execute a complete query operation.
    ///
    /// Creates job, waits for completion, and returns all results.
    #[instrument(skip(self))]
    pub async fn execute_query(&self, soql: &str) -> Result<QueryJobResult> {
        // Create job
        let request = CreateQueryJobRequest::new(soql);
        let job = self.create_query_job(request).await?;

        // Wait for completion
        let completed_job = self.wait_for_query_job(&job.id).await?;

        // Get all results
        let results = if completed_job.state.is_success() {
            Some(self.get_all_query_results(&job.id).await?)
        } else {
            None
        };

        Ok(QueryJobResult {
            job: completed_job,
            results,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = BulkApiClient::new("https://test.salesforce.com", "token123").unwrap();

        assert_eq!(client.instance_url(), "https://test.salesforce.com");
        assert_eq!(client.api_version(), "62.0");
    }

    #[test]
    fn test_poll_interval() {
        let client = BulkApiClient::new("https://test.salesforce.com", "token123")
            .unwrap()
            .with_poll_interval(Duration::from_secs(10));

        assert_eq!(client.poll_interval, Duration::from_secs(10));
    }

    #[test]
    fn test_max_wait() {
        let client = BulkApiClient::new("https://test.salesforce.com", "token123")
            .unwrap()
            .with_max_wait(Duration::from_secs(120));

        assert_eq!(client.max_wait, Duration::from_secs(120));
    }
}
