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

    /// Describe the fields and structure of a specific metadata value type.
    ///
    /// The `type_name` parameter must be a fully namespace-qualified type name
    /// in the format `{http://soap.sforce.com/2006/04/metadata}TypeName`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let result = client.describe_value_type(
    ///     "{http://soap.sforce.com/2006/04/metadata}CustomObject"
    /// ).await?;
    ///
    /// for field in &result.value_type_fields {
    ///     println!("Field: {} ({})", field.name, field.soap_type);
    /// }
    /// ```
    ///
    /// Available since API v30.0.
    pub async fn describe_value_type(
        &self,
        type_name: &str,
    ) -> Result<crate::describe::DescribeValueTypeResult> {
        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:met="http://soap.sforce.com/2006/04/metadata">
  <soapenv:Header>
    <met:SessionHeader>
      <met:sessionId>{session_id}</met:sessionId>
    </met:SessionHeader>
  </soapenv:Header>
  <soapenv:Body>
    <met:describeValueType>
      <met:type>{type_name}</met:type>
    </met:describeValueType>
  </soapenv:Body>
</soapenv:Envelope>"#,
            session_id = xml::escape(&self.access_token),
            type_name = xml::escape(type_name),
        );

        let response = self
            .http_client
            .post(self.metadata_url())
            .headers(self.build_headers("describeValueType"))
            .body(envelope)
            .send()
            .await?;

        let response_text = response.text().await?;

        if let Some(fault) = self.parse_soap_fault(&response_text) {
            return Err(Error::new(ErrorKind::SoapFault(fault.to_string())));
        }

        self.parse_describe_value_type_result(&response_text)
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

    /// Parse describe value type result from XML.
    fn parse_describe_value_type_result(
        &self,
        xml: &str,
    ) -> Result<crate::describe::DescribeValueTypeResult> {
        // Extract all top-level valueTypeFields
        let value_type_fields = self.parse_value_type_fields(xml, "valueTypeFields");

        // Extract the optional parentField
        let parent_field = if xml.contains("<parentField>") {
            self.parse_single_value_type_field(xml, "parentField")
        } else {
            None
        };

        Ok(crate::describe::DescribeValueTypeResult {
            value_type_fields,
            parent_field,
        })
    }

    /// Parse all ValueTypeField elements with a specific tag.
    fn parse_value_type_fields(
        &self,
        xml: &str,
        tag: &str,
    ) -> Vec<crate::describe::ValueTypeField> {
        let mut fields = Vec::new();
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        let mut search_from = xml;
        while let Some(start_idx) = search_from.find(&start_tag) {
            let remaining = &search_from[start_idx + start_tag.len()..];

            // Find the matching closing tag by counting depth
            let mut depth = 1;
            let mut pos = 0;
            let mut found_end = None;

            while pos < remaining.len() && depth > 0 {
                if remaining[pos..].starts_with(&start_tag) {
                    depth += 1;
                    pos += start_tag.len();
                } else if remaining[pos..].starts_with(&end_tag) {
                    depth -= 1;
                    if depth == 0 {
                        found_end = Some(pos);
                        break;
                    }
                    pos += end_tag.len();
                } else {
                    pos += 1;
                }
            }

            if let Some(end_pos) = found_end {
                // Extract the content within this field (without the tags)
                let field_content = &remaining[..end_pos];

                // Parse this field
                if let Some(field) = self.parse_value_type_field_from_content(field_content) {
                    fields.push(field);
                }

                // Move past this field to find more siblings
                search_from = &remaining[end_pos + end_tag.len()..];
            } else {
                // No matching closing tag found, stop searching
                break;
            }
        }
        fields
    }

    /// Parse a single ValueTypeField with a specific tag (e.g., "parentField").
    fn parse_single_value_type_field(
        &self,
        xml: &str,
        tag: &str,
    ) -> Option<crate::describe::ValueTypeField> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        if let Some(start_idx) = xml.find(&start_tag) {
            let remaining = &xml[start_idx + start_tag.len()..];
            if let Some(end_idx) = remaining.find(&end_tag) {
                let content = &remaining[..end_idx];
                return self.parse_value_type_field_from_content(content);
            }
        }
        None
    }

    /// Parse nested ValueTypeField elements within a parent field block.
    /// This handles the recursive structure where fields can contain other fields.
    fn parse_nested_value_type_fields_in_block(
        &self,
        block: &str,
    ) -> Vec<crate::describe::ValueTypeField> {
        let mut nested_fields = Vec::new();
        let start_tag = "<valueTypeFields>";
        let end_tag = "</valueTypeFields>";

        // Find the first occurrence of the start tag - this would be a nested field
        // (the outer field's opening tag would be before 'block')
        let mut search_from = block;

        while let Some(start_idx) = search_from.find(start_tag) {
            let remaining = &search_from[start_idx + start_tag.len()..];

            // Find the matching closing tag by counting depth
            let mut depth = 1;
            let mut pos = 0;
            let mut found_end = None;

            while pos < remaining.len() && depth > 0 {
                if remaining[pos..].starts_with(start_tag) {
                    depth += 1;
                    pos += start_tag.len();
                } else if remaining[pos..].starts_with(end_tag) {
                    depth -= 1;
                    if depth == 0 {
                        found_end = Some(pos);
                        break;
                    }
                    pos += end_tag.len();
                } else {
                    pos += 1;
                }
            }

            if let Some(end_pos) = found_end {
                // Extract the content within this nested field
                let field_content = &remaining[..end_pos];

                // Parse this nested field recursively
                if let Some(field) = self.parse_value_type_field_from_content(field_content) {
                    nested_fields.push(field);
                }

                // Move past this field to find more siblings
                search_from = &remaining[end_pos + end_tag.len()..];
            } else {
                // No matching closing tag found, stop searching
                break;
            }
        }

        nested_fields
    }

    /// Parse a ValueTypeField from the content (without the wrapping tags).
    fn parse_value_type_field_from_content(
        &self,
        content: &str,
    ) -> Option<crate::describe::ValueTypeField> {
        let name = self.extract_element(content, "name")?;
        let soap_type = self.extract_element(content, "soapType")?;

        let is_foreign_key = self
            .extract_element(content, "isForeignKey")
            .map(|s| s == "true")
            .unwrap_or(false);

        let foreign_key_domain = self.extract_element(content, "foreignKeyDomain");

        let is_name_field = self
            .extract_element(content, "isNameField")
            .map(|s| s == "true")
            .unwrap_or(false);

        let min_occurs = self
            .extract_element(content, "minOccurs")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let max_occurs = self
            .extract_element(content, "maxOccurs")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        // Parse picklist values
        let picklist_values = self.parse_picklist_entries(content);

        // Recursively parse nested fields
        let fields = self.parse_nested_value_type_fields_in_block(content);

        Some(crate::describe::ValueTypeField {
            name,
            soap_type,
            is_foreign_key,
            foreign_key_domain,
            is_name_field,
            min_occurs,
            max_occurs,
            fields,
            picklist_values,
        })
    }

    /// Parse all PicklistEntry elements from XML.
    fn parse_picklist_entries(&self, xml: &str) -> Vec<crate::describe::PicklistEntry> {
        let mut entries = Vec::new();
        let start_tag = "<picklistValues>";
        let end_tag = "</picklistValues>";

        let mut search_from = xml;
        while let Some(start_idx) = search_from.find(start_tag) {
            let remaining = &search_from[start_idx..];
            if let Some(end_idx) = remaining.find(end_tag) {
                let block = &remaining[..end_idx + end_tag.len()];

                let active = self
                    .extract_element(block, "active")
                    .map(|s| s == "true")
                    .unwrap_or(false);

                let default_value = self
                    .extract_element(block, "defaultValue")
                    .map(|s| s == "true")
                    .unwrap_or(false);

                let label = self.extract_element(block, "label").unwrap_or_default();
                let value = self.extract_element(block, "value").unwrap_or_default();

                entries.push(crate::describe::PicklistEntry {
                    active,
                    default_value,
                    label,
                    value,
                });

                search_from = &remaining[end_idx + end_tag.len()..];
            } else {
                break;
            }
        }
        entries
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

    #[test]
    fn test_metadata_url_construction() {
        let client = MetadataClient::from_parts("https://na1.salesforce.com", "token")
            .with_api_version("62.0");
        assert_eq!(
            client.metadata_url(),
            "https://na1.salesforce.com/services/Soap/m/62.0"
        );
    }

    #[test]
    fn test_build_headers() {
        let client = MetadataClient::from_parts("https://na1.salesforce.com", "token123");
        let headers = client.build_headers("deploy");

        assert_eq!(
            headers.get("content-type").unwrap(),
            "text/xml;charset=UTF-8"
        );
        assert_eq!(headers.get("soapaction").unwrap(), "deploy");
        assert_eq!(headers.get("authorization").unwrap(), "Bearer token123");
    }

    #[test]
    fn test_parse_soap_fault() {
        let client = MetadataClient::from_parts("url", "token");

        let xml = r#"
        <soap:Envelope>
            <soap:Body>
                <soap:Fault>
                    <faultcode>sf:INVALID_SESSION_ID</faultcode>
                    <faultstring>Session expired or invalid</faultstring>
                </soap:Fault>
            </soap:Body>
        </soap:Envelope>"#;

        let fault = client.parse_soap_fault(xml).unwrap();
        assert_eq!(fault.fault_code, "sf:INVALID_SESSION_ID");
        assert_eq!(fault.fault_string, "Session expired or invalid");
    }

    #[test]
    fn test_parse_soap_fault_returns_none_for_success() {
        let client = MetadataClient::from_parts("url", "token");
        let xml =
            "<soap:Envelope><soap:Body><result><id>123</id></result></soap:Body></soap:Envelope>";
        assert!(client.parse_soap_fault(xml).is_none());
    }

    #[test]
    fn test_extract_elements_multiple() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = "<root><name>Alice</name><name>Bob</name><name>Charlie</name></root>";
        let names = client.extract_elements(xml, "name");
        assert_eq!(names, vec!["Alice", "Bob", "Charlie"]);
    }

    #[test]
    fn test_extract_elements_empty() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = "<root><other>value</other></root>";
        let names = client.extract_elements(xml, "name");
        assert!(names.is_empty());
    }

    #[test]
    fn test_extract_element_with_namespaced_open_tag() {
        let client = MetadataClient::from_parts("url", "token");
        // Salesforce SOAP responses sometimes use namespace-prefixed open tags
        // with un-prefixed close tags
        let xml = "<root><sf:id>12345</id></root>";
        assert_eq!(client.extract_element(xml, "id"), Some("12345".to_string()));

        let xml = "<root><met:status>Succeeded</status></root>";
        assert_eq!(
            client.extract_element(xml, "status"),
            Some("Succeeded".to_string())
        );

        let xml = "<root><tns:done>true</done></root>";
        assert_eq!(
            client.extract_element(xml, "done"),
            Some("true".to_string())
        );
    }

    #[test]
    fn test_parse_deploy_result_with_failures() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <result>
                <id>0Af456</id>
                <done>true</done>
                <status>Failed</status>
                <success>false</success>
                <numberComponentsDeployed>3</numberComponentsDeployed>
                <numberComponentErrors>2</numberComponentErrors>
                <numberComponentsTotal>5</numberComponentsTotal>
                <numberTestsCompleted>10</numberTestsCompleted>
                <numberTestErrors>1</numberTestErrors>
                <numberTestsTotal>11</numberTestsTotal>
                <errorMessage>Deployment failed</errorMessage>
                <componentFailures>
                    <componentType>ApexClass</componentType>
                    <fullName>BadClass</fullName>
                    <problem>Compilation error</problem>
                    <problemType>Error</problemType>
                    <lineNumber>42</lineNumber>
                    <columnNumber>5</columnNumber>
                    <created>false</created>
                    <deleted>false</deleted>
                </componentFailures>
            </result>
        "#;

        let result = client.parse_deploy_result(xml).unwrap();
        assert_eq!(result.id, "0Af456");
        assert!(!result.success);
        assert_eq!(result.status, DeployStatus::Failed);
        assert_eq!(result.number_components_deployed, 3);
        assert_eq!(result.number_components_errors, 2);
        assert_eq!(result.number_tests_completed, 10);
        assert_eq!(result.number_tests_errors, 1);
        assert_eq!(result.error_message, Some("Deployment failed".to_string()));
        assert_eq!(result.component_failures.len(), 1);
        assert_eq!(
            result.component_failures[0].full_name,
            Some("BadClass".to_string())
        );
        assert_eq!(result.component_failures[0].line_number, Some(42));
        assert_eq!(result.component_failures[0].column_number, Some(5));
    }

    #[test]
    fn test_parse_test_failures() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <failures>
                <name>MyTestClass</name>
                <methodName>testInsert</methodName>
                <message>System.AssertException: Expected 1, got 2</message>
                <stackTrace>Class.MyTestClass.testInsert: line 15, column 1</stackTrace>
            </failures>
            <failures>
                <name>MyTestClass</name>
                <methodName>testUpdate</methodName>
                <message>Null pointer</message>
            </failures>
        "#;

        let failures = client.parse_test_failures(xml);
        assert_eq!(failures.len(), 2);
        assert_eq!(failures[0].name, Some("MyTestClass".to_string()));
        assert_eq!(failures[0].method_name, Some("testInsert".to_string()));
        assert!(failures[0]
            .message
            .as_ref()
            .unwrap()
            .contains("AssertException"));
        assert!(failures[0].stack_trace.is_some());
        assert!(failures[1].stack_trace.is_none());
    }

    #[test]
    fn test_parse_component_successes() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <componentSuccesses>
                <componentType>ApexClass</componentType>
                <fileName>classes/MyClass.cls</fileName>
                <fullName>MyClass</fullName>
                <created>true</created>
                <deleted>false</deleted>
            </componentSuccesses>
        "#;

        let successes = client.parse_component_successes(xml);
        assert_eq!(successes.len(), 1);
        assert_eq!(successes[0].component_type, Some("ApexClass".to_string()));
        assert_eq!(successes[0].full_name, Some("MyClass".to_string()));
        assert!(successes[0].created);
        assert!(!successes[0].deleted);
    }

    #[test]
    fn test_parse_retrieve_result_with_messages_and_files() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <result>
                <id>09S789</id>
                <done>true</done>
                <status>Succeeded</status>
                <success>true</success>
                <fileProperties>
                    <fullName>MyClass</fullName>
                    <fileName>classes/MyClass.cls</fileName>
                    <type>ApexClass</type>
                    <id>01p123</id>
                </fileProperties>
                <messages>
                    <fileName>classes/OldClass.cls</fileName>
                    <problem>Entity is deleted</problem>
                </messages>
            </result>
        "#;

        let result = client.parse_retrieve_result(xml).unwrap();
        assert!(result.success);
        assert_eq!(result.file_properties.len(), 1);
        assert_eq!(result.file_properties[0].full_name, "MyClass");
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.messages[0].file_name, "classes/OldClass.cls");
        assert_eq!(result.messages[0].problem, "Entity is deleted");
    }

    #[test]
    fn test_parse_describe_value_type_result() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <describeValueTypeResponse>
                <result>
                    <valueTypeFields>
                        <name>fullName</name>
                        <soapType>xsd:string</soapType>
                        <isForeignKey>false</isForeignKey>
                        <isNameField>true</isNameField>
                        <minOccurs>1</minOccurs>
                        <maxOccurs>1</maxOccurs>
                    </valueTypeFields>
                    <valueTypeFields>
                        <name>label</name>
                        <soapType>xsd:string</soapType>
                        <isForeignKey>false</isForeignKey>
                        <isNameField>false</isNameField>
                        <minOccurs>1</minOccurs>
                        <maxOccurs>1</maxOccurs>
                    </valueTypeFields>
                    <valueTypeFields>
                        <name>deploymentStatus</name>
                        <soapType>tns:DeploymentStatus</soapType>
                        <isForeignKey>false</isForeignKey>
                        <isNameField>false</isNameField>
                        <minOccurs>0</minOccurs>
                        <maxOccurs>1</maxOccurs>
                        <picklistValues>
                            <active>true</active>
                            <defaultValue>false</defaultValue>
                            <label>In Development</label>
                            <value>InDevelopment</value>
                        </picklistValues>
                        <picklistValues>
                            <active>true</active>
                            <defaultValue>true</defaultValue>
                            <label>Deployed</label>
                            <value>Deployed</value>
                        </picklistValues>
                    </valueTypeFields>
                    <parentField>
                        <name>Metadata</name>
                        <soapType>tns:Metadata</soapType>
                        <isForeignKey>false</isForeignKey>
                        <isNameField>false</isNameField>
                        <minOccurs>0</minOccurs>
                        <maxOccurs>1</maxOccurs>
                    </parentField>
                </result>
            </describeValueTypeResponse>
        "#;

        let result = client.parse_describe_value_type_result(xml).unwrap();

        // Check value type fields
        assert_eq!(result.value_type_fields.len(), 3);

        // Check first field (fullName)
        assert_eq!(result.value_type_fields[0].name, "fullName");
        assert_eq!(result.value_type_fields[0].soap_type, "xsd:string");
        assert!(result.value_type_fields[0].is_name_field);
        assert!(!result.value_type_fields[0].is_foreign_key);
        assert_eq!(result.value_type_fields[0].min_occurs, 1);
        assert_eq!(result.value_type_fields[0].max_occurs, 1);

        // Check second field (label)
        assert_eq!(result.value_type_fields[1].name, "label");
        assert!(!result.value_type_fields[1].is_name_field);

        // Check third field (deploymentStatus) with picklist values
        assert_eq!(result.value_type_fields[2].name, "deploymentStatus");
        assert_eq!(result.value_type_fields[2].picklist_values.len(), 2);
        assert_eq!(
            result.value_type_fields[2].picklist_values[0].label,
            "In Development"
        );
        assert_eq!(
            result.value_type_fields[2].picklist_values[0].value,
            "InDevelopment"
        );
        assert!(!result.value_type_fields[2].picklist_values[0].default_value);
        assert_eq!(
            result.value_type_fields[2].picklist_values[1].label,
            "Deployed"
        );
        assert!(result.value_type_fields[2].picklist_values[1].default_value);

        // Check parent field
        assert!(result.parent_field.is_some());
        let parent = result.parent_field.unwrap();
        assert_eq!(parent.name, "Metadata");
        assert_eq!(parent.soap_type, "tns:Metadata");
    }

    #[test]
    fn test_parse_describe_value_type_result_with_nested_fields() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <describeValueTypeResponse>
                <result>
                    <valueTypeFields>
                        <name>address</name>
                        <soapType>tns:Address</soapType>
                        <isForeignKey>false</isForeignKey>
                        <isNameField>false</isNameField>
                        <minOccurs>0</minOccurs>
                        <maxOccurs>1</maxOccurs>
                        <valueTypeFields>
                            <name>street</name>
                            <soapType>xsd:string</soapType>
                            <isForeignKey>false</isForeignKey>
                            <isNameField>false</isNameField>
                            <minOccurs>0</minOccurs>
                            <maxOccurs>1</maxOccurs>
                        </valueTypeFields>
                        <valueTypeFields>
                            <name>city</name>
                            <soapType>xsd:string</soapType>
                            <isForeignKey>false</isForeignKey>
                            <isNameField>false</isNameField>
                            <minOccurs>0</minOccurs>
                            <maxOccurs>1</maxOccurs>
                        </valueTypeFields>
                    </valueTypeFields>
                </result>
            </describeValueTypeResponse>
        "#;

        let result = client.parse_describe_value_type_result(xml).unwrap();

        // Check that we have the parent field
        assert_eq!(result.value_type_fields.len(), 1);
        let address_field = &result.value_type_fields[0];
        assert_eq!(address_field.name, "address");
        assert_eq!(address_field.soap_type, "tns:Address");

        // Check that nested fields are parsed
        assert_eq!(address_field.fields.len(), 2);
        assert_eq!(address_field.fields[0].name, "street");
        assert_eq!(address_field.fields[0].soap_type, "xsd:string");
        assert_eq!(address_field.fields[1].name, "city");
        assert_eq!(address_field.fields[1].soap_type, "xsd:string");
    }

    #[tokio::test]
    async fn test_describe_value_type_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let soap_response = r#"<?xml version="1.0" encoding="UTF-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
  <soapenv:Body>
    <describeValueTypeResponse xmlns="http://soap.sforce.com/2006/04/metadata">
      <result>
        <valueTypeFields>
          <name>fullName</name>
          <soapType>xsd:string</soapType>
          <isForeignKey>false</isForeignKey>
          <isNameField>true</isNameField>
          <minOccurs>1</minOccurs>
          <maxOccurs>1</maxOccurs>
        </valueTypeFields>
        <valueTypeFields>
          <name>label</name>
          <soapType>xsd:string</soapType>
          <isForeignKey>false</isForeignKey>
          <isNameField>false</isNameField>
          <minOccurs>0</minOccurs>
          <maxOccurs>1</maxOccurs>
        </valueTypeFields>
      </result>
    </describeValueTypeResponse>
  </soapenv:Body>
