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
                "Failed to delete ingest job: status {}",
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
    // Query Job Operations - SECURED
    // =========================================================================
    // All query operations now use QueryBuilder for automatic SOQL injection prevention.
    // No raw SOQL methods are exposed in the public API.

    /// Execute a complete query operation with automatic SOQL injection prevention.
    ///
    /// This is the ONLY way to execute bulk queries. It uses QueryBuilder for automatic
    /// escaping of user input, making it impossible to introduce SOQL injection vulnerabilities.
    ///
    /// Creates job, waits for completion, and returns all results.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use busbar_sf_bulk::{BulkApiClient, QueryBuilder};
    ///
    /// let client = BulkApiClient::new(instance_url, access_token)?;
    ///
    /// // User input is automatically escaped - safe by default!
    /// let user_input = "O'Brien's Company";
    /// let result = client.execute_query(
    ///     QueryBuilder::new("Account")?
    ///         .select(&["Id", "Name", "Industry"])
    ///         .where_eq("Name", user_input)?  // Automatically escaped!
    ///         .limit(10000)
    /// ).await?;
    ///
    /// println!("Retrieved {} records", result.job.number_records_processed);
    /// ```
    ///
    /// # Security
    ///
    /// QueryBuilder automatically escapes all user input to prevent SOQL injection attacks.
    /// There is no way to bypass this security - it's built into the API design.
    #[cfg(feature = "query-builder")]
    #[instrument(skip(self, query_builder))]
    pub async fn execute_query<T>(
        &self,
        query_builder: busbar_sf_rest::QueryBuilder<T>,
    ) -> Result<QueryJobResult>
    where
        T: serde::de::DeserializeOwned + Clone,
    {
        // Build the safe SOQL query
        let soql = query_builder
            .build()
            .map_err(|e| Error::new(ErrorKind::Api(format!("Failed to build query: {}", e))))?;

        // Create job
        let request = CreateQueryJobRequest::new(soql);
        let url = self.client.bulk_url("query");
        let job: QueryJob = self.client.post_json(&url, &request).await?;

        // Wait for completion
        let completed_job = self.wait_for_query_job_internal(&job.id).await?;

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

    /// Abort a query job.
    ///
    /// This can be used with job IDs from `execute_query()`.
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

    /// Wait for a query job to complete (internal implementation).
    #[instrument(skip(self))]
    async fn wait_for_query_job_internal(&self, job_id: &str) -> Result<QueryJob> {
        let start = std::time::Instant::now();

        loop {
            let url = format!("{}/{}", self.client.bulk_url("query"), job_id);
            let job: QueryJob = self.client.get_json(&url).await?;

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

    /// Get a batch of parallel result URLs for concurrent download (GA since API v62.0+).
    ///
    /// Returns up to 5 result URLs per call that can be downloaded concurrently,
    /// dramatically reducing download time for large datasets.
    ///
    /// # Arguments
    ///
    /// * `job_id` - The query job ID
    /// * `max_records` - Optional maximum number of result URLs to return (up to 5)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use busbar_sf_bulk::BulkApiClient;
    ///
    /// let client = BulkApiClient::new(instance_url, access_token)?;
    ///
    /// // Get first batch of result URLs
    /// let batch = client.get_parallel_query_results(job_id, None).await?;
    ///
    /// // For simple use cases, prefer the high-level method:
    /// let csv_data = client.get_all_query_results_parallel(job_id).await?;
    /// ```
    ///
    /// # API Version
    ///
    /// This endpoint requires API version 62.0 or higher (Winter '25+).
    #[instrument(skip(self))]
    pub async fn get_parallel_query_results(
        &self,
        job_id: &str,
        max_records: Option<u32>,
    ) -> Result<ParallelResultsBatch> {
        let mut url = format!(
            "{}/{}/parallelResults",
            self.client.bulk_url("query"),
            job_id
        );

        if let Some(max) = max_records {
            url = format!("{}?maxRecords={}", url, max);
        }

        let batch: ParallelResultsBatch = self.client.get_json(&url).await?;
        Ok(batch)
    }

    /// Normalize a URL that may be relative or absolute.
    fn normalize_url(&self, url: &str) -> String {
        if url.starts_with('/') {
            format!("{}{}", self.client.instance_url(), url)
        } else if url.starts_with("http") {
            url.to_string()
        } else {
            format!("{}/{}", self.client.instance_url(), url)
        }
    }

    /// Get all query results using parallel download (high-level convenience).
    ///
    /// This method fetches all result pages concurrently using the parallel results
    /// endpoint (GA since API v62.0+), which can dramatically reduce download time
    /// for large datasets compared to serial pagination.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use busbar_sf_bulk::{BulkApiClient, QueryBuilder};
    ///
    /// let client = BulkApiClient::new(instance_url, access_token)?;
    ///
    /// // Execute query
    /// let result = client.execute_query(
    ///     QueryBuilder::new("Account")?
    ///         .select(&["Id", "Name"])
    ///         .limit(100000)
    /// ).await?;
    ///
    /// // Get all results in parallel
    /// let csv_data = client.get_all_query_results_parallel(&result.job.id).await?;
    /// ```
    ///
    /// # API Version
    ///
    /// This method requires API version 62.0 or higher (Winter '25+).
    #[instrument(skip(self))]
    pub async fn get_all_query_results_parallel(&self, job_id: &str) -> Result<String> {
        use futures::future::join_all;

        let mut all_results = String::new();
        let mut first_batch = true;

        // Fetch all batches of result URLs
        let mut next_batch_url: Option<String> = None;
        loop {
            // Get batch of result URLs
            let batch = if let Some(url) = next_batch_url.take() {
                let normalized_url = self.normalize_url(&url);
                self.client.get_json(&normalized_url).await?
            } else {
                self.get_parallel_query_results(job_id, None).await?
            };

            // Download all URLs in this batch concurrently
            let download_tasks: Vec<_> = batch
                .result_url
                .into_iter()
                .map(|url| {
                    let client = &self.client;
                    let full_url = self.normalize_url(&url);
                    async move {
                        let request = client.get(&full_url).header("Accept", "text/csv");
                        let response = client.execute(request).await?;

                        if !response.is_success() {
                            return Err(Error::new(ErrorKind::Api(format!(
                                "Failed to get parallel result: status {}",
                                response.status()
                            ))));
                        }

                        response.text().await.map_err(Into::into)
                    }
                })
                .collect();

            let results: Vec<Result<String>> = join_all(download_tasks).await;

            // Combine results
            for (i, result) in results.into_iter().enumerate() {
                let csv_data = result?;

                if first_batch && i == 0 {
                    // First chunk includes header
                    all_results = csv_data;
                    first_batch = false;
                } else if let Some(newline_pos) = csv_data.find('\n') {
                    // Skip header row for subsequent chunks (more efficient than lines().skip(1))
                    if !all_results.is_empty() {
                        all_results.push('\n');
                    }
                    all_results.push_str(&csv_data[newline_pos + 1..]);
                }
            }

            // Check if there are more batches
            if let Some(next_url) = batch.next_records_url {
                next_batch_url = Some(next_url);
            } else {
                break;
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

    #[test]
    fn test_normalize_url_relative() {
        let client = BulkApiClient::new("https://na1.salesforce.com", "token").unwrap();
        assert_eq!(
            client.normalize_url("/services/data/v62.0/jobs/query/750xx/results/1"),
            "https://na1.salesforce.com/services/data/v62.0/jobs/query/750xx/results/1"
        );
    }

    #[test]
    fn test_normalize_url_absolute() {
        let client = BulkApiClient::new("https://na1.salesforce.com", "token").unwrap();
        assert_eq!(
            client.normalize_url("https://na1.salesforce.com/services/data/v62.0/jobs/query/750xx/results/1"),
            "https://na1.salesforce.com/services/data/v62.0/jobs/query/750xx/results/1"
        );
    }

    #[test]
    fn test_normalize_url_bare_path() {
        let client = BulkApiClient::new("https://na1.salesforce.com", "token").unwrap();
        assert_eq!(
            client.normalize_url("services/data/v62.0/jobs/query/750xx/results/1"),
            "https://na1.salesforce.com/services/data/v62.0/jobs/query/750xx/results/1"
        );
    }

    #[tokio::test]
    async fn test_get_parallel_query_results_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "resultUrl": [
                "/services/data/v62.0/jobs/query/750xx000000001/results/1",
                "/services/data/v62.0/jobs/query/750xx000000001/results/2"
            ],
            "nextRecordsUrl": null
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/jobs/query/750xx000000001/parallelResults"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = BulkApiClient::new(mock_server.uri(), "test-token").unwrap();

        let batch = client
            .get_parallel_query_results("750xx000000001", None)
            .await
            .expect("get_parallel_query_results should succeed");

        assert_eq!(batch.result_url.len(), 2);
        assert!(batch.result_url[0].contains("results/1"));
        assert!(batch.result_url[1].contains("results/2"));
        assert!(batch.next_records_url.is_none());
    }

    #[tokio::test]
    async fn test_get_parallel_query_results_with_max_records() {
        use wiremock::matchers::{method, path_regex, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "resultUrl": [
                "/services/data/v62.0/jobs/query/750xx000000002/results/1"
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/jobs/query/750xx000000002/parallelResults"))
            .and(query_param("maxRecords", "3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = BulkApiClient::new(mock_server.uri(), "test-token").unwrap();

        let batch = client
            .get_parallel_query_results("750xx000000002", Some(3))
            .await
            .expect("get_parallel_query_results with maxRecords should succeed");

        assert_eq!(batch.result_url.len(), 1);
    }

    #[tokio::test]
    async fn test_get_all_query_results_parallel_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Mock parallel results endpoint
        let response_body = serde_json::json!({
            "resultUrl": [
                format!("{}/services/data/v62.0/jobs/query/750xx000000003/results/1", mock_server.uri()),
                format!("{}/services/data/v62.0/jobs/query/750xx000000003/results/2", mock_server.uri())
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/jobs/query/750xx000000003/parallelResults"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        // Mock result URL 1
        Mock::given(method("GET"))
            .and(path_regex(".*/results/1$"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("Id,Name\n001xx1,Account One\n001xx2,Account Two"),
            )
            .mount(&mock_server)
            .await;

        // Mock result URL 2
        Mock::given(method("GET"))
            .and(path_regex(".*/results/2$"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("Id,Name\n001xx3,Account Three\n001xx4,Account Four"),
            )
            .mount(&mock_server)
            .await;

        let client = BulkApiClient::new(mock_server.uri(), "test-token").unwrap();

        let csv = client
            .get_all_query_results_parallel("750xx000000003")
            .await
            .expect("get_all_query_results_parallel should succeed");

        // First chunk has header, subsequent chunks have header stripped
        assert!(csv.contains("Id,Name"));
        assert!(csv.contains("Account One"));
        assert!(csv.contains("Account Two"));
        assert!(csv.contains("Account Three"));
        assert!(csv.contains("Account Four"));
    }
}
