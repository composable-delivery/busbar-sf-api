//! Metadata API client.

use base64::{engine::general_purpose, Engine as _};
use busbar_sf_auth::{Credentials, SalesforceCredentials};
use busbar_sf_client::security::xml;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::time::Duration;
use tokio::time::sleep;

use crate::deploy::{ComponentFailure, DeployOptions, DeployResult, DeployStatus};
use crate::describe::{DescribeMetadataResult, MetadataType};
use crate::error::{Error, ErrorKind, Result};
use crate::list::MetadataComponent;
use crate::retrieve::{PackageManifest, RetrieveMessage, RetrieveResult, RetrieveStatus};
use crate::types::{
    ComponentSuccess, FileProperties, SoapFault, TestFailure, TestLevel, DEFAULT_API_VERSION,
};

/// SOAP Action header name.
static SOAP_ACTION_HEADER: HeaderName = HeaderName::from_static("soapaction");

/// Salesforce Metadata API client.
#[derive(Debug)]
pub struct MetadataClient {
    instance_url: String,
    access_token: String,
    api_version: String,
    http_client: reqwest::Client,
}

impl MetadataClient {
    /// Create a new Metadata API client from credentials.
    pub fn new(credentials: &SalesforceCredentials) -> Result<Self> {
        Ok(Self {
            instance_url: credentials.instance_url().to_string(),
            access_token: credentials.access_token().to_string(),
            api_version: DEFAULT_API_VERSION.to_string(),
            http_client: reqwest::Client::new(),
        })
    }

    /// Create a new Metadata API client from instance URL and access token.
    pub fn from_parts(instance_url: impl Into<String>, access_token: impl Into<String>) -> Self {
        Self {
            instance_url: instance_url.into(),
            access_token: access_token.into(),
            api_version: DEFAULT_API_VERSION.to_string(),
            http_client: reqwest::Client::new(),
        }
    }