</soapenv:Envelope>"#;

        Mock::given(method("POST"))
            .and(path_regex("/services/Soap/m/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_string(soap_response))
            .mount(&mock_server)
            .await;

        let client = MetadataClient::from_parts(mock_server.uri(), "test-token");

        let result = client
            .describe_value_type("{http://soap.sforce.com/2006/04/metadata}CustomObject")
            .await
            .expect("describe_value_type should succeed");

        assert_eq!(result.value_type_fields.len(), 2);
        assert_eq!(result.value_type_fields[0].name, "fullName");
        assert!(result.value_type_fields[0].is_name_field);
        assert_eq!(result.value_type_fields[1].name, "label");
        assert!(!result.value_type_fields[1].is_name_field);
        assert!(result.parent_field.is_none());
    }

    #[tokio::test]
    async fn test_describe_value_type_soap_fault() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let fault_response = r#"<?xml version="1.0" encoding="UTF-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
  <soapenv:Body>
    <soapenv:Fault>
      <faultcode>sf:INVALID_TYPE</faultcode>
      <faultstring>Invalid type: {http://soap.sforce.com/2006/04/metadata}BadType</faultstring>
    </soapenv:Fault>
  </soapenv:Body>
</soapenv:Envelope>"#;

        Mock::given(method("POST"))
            .and(path_regex("/services/Soap/m/.*"))
            .respond_with(ResponseTemplate::new(500).set_body_string(fault_response))
            .mount(&mock_server)
            .await;

        let client = MetadataClient::from_parts(mock_server.uri(), "test-token");

        let result = client
            .describe_value_type("{http://soap.sforce.com/2006/04/metadata}BadType")
            .await;

        assert!(result.is_err(), "Should fail with SOAP fault");
    }
}