    /// Set the API version.
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }

    /// Set a custom HTTP client.
    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = client;
        self
    }

    /// Get the Metadata API SOAP endpoint URL.
    fn metadata_url(&self) -> String {
        format!("{}/services/Soap/m/{}", self.instance_url, self.api_version)
    }

    /// Build common headers for SOAP requests.
    fn build_headers(&self, soap_action: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("text/xml;charset=UTF-8"),
        );
        headers.insert(
            SOAP_ACTION_HEADER.clone(),
            HeaderValue::from_str(soap_action).unwrap(),
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.access_token)).unwrap(),
        );
        headers
    }

    // ========================================================================
    // Deploy Operations
    // ========================================================================

    /// Deploy a metadata package.
    ///
    /// The `package_zip` must be a properly structured zip file with metadata
    /// in the correct directory structure (e.g., `classes/MyClass.cls`).
    ///
    /// Returns the async process ID for tracking the deployment.
    pub async fn deploy(&self, package_zip: &[u8], options: DeployOptions) -> Result<String> {
        let encoded_zip = general_purpose::STANDARD.encode(package_zip);

        let test_level_xml = options
            .test_level
            .map(|tl| format!("<testLevel>{}</testLevel>", tl))
            .unwrap_or_default();

        let run_tests_xml = if options.test_level == Some(TestLevel::RunSpecifiedTests) {
            options
                .run_tests
                .iter()
                .map(|t| format!("<runTests>{}</runTests>", xml::escape(t)))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            String::new()
        };

        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <soap:Header>
    <SessionHeader xmlns="http://soap.sforce.com/2006/04/metadata">
      <sessionId>{session_id}</sessionId>
    </SessionHeader>
  </soap:Header>
  <soap:Body>
    <deploy xmlns="http://soap.sforce.com/2006/04/metadata">
      <ZipFile>{zip_file}</ZipFile>
      <DeployOptions>
        <allowMissingFiles>{allow_missing}</allowMissingFiles>
        <autoUpdatePackage>{auto_update}</autoUpdatePackage>
        <checkOnly>{check_only}</checkOnly>
        <ignoreWarnings>{ignore_warnings}</ignoreWarnings>
        <performRetrieve>{perform_retrieve}</performRetrieve>
        <purgeOnDelete>{purge_on_delete}</purgeOnDelete>
        <rollbackOnError>{rollback_on_error}</rollbackOnError>
        <runAllTests>{run_all_tests}</runAllTests>
        <singlePackage>{single_package}</singlePackage>
        {test_level}
        {run_tests}
      </DeployOptions>
    </deploy>
  </soap:Body>
</soap:Envelope>"#,
            session_id = self.access_token,
            zip_file = encoded_zip,
            allow_missing = options.allow_missing_files,
            auto_update = options.auto_update_package,
            check_only = options.check_only,
            ignore_warnings = options.ignore_warnings,
            perform_retrieve = options.perform_retrieve,
            purge_on_delete = options.purge_on_delete,
            rollback_on_error = options.rollback_on_error,
            run_all_tests = options.run_all_tests,
            single_package = options.single_package,
            test_level = test_level_xml,
            run_tests = run_tests_xml,
        );

        let response = self
            .http_client
            .post(self.metadata_url())
            .headers(self.build_headers("deploy"))
            .body(envelope)
            .send()
            .await?;

        let response_text = response.text().await?;

        // Check for SOAP fault
        if let Some(fault) = self.parse_soap_fault(&response_text) {
            return Err(Error::new(ErrorKind::SoapFault(fault.to_string())));
        }

        self.extract_element(&response_text, "id").ok_or_else(|| {
            Error::new(ErrorKind::InvalidResponse(
                "No async process ID in deploy response".to_string(),
            ))
        })
    }

    /// Check the status of a deploy operation.
    pub async fn check_deploy_status(
        &self,
        async_process_id: &str,
        include_details: bool,
    ) -> Result<DeployResult> {
        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <soap:Header>
    <SessionHeader xmlns="http://soap.sforce.com/2006/04/metadata">
      <sessionId>{session_id}</sessionId>
    </SessionHeader>
  </soap:Header>
  <soap:Body>
    <checkDeployStatus xmlns="http://soap.sforce.com/2006/04/metadata">
      <asyncProcessId>{process_id}</asyncProcessId>
      <includeDetails>{include_details}</includeDetails>
    </checkDeployStatus>
  </soap:Body>
</soap:Envelope>"#,
            session_id = self.access_token,
            process_id = xml::escape(async_process_id),
            include_details = include_details,
        );

        let response = self
            .http_client
            .post(self.metadata_url())
            .headers(self.build_headers("checkDeployStatus"))
            .body(envelope)
            .send()
            .await?;

        let response_text = response.text().await?;

        if let Some(fault) = self.parse_soap_fault(&response_text) {
            return Err(Error::new(ErrorKind::SoapFault(fault.to_string())));
        }

        self.parse_deploy_result(&response_text)
    }

    /// Poll for deploy completion with timeout.
    pub async fn poll_deploy_status(
        &self,
        async_process_id: &str,
        timeout: Duration,
        poll_interval: Duration,
    ) -> Result<DeployResult> {
        let start = tokio::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err(Error::new(ErrorKind::Timeout));
            }

            let result = self.check_deploy_status(async_process_id, true).await?;

            if result.done {
                if result.success {
                    return Ok(result);
                } else {
                    return Err(Error::new(ErrorKind::DeploymentFailed {
                        message: result
                            .error_message
                            .unwrap_or_else(|| "Unknown error".to_string()),
                        failures: result.component_failures,
                    }));
                }
            }

            sleep(poll_interval).await;
        }
    }

    /// Deploy and wait for completion.
    pub async fn deploy_and_wait(
        &self,
        package_zip: &[u8],
        options: DeployOptions,
        timeout: Duration,
        poll_interval: Duration,
    ) -> Result<DeployResult> {
        let async_id = self.deploy(package_zip, options).await?;
        self.poll_deploy_status(&async_id, timeout, poll_interval)
            .await
    }

    // ========================================================================
    // Retrieve Operations
    // ========================================================================

    /// Start a retrieve operation for unpackaged metadata.
    ///
    /// Use a `PackageManifest` to safely specify what to retrieve.
    /// All values are properly XML-escaped to prevent injection attacks.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use busbar_sf_metadata::{MetadataClient, PackageManifest};
    ///
    /// let manifest = PackageManifest::new("62.0")
    ///     .add_type("ApexClass", vec!["*".to_string()])
    ///     .add_type("ApexTrigger", vec!["*".to_string()]);
    ///
    /// let async_id = client.retrieve_unpackaged(&manifest).await?;
    /// ```
    pub async fn retrieve_unpackaged(&self, manifest: &PackageManifest) -> Result<String> {
        // Build XML with proper escaping to prevent injection
        let package_xml = manifest.to_xml();

        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <soap:Header>
    <SessionHeader xmlns="http://soap.sforce.com/2006/04/metadata">
      <sessionId>{session_id}</sessionId>
    </SessionHeader>
  </soap:Header>
  <soap:Body>
    <retrieve xmlns="http://soap.sforce.com/2006/04/metadata">
      <retrieveRequest>
        <apiVersion>{api_version}</apiVersion>
        <unpackaged>
          {package_xml}
        </unpackaged>
      </retrieveRequest>
    </retrieve>
  </soap:Body>
</soap:Envelope>"#,
            session_id = self.access_token,
            api_version = self.api_version,
            package_xml = package_xml,
        );

        let response = self
            .http_client
            .post(self.metadata_url())
            .headers(self.build_headers("retrieve"))
            .body(envelope)
            .send()
            .await?;

        let response_text = response.text().await?;

        if let Some(fault) = self.parse_soap_fault(&response_text) {
            return Err(Error::new(ErrorKind::SoapFault(fault.to_string())));
        }

        self.extract_element(&response_text, "id").ok_or_else(|| {
            Error::new(ErrorKind::InvalidResponse(
                "No async process ID in retrieve response".to_string(),
            ))
        })
    }

    /// Start a retrieve operation for a managed package.
    pub async fn retrieve_packaged(&self, package_name: &str) -> Result<String> {
        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <soap:Header>
    <SessionHeader xmlns="http://soap.sforce.com/2006/04/metadata">
      <sessionId>{session_id}</sessionId>
    </SessionHeader>
  </soap:Header>
  <soap:Body>
    <retrieve xmlns="http://soap.sforce.com/2006/04/metadata">
      <retrieveRequest>
        <apiVersion>{api_version}</apiVersion>
        <packageNames>{package_name}</packageNames>
      </retrieveRequest>
    </retrieve>
  </soap:Body>
</soap:Envelope>"#,
            session_id = self.access_token,
            api_version = self.api_version,
            package_name = xml::escape(package_name),
        );

        let response = self
            .http_client
            .post(self.metadata_url())
            .headers(self.build_headers("retrieve"))
            .body(envelope)
            .send()
            .await?;

        let response_text = response.text().await?;

        if let Some(fault) = self.parse_soap_fault(&response_text) {
            return Err(Error::new(ErrorKind::SoapFault(fault.to_string())));
        }

        self.extract_element(&response_text, "id").ok_or_else(|| {
            Error::new(ErrorKind::InvalidResponse(
                "No async process ID in retrieve response".to_string(),
            ))
        })
    }

    /// Check the status of a retrieve operation.
    pub async fn check_retrieve_status(
        &self,
        async_process_id: &str,
        include_zip: bool,
    ) -> Result<RetrieveResult> {
        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <soap:Header>
    <SessionHeader xmlns="http://soap.sforce.com/2006/04/metadata">
      <sessionId>{session_id}</sessionId>
    </SessionHeader>
  </soap:Header>
  <soap:Body>
    <checkRetrieveStatus xmlns="http://soap.sforce.com/2006/04/metadata">
      <asyncProcessId>{process_id}</asyncProcessId>
      <includeZip>{include_zip}</includeZip>
    </checkRetrieveStatus>
  </soap:Body>
</soap:Envelope>"#,
            session_id = self.access_token,
            process_id = xml::escape(async_process_id),
            include_zip = include_zip,
        );

        let response = self
            .http_client
            .post(self.metadata_url())
            .headers(self.build_headers("checkRetrieveStatus"))
            .body(envelope)
            .send()
            .await?;

        let response_text = response.text().await?;

        if let Some(fault) = self.parse_soap_fault(&response_text) {
            return Err(Error::new(ErrorKind::SoapFault(fault.to_string())));
        }

        self.parse_retrieve_result(&response_text)
    }

    /// Poll for retrieve completion with timeout.
    pub async fn poll_retrieve_status(
        &self,
        async_process_id: &str,
        timeout: Duration,
        poll_interval: Duration,
    ) -> Result<RetrieveResult> {
        let start = tokio::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err(Error::new(ErrorKind::Timeout));
            }

            let result = self.check_retrieve_status(async_process_id, true).await?;

            if result.done {
                if result.success {
                    return Ok(result);
                } else {
                    return Err(Error::new(ErrorKind::RetrieveFailed(
                        result
                            .error_message
                            .unwrap_or_else(|| "Unknown error".to_string()),
                    )));
                }
            }

            sleep(poll_interval).await;
        }
    }

    /// Retrieve and wait for completion.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use busbar_sf_metadata::{MetadataClient, PackageManifest};
    /// use std::time::Duration;
    ///
    /// let manifest = PackageManifest::new("62.0")
    ///     .add_type("ApexClass", vec!["*".to_string()]);
    ///
    /// let result = client.retrieve_unpackaged_and_wait(
    ///     &manifest,
    ///     Duration::from_secs(600),
    ///     Duration::from_secs(5),
    /// ).await?;
    /// ```
    pub async fn retrieve_unpackaged_and_wait(
        &self,
        manifest: &PackageManifest,
        timeout: Duration,
        poll_interval: Duration,
    ) -> Result<RetrieveResult> {
        let async_id = self.retrieve_unpackaged(manifest).await?;
        self.poll_retrieve_status(&async_id, timeout, poll_interval)
            .await
    }

    // ========================================================================
    // List Metadata Operations
    // ========================================================================

    /// List metadata components of a specific type.
    pub async fn list_metadata(
        &self,
        metadata_type: &str,
        folder: Option<&str>,
    ) -> Result<Vec<MetadataComponent>> {
        let folder_xml = folder
            .map(|f| format!("\n      <folder>{}</folder>", xml::escape(f)))
            .unwrap_or_default();

        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <soap:Header>
    <SessionHeader xmlns="http://soap.sforce.com/2006/04/metadata">
      <sessionId>{session_id}</sessionId>
    </SessionHeader>
  </soap:Header>
  <soap:Body>
    <listMetadata xmlns="http://soap.sforce.com/2006/04/metadata">
      <queries>
        <type>{metadata_type}</type>{folder}
      </queries>
      <asOfVersion>{api_version}</asOfVersion>
    </listMetadata>
  </soap:Body>
</soap:Envelope>"#,
            session_id = self.access_token,
            metadata_type = xml::escape(metadata_type),
            folder = folder_xml,
            api_version = self.api_version,
        );

        let response = self
            .http_client
            .post(self.metadata_url())
            .headers(self.build_headers("listMetadata"))
            .body(envelope)
            .send()
            .await?;

        let response_text = response.text().await?;

        if let Some(fault) = self.parse_soap_fault(&response_text) {
            return Err(Error::new(ErrorKind::SoapFault(fault.to_string())));
        }

        self.parse_list_metadata_result(&response_text, metadata_type)
    }

    // ========================================================================
    // Describe Metadata Operations
    // ========================================================================

    /// Describe all available metadata types.
    pub async fn describe_metadata(&self) -> Result<DescribeMetadataResult> {
        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:tns="http://soap.sforce.com/2006/04/metadata">
  <soapenv:Header>
    <tns:SessionHeader>
      <tns:sessionId>{session_id}</tns:sessionId>
    </tns:SessionHeader>
  </soapenv:Header>
  <soapenv:Body>
    <tns:describeMetadata>
      <asOfVersion>{api_version}</asOfVersion>
    </tns:describeMetadata>
  </soapenv:Body>
</soapenv:Envelope>"#,
            session_id = self.access_token,
            api_version = self.api_version,
        );

        let response = self
            .http_client
            .post(self.metadata_url())
            .headers(self.build_headers("describeMetadata"))
            .body(envelope)
            .send()
            .await?;

        let response_text = response.text().await?;

        if let Some(fault) = self.parse_soap_fault(&response_text) {
            return Err(Error::new(ErrorKind::SoapFault(fault.to_string())));
        }

        self.parse_describe_metadata_result(&response_text)
    }

    /// Get a list of all metadata type names.
    pub async fn list_metadata_types(&self) -> Result<Vec<String>> {
        let result = self.describe_metadata().await?;
        let mut types: Vec<String> = result
            .metadata_objects
            .into_iter()
            .flat_map(|obj| {
                let mut names = vec![obj.xml_name];
                names.extend(obj.child_xml_names);
                names
            })
            .collect();
        types.sort();
        types.dedup();
        Ok(types)
    }

    // ========================================================================
    // Private Helper Methods
    // ========================================================================

    /// Parse a SOAP fault from the response.
    fn parse_soap_fault(&self, xml: &str) -> Option<SoapFault> {
        if !xml.contains("faultcode") {
            return None;
        }

        let fault_code = self.extract_element(xml, "faultcode")?;
        let fault_string = self
            .extract_element(xml, "faultstring")
            .unwrap_or_else(|| "Unknown error".to_string());

        Some(SoapFault {
            fault_code,
            fault_string,
        })
    }

    /// Extract a simple element value from XML.
    fn extract_element(&self, xml: &str, tag: &str) -> Option<String> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        let start_patterns = [
            start_tag.clone(),
            format!("<{}:{}>", "sf", tag),
            format!("<{}:{}>", "met", tag),
            format!("<{}:{}>", "tns", tag),
        ];

        for start in &start_patterns {
            if let Some(start_idx) = xml.find(start) {
                let content_start = start_idx + start.len();
                let search_from = &xml[content_start..];
                if let Some(end_idx) = search_from.find(&end_tag).or_else(|| {
                    search_from.find(&format!("</{}", tag.split(':').next_back().unwrap_or(tag)))
                }) {
                    return Some(search_from[..end_idx].to_string());
                }
            }
        }
        None
    }

    /// Extract all elements with a given tag.
    fn extract_elements(&self, xml: &str, tag: &str) -> Vec<String> {
        let mut results = Vec::new();
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        let mut search_from = xml;
        while let Some(start_idx) = search_from.find(&start_tag) {
            let content_start = start_idx + start_tag.len();
            let remaining = &search_from[content_start..];
            if let Some(end_idx) = remaining.find(&end_tag) {
                results.push(remaining[..end_idx].to_string());
                search_from = &remaining[end_idx + end_tag.len()..];
            } else {
                break;
            }
        }
        results
    }

    /// Parse deploy result from XML.
    fn parse_deploy_result(&self, xml: &str) -> Result<DeployResult> {
        let id = self
            .extract_element(xml, "id")
            .ok_or_else(|| Error::new(ErrorKind::InvalidResponse("Missing id".to_string())))?;

        let done = self
            .extract_element(xml, "done")
            .map(|s| s == "true")
            .unwrap_or(false);

        let status_str = self
            .extract_element(xml, "status")
            .unwrap_or_else(|| "Pending".to_string());
        let status = status_str.parse().unwrap_or(DeployStatus::Pending);

        let success = self
            .extract_element(xml, "success")
            .map(|s| s == "true")
            .unwrap_or(false);

        let error_message = self.extract_element(xml, "errorMessage");
        let state_detail = self.extract_element(xml, "stateDetail");

        let number_components_deployed = self
            .extract_element(xml, "numberComponentsDeployed")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let number_components_errors = self
            .extract_element(xml, "numberComponentErrors")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let number_components_total = self
            .extract_element(xml, "numberComponentsTotal")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let number_tests_completed = self
            .extract_element(xml, "numberTestsCompleted")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let number_tests_errors = self
            .extract_element(xml, "numberTestErrors")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let number_tests_total = self
            .extract_element(xml, "numberTestsTotal")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let component_failures = self.parse_component_failures(xml);
        let component_successes = self.parse_component_successes(xml);
        let test_failures = self.parse_test_failures(xml);

        Ok(DeployResult {
            id,
            done,
            status,
            success,
            error_message,
            number_components_deployed,
            number_components_errors,
            number_components_total,
            number_tests_completed,
            number_tests_errors,
            number_tests_total,
            component_failures,
            component_successes,
            test_failures,
            state_detail,
        })
    }

    /// Parse component failures from XML.
    fn parse_component_failures(&self, xml: &str) -> Vec<ComponentFailure> {
        let mut failures = Vec::new();
        let pattern = "<componentFailures>";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</componentFailures>") {
                let block = &remaining[..end + "</componentFailures>".len()];

                let failure = ComponentFailure {
                    component_type: self.extract_element(block, "componentType"),
                    file_name: self.extract_element(block, "fileName"),
                    full_name: self.extract_element(block, "fullName"),
                    line_number: self
                        .extract_element(block, "lineNumber")
                        .and_then(|s| s.parse().ok()),
                    column_number: self
                        .extract_element(block, "columnNumber")
                        .and_then(|s| s.parse().ok()),
                    problem: self
                        .extract_element(block, "problem")
                        .unwrap_or_else(|| "Unknown problem".to_string()),
                    problem_type: self
                        .extract_element(block, "problemType")
                        .unwrap_or_else(|| "Error".to_string()),
                    created: self
                        .extract_element(block, "created")
                        .map(|s| s == "true")
                        .unwrap_or(false),
                    deleted: self
                        .extract_element(block, "deleted")
                        .map(|s| s == "true")
                        .unwrap_or(false),
                };

                failures.push(failure);
                search_from = &remaining[end + "</componentFailures>".len()..];
            } else {
                break;
            }
        }

        failures
    }

    /// Parse component successes from XML.
    fn parse_component_successes(&self, xml: &str) -> Vec<ComponentSuccess> {
        let mut successes = Vec::new();
        let pattern = "<componentSuccesses>";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</componentSuccesses>") {
                let block = &remaining[..end + "</componentSuccesses>".len()];

                let success = ComponentSuccess {
                    component_type: self.extract_element(block, "componentType"),
                    file_name: self.extract_element(block, "fileName"),
                    full_name: self.extract_element(block, "fullName"),
                    created: self
                        .extract_element(block, "created")
                        .map(|s| s == "true")
                        .unwrap_or(false),
                    deleted: self
                        .extract_element(block, "deleted")
                        .map(|s| s == "true")
                        .unwrap_or(false),
                };

                successes.push(success);
                search_from = &remaining[end + "</componentSuccesses>".len()..];
            } else {
                break;
            }
        }

        successes
    }

    /// Parse test failures from XML.
    fn parse_test_failures(&self, xml: &str) -> Vec<TestFailure> {
        let mut failures = Vec::new();
        let pattern = "<failures>";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</failures>") {
                let block = &remaining[..end + "</failures>".len()];

                let failure = TestFailure {
                    name: self.extract_element(block, "name"),
                    method_name: self.extract_element(block, "methodName"),
                    message: self.extract_element(block, "message"),
                    stack_trace: self.extract_element(block, "stackTrace"),
                    namespace: self.extract_element(block, "namespace"),
                };

                failures.push(failure);
                search_from = &remaining[end + "</failures>".len()..];
            } else {
                break;
            }
        }

        failures
    }

    /// Parse retrieve result from XML.
    fn parse_retrieve_result(&self, xml: &str) -> Result<RetrieveResult> {
        let id = self
            .extract_element(xml, "id")
            .ok_or_else(|| Error::new(ErrorKind::InvalidResponse("Missing id".to_string())))?;

        let done = self
            .extract_element(xml, "done")
            .map(|s| s == "true")
            .unwrap_or(false);

        let status_str = self
            .extract_element(xml, "status")
            .unwrap_or_else(|| "Pending".to_string());
        let status = status_str.parse().unwrap_or(RetrieveStatus::Pending);

        let success = self
            .extract_element(xml, "success")
            .map(|s| s == "true")
            .unwrap_or(false);

        let error_message = self.extract_element(xml, "errorMessage");
        let error_status_code = self.extract_element(xml, "errorStatusCode");
        let zip_file = self.extract_element(xml, "zipFile");

        let file_properties = self.parse_file_properties(xml);
        let messages = self.parse_retrieve_messages(xml);

        Ok(RetrieveResult {
            id,
            done,
            status,
            success,
            error_message,
            error_status_code,
            zip_file,
            file_properties,
            messages,
        })
    }

    /// Parse file properties from retrieve result.
    fn parse_file_properties(&self, xml: &str) -> Vec<FileProperties> {
        let mut properties = Vec::new();
        let pattern = "<fileProperties>";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</fileProperties>") {
                let block = &remaining[..end + "</fileProperties>".len()];

                if let (Some(file_name), Some(full_name), Some(id)) = (
                    self.extract_element(block, "fileName"),
                    self.extract_element(block, "fullName"),
                    self.extract_element(block, "id"),
                ) {
                    let prop = FileProperties {
                        created_by_id: self
                            .extract_element(block, "createdById")
                            .unwrap_or_default(),
                        created_by_name: self
                            .extract_element(block, "createdByName")
                            .unwrap_or_default(),
                        created_date: self
                            .extract_element(block, "createdDate")
                            .unwrap_or_default(),
                        file_name,
                        full_name,
                        id,
                        last_modified_by_id: self
                            .extract_element(block, "lastModifiedById")
                            .unwrap_or_default(),
                        last_modified_by_name: self
                            .extract_element(block, "lastModifiedByName")
                            .unwrap_or_default(),
                        last_modified_date: self
                            .extract_element(block, "lastModifiedDate")
                            .unwrap_or_default(),
                        manageable_state: self.extract_element(block, "manageableState"),
                        namespace_prefix: self.extract_element(block, "namespacePrefix"),
                        component_type: self.extract_element(block, "type").unwrap_or_default(),
                    };
                    properties.push(prop);
                }

                search_from = &remaining[end + "</fileProperties>".len()..];
            } else {
                break;
            }
        }

        properties
    }

    /// Parse retrieve messages from XML.
    fn parse_retrieve_messages(&self, xml: &str) -> Vec<RetrieveMessage> {
        let mut messages = Vec::new();
        let pattern = "<messages>";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</messages>") {
                let block = &remaining[..end + "</messages>".len()];

                if let (Some(file_name), Some(problem)) = (
                    self.extract_element(block, "fileName"),
                    self.extract_element(block, "problem"),
                ) {
                    messages.push(RetrieveMessage { file_name, problem });
                }

                search_from = &remaining[end + "</messages>".len()..];
            } else {
                break;
            }
        }

        messages
    }

    /// Parse list metadata result.
    fn parse_list_metadata_result(
        &self,
        xml: &str,
        metadata_type: &str,
    ) -> Result<Vec<MetadataComponent>> {
        let mut results = Vec::new();
        let pattern = "<result>";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</result>") {
                let block = &remaining[..end + "</result>".len()];

                if let Some(full_name) = self.extract_element(block, "fullName") {
                    let info = MetadataComponent {
                        created_by_id: self.extract_element(block, "createdById"),
                        created_by_name: self.extract_element(block, "createdByName"),
                        created_date: self.extract_element(block, "createdDate"),
                        file_name: self.extract_element(block, "fileName"),
                        full_name,
                        id: self.extract_element(block, "id"),
                        last_modified_by_id: self.extract_element(block, "lastModifiedById"),
                        last_modified_by_name: self.extract_element(block, "lastModifiedByName"),
                        last_modified_date: self.extract_element(block, "lastModifiedDate"),
                        manageable_state: self.extract_element(block, "manageableState"),
                        namespace_prefix: self.extract_element(block, "namespacePrefix"),
                        metadata_type: self
                            .extract_element(block, "type")
                            .unwrap_or_else(|| metadata_type.to_string()),
                    };
                    results.push(info);
                }

                search_from = &remaining[end + "</result>".len()..];
            } else {
                break;
            }
        }

        Ok(results)
    }

    /// Parse describe metadata result.
    fn parse_describe_metadata_result(&self, xml: &str) -> Result<DescribeMetadataResult> {
        let mut metadata_objects = Vec::new();
        let pattern = "<metadataObjects>";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</metadataObjects>") {
                let block = &remaining[..end + "</metadataObjects>".len()];

                if let Some(xml_name) = self.extract_element(block, "xmlName") {
                    let child_names = self.extract_elements(block, "childXmlNames");

                    let desc = MetadataType {
                        xml_name,
                        directory_name: self.extract_element(block, "directoryName"),
                        suffix: self.extract_element(block, "suffix"),
                        in_folder: self
                            .extract_element(block, "inFolder")
                            .map(|s| s == "true")
                            .unwrap_or(false),
                        meta_file: self
                            .extract_element(block, "metaFile")
                            .map(|s| s == "true")
                            .unwrap_or(false),
                        child_xml_names: child_names,
                    };
                    metadata_objects.push(desc);
                }

                search_from = &remaining[end + "</metadataObjects>".len()..];
            } else {
                break;
            }
        }

        let organization_namespace = self.extract_element(xml, "organizationNamespace");
        let partial_save_allowed = self
            .extract_element(xml, "partialSaveAllowed")
            .map(|s| s == "true")
            .unwrap_or(false);
        let test_required = self
            .extract_element(xml, "testRequired")
            .map(|s| s == "true")
            .unwrap_or(false);

        Ok(DescribeMetadataResult {
            metadata_objects,
            organization_namespace,
            partial_save_allowed,
            test_required,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = MetadataClient::from_parts("https://test.salesforce.com", "token123");
        assert_eq!(client.api_version, DEFAULT_API_VERSION);
    }

    #[test]
    fn test_client_with_version() {
        let client = MetadataClient::from_parts("https://test.salesforce.com", "token123")
            .with_api_version("58.0");
        assert_eq!(client.api_version, "58.0");
    }

    #[test]
    fn test_extract_element() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = "<root><id>12345</id><done>true</done></root>";

        assert_eq!(client.extract_element(xml, "id"), Some("12345".to_string()));
        assert_eq!(
            client.extract_element(xml, "done"),
            Some("true".to_string())
        );
        assert_eq!(client.extract_element(xml, "missing"), None);
    }

    #[test]
    fn test_parse_deploy_result() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <checkDeployStatusResponse>
                <result>
                    <id>0Af123</id>
                    <done>true</done>
                    <status>Succeeded</status>
                    <success>true</success>
                    <numberComponentsDeployed>5</numberComponentsDeployed>
                    <numberComponentErrors>0</numberComponentErrors>
                    <numberComponentsTotal>5</numberComponentsTotal>
                </result>
            </checkDeployStatusResponse>
        "#;

        let result = client.parse_deploy_result(xml).unwrap();
        assert_eq!(result.id, "0Af123");
        assert!(result.done);
        assert_eq!(result.status, DeployStatus::Succeeded);
        assert!(result.success);
        assert_eq!(result.number_components_deployed, 5);
    }

    #[test]
    fn test_parse_component_failures() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <componentFailures>
                <componentType>ApexClass</componentType>
                <fileName>classes/MyClass.cls</fileName>
                <fullName>MyClass</fullName>
                <lineNumber>10</lineNumber>
                <problem>Missing semicolon</problem>
                <problemType>Error</problemType>
                <created>false</created>
                <deleted>false</deleted>
            </componentFailures>
        "#;

        let failures = client.parse_component_failures(xml);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].component_type, Some("ApexClass".to_string()));
        assert_eq!(failures[0].problem, "Missing semicolon");
        assert_eq!(failures[0].line_number, Some(10));
    }

    #[test]
    fn test_parse_retrieve_result() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <checkRetrieveStatusResponse>
                <result>
                    <id>09S123</id>
                    <done>true</done>
                    <status>Succeeded</status>
                    <success>true</success>
                    <zipFile>UEsDBBQ...</zipFile>
                </result>
            </checkRetrieveStatusResponse>
        "#;

        let result = client.parse_retrieve_result(xml).unwrap();
        assert_eq!(result.id, "09S123");
        assert!(result.done);
        assert_eq!(result.status, RetrieveStatus::Succeeded);
        assert!(result.zip_file.is_some());
    }
}
