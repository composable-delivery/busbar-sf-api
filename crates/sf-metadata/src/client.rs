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
    ComponentSuccess, DeleteResult, FileProperties, MetadataError, ReadResult, SaveResult,
    SoapFault, TestFailure, TestLevel, UpsertResult, DEFAULT_API_VERSION,
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
    // CRUD-Based Metadata Operations
    // ========================================================================

    /// Create one or more metadata components.
    ///
    /// This is a synchronous CRUD operation that creates new metadata components
    /// without requiring zip packaging. Maximum 10 components per call.
    ///
    /// # Limitations
    ///
    /// - Does NOT support ApexClass or ApexTrigger (use deploy/retrieve for those)
    /// - Maximum 10 metadata components per call
    /// - Available since API version 30.0
    ///
    /// # Arguments
    ///
    /// * `metadata_type` - The type of metadata component (e.g., "CustomObject", "CustomField")
    /// * `metadata_objects` - Array of metadata objects as JSON values
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use serde_json::json;
    ///
    /// let metadata = vec![json!({
    ///     "fullName": "MyCustomObject__c",
    ///     "label": "My Custom Object",
    ///     "pluralLabel": "My Custom Objects",
    ///     "nameField": {
    ///         "type": "AutoNumber",
    ///         "label": "Record Number"
    ///     }
    /// })];
    ///
    /// let results = client.create_metadata("CustomObject", &metadata).await?;
    /// for result in results {
    ///     if result.success {
    ///         println!("Created: {}", result.full_name);
    ///     } else {
    ///         println!("Failed: {}", result.full_name);
    ///     }
    /// }
    /// ```
    pub async fn create_metadata(
        &self,
        metadata_type: &str,
        metadata_objects: &[serde_json::Value],
    ) -> Result<Vec<SaveResult>> {
        if metadata_objects.is_empty() {
            return Ok(Vec::new());
        }
        if metadata_objects.len() > 10 {
            return Err(Error::new(ErrorKind::Other(
                "Maximum 10 metadata components per call".to_string(),
            )));
        }

        let metadata_elements: Vec<String> = metadata_objects
            .iter()
            .map(|obj| self.build_metadata_element(metadata_type, obj))
            .collect();

        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:met="http://soap.sforce.com/2006/04/metadata" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <soapenv:Header>
    <met:SessionHeader>
      <met:sessionId>{session_id}</met:sessionId>
    </met:SessionHeader>
  </soapenv:Header>
  <soapenv:Body>
    <met:createMetadata>
{metadata_elements}
    </met:createMetadata>
  </soapenv:Body>
</soapenv:Envelope>"#,
            session_id = self.access_token,
            metadata_elements = metadata_elements.join("\n"),
        );

        let response = self
            .http_client
            .post(self.metadata_url())
            .headers(self.build_headers("createMetadata"))
            .body(envelope)
            .send()
            .await?;

        let response_text = response.text().await?;

        if let Some(fault) = self.parse_soap_fault(&response_text) {
            return Err(Error::new(ErrorKind::SoapFault(fault.to_string())));
        }

        self.parse_save_results(&response_text)
    }

    /// Read metadata components by type and full names.
    ///
    /// This is a synchronous operation that retrieves metadata components.
    /// The returned metadata objects are polymorphic and contain type-specific fields.
    ///
    /// # Limitations
    ///
    /// - Does NOT support ApexClass or ApexTrigger (use deploy/retrieve for those)
    /// - Available since API version 30.0
    ///
    /// # Arguments
    ///
    /// * `metadata_type` - The type of metadata component (e.g., "CustomObject")
    /// * `full_names` - Array of full names to read
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = client.read_metadata(
    ///     "CustomObject",
    ///     &["Account", "Contact"]
    /// ).await?;
    ///
    /// for record in result.records {
    ///     println!("Retrieved: {:?}", record);
    /// }
    /// ```
    pub async fn read_metadata(
        &self,
        metadata_type: &str,
        full_names: &[&str],
    ) -> Result<ReadResult> {
        if full_names.is_empty() {
            return Ok(ReadResult {
                records: Vec::new(),
            });
        }

        let full_name_elements: String = full_names
            .iter()
            .map(|name| format!("      <met:fullNames>{}</met:fullNames>", xml::escape(name)))
            .collect::<Vec<_>>()
            .join("\n");

        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:met="http://soap.sforce.com/2006/04/metadata">
  <soapenv:Header>
    <met:SessionHeader>
      <met:sessionId>{session_id}</met:sessionId>
    </met:SessionHeader>
  </soapenv:Header>
  <soapenv:Body>
    <met:readMetadata>
      <met:type>{metadata_type}</met:type>
{full_name_elements}
    </met:readMetadata>
  </soapenv:Body>
</soapenv:Envelope>"#,
            session_id = self.access_token,
            metadata_type = xml::escape(metadata_type),
            full_name_elements = full_name_elements,
        );

        let response = self
            .http_client
            .post(self.metadata_url())
            .headers(self.build_headers("readMetadata"))
            .body(envelope)
            .send()
            .await?;

        let response_text = response.text().await?;

        if let Some(fault) = self.parse_soap_fault(&response_text) {
            return Err(Error::new(ErrorKind::SoapFault(fault.to_string())));
        }

        self.parse_read_result(&response_text)
    }

    /// Update one or more existing metadata components.
    ///
    /// This is a synchronous CRUD operation that updates existing metadata components
    /// without requiring zip packaging. Maximum 10 components per call.
    ///
    /// # Limitations
    ///
    /// - Does NOT support ApexClass or ApexTrigger (use deploy/retrieve for those)
    /// - Maximum 10 metadata components per call
    /// - Available since API version 30.0
    ///
    /// # Arguments
    ///
    /// * `metadata_type` - The type of metadata component (e.g., "CustomObject", "CustomField")
    /// * `metadata_objects` - Array of metadata objects as JSON values
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use serde_json::json;
    ///
    /// let metadata = vec![json!({
    ///     "fullName": "MyCustomObject__c",
    ///     "label": "Updated Label"
    /// })];
    ///
    /// let results = client.update_metadata("CustomObject", &metadata).await?;
    /// ```
    pub async fn update_metadata(
        &self,
        metadata_type: &str,
        metadata_objects: &[serde_json::Value],
    ) -> Result<Vec<SaveResult>> {
        if metadata_objects.is_empty() {
            return Ok(Vec::new());
        }
        if metadata_objects.len() > 10 {
            return Err(Error::new(ErrorKind::Other(
                "Maximum 10 metadata components per call".to_string(),
            )));
        }

        let metadata_elements: Vec<String> = metadata_objects
            .iter()
            .map(|obj| self.build_metadata_element(metadata_type, obj))
            .collect();

        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:met="http://soap.sforce.com/2006/04/metadata" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <soapenv:Header>
    <met:SessionHeader>
      <met:sessionId>{session_id}</met:sessionId>
    </met:SessionHeader>
  </soapenv:Header>
  <soapenv:Body>
    <met:updateMetadata>
{metadata_elements}
    </met:updateMetadata>
  </soapenv:Body>
</soapenv:Envelope>"#,
            session_id = self.access_token,
            metadata_elements = metadata_elements.join("\n"),
        );

        let response = self
            .http_client
            .post(self.metadata_url())
            .headers(self.build_headers("updateMetadata"))
            .body(envelope)
            .send()
            .await?;

        let response_text = response.text().await?;

        if let Some(fault) = self.parse_soap_fault(&response_text) {
            return Err(Error::new(ErrorKind::SoapFault(fault.to_string())));
        }

        self.parse_save_results(&response_text)
    }

    /// Create or update metadata components (upsert operation).
    ///
    /// This is a synchronous CRUD operation that creates new components if they don't exist,
    /// or updates them if they already exist. Maximum 10 components per call.
    ///
    /// # Limitations
    ///
    /// - Does NOT support ApexClass or ApexTrigger (use deploy/retrieve for those)
    /// - Maximum 10 metadata components per call
    /// - Available since API version 30.0
    ///
    /// # Arguments
    ///
    /// * `metadata_type` - The type of metadata component (e.g., "CustomObject", "CustomField")
    /// * `metadata_objects` - Array of metadata objects as JSON values
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use serde_json::json;
    ///
    /// let metadata = vec![json!({
    ///     "fullName": "MyCustomObject__c",
    ///     "label": "My Custom Object"
    /// })];
    ///
    /// let results = client.upsert_metadata("CustomObject", &metadata).await?;
    /// for result in results {
    ///     if result.success {
    ///         if result.created {
    ///             println!("Created: {}", result.full_name);
    ///         } else {
    ///             println!("Updated: {}", result.full_name);
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn upsert_metadata(
        &self,
        metadata_type: &str,
        metadata_objects: &[serde_json::Value],
    ) -> Result<Vec<UpsertResult>> {
        if metadata_objects.is_empty() {
            return Ok(Vec::new());
        }
        if metadata_objects.len() > 10 {
            return Err(Error::new(ErrorKind::Other(
                "Maximum 10 metadata components per call".to_string(),
            )));
        }

        let metadata_elements: Vec<String> = metadata_objects
            .iter()
            .map(|obj| self.build_metadata_element(metadata_type, obj))
            .collect();

        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:met="http://soap.sforce.com/2006/04/metadata" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <soapenv:Header>
    <met:SessionHeader>
      <met:sessionId>{session_id}</met:sessionId>
    </met:SessionHeader>
  </soapenv:Header>
  <soapenv:Body>
    <met:upsertMetadata>
{metadata_elements}
    </met:upsertMetadata>
  </soapenv:Body>
</soapenv:Envelope>"#,
            session_id = self.access_token,
            metadata_elements = metadata_elements.join("\n"),
        );

        let response = self
            .http_client
            .post(self.metadata_url())
            .headers(self.build_headers("upsertMetadata"))
            .body(envelope)
            .send()
            .await?;

        let response_text = response.text().await?;

        if let Some(fault) = self.parse_soap_fault(&response_text) {
            return Err(Error::new(ErrorKind::SoapFault(fault.to_string())));
        }

        self.parse_upsert_results(&response_text)
    }

    /// Delete metadata components by type and full names.
    ///
    /// This is a synchronous CRUD operation that deletes metadata components.
    /// Maximum 10 components per call.
    ///
    /// # Limitations
    ///
    /// - Does NOT support ApexClass or ApexTrigger (use deploy/retrieve for those)
    /// - Maximum 10 metadata components per call
    /// - Available since API version 30.0
    ///
    /// # Arguments
    ///
    /// * `metadata_type` - The type of metadata component (e.g., "CustomObject")
    /// * `full_names` - Array of full names to delete
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let results = client.delete_metadata(
    ///     "CustomObject",
    ///     &["MyObject__c", "AnotherObject__c"]
    /// ).await?;
    ///
    /// for result in results {
    ///     if result.success {
    ///         println!("Deleted: {}", result.full_name);
    ///     } else {
    ///         println!("Failed to delete: {}", result.full_name);
    ///     }
    /// }
    /// ```
    pub async fn delete_metadata(
        &self,
        metadata_type: &str,
        full_names: &[&str],
    ) -> Result<Vec<DeleteResult>> {
        if full_names.is_empty() {
            return Ok(Vec::new());
        }
        if full_names.len() > 10 {
            return Err(Error::new(ErrorKind::Other(
                "Maximum 10 metadata components per call".to_string(),
            )));
        }

        let full_name_elements: String = full_names
            .iter()
            .map(|name| format!("      <met:fullNames>{}</met:fullNames>", xml::escape(name)))
            .collect::<Vec<_>>()
            .join("\n");

        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:met="http://soap.sforce.com/2006/04/metadata">
  <soapenv:Header>
    <met:SessionHeader>
      <met:sessionId>{session_id}</met:sessionId>
    </met:SessionHeader>
  </soapenv:Header>
  <soapenv:Body>
    <met:deleteMetadata>
      <met:type>{metadata_type}</met:type>
{full_name_elements}
    </met:deleteMetadata>
  </soapenv:Body>
</soapenv:Envelope>"#,
            session_id = self.access_token,
            metadata_type = xml::escape(metadata_type),
            full_name_elements = full_name_elements,
        );

        let response = self
            .http_client
            .post(self.metadata_url())
            .headers(self.build_headers("deleteMetadata"))
            .body(envelope)
            .send()
            .await?;

        let response_text = response.text().await?;

        if let Some(fault) = self.parse_soap_fault(&response_text) {
            return Err(Error::new(ErrorKind::SoapFault(fault.to_string())));
        }

        self.parse_delete_results(&response_text)
    }

    /// Rename a metadata component.
    ///
    /// This is a synchronous CRUD operation that renames a single metadata component.
    /// Only one component can be renamed per call.
    ///
    /// # Limitations
    ///
    /// - Does NOT support ApexClass or ApexTrigger (use deploy/retrieve for those)
    /// - Only one component can be renamed per call
    /// - Available since API version 30.0
    ///
    /// # Arguments
    ///
    /// * `metadata_type` - The type of metadata component (e.g., "CustomObject")
    /// * `old_full_name` - Current full name of the component
    /// * `new_full_name` - New full name for the component
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = client.rename_metadata(
    ///     "CustomObject",
    ///     "OldName__c",
    ///     "NewName__c"
    /// ).await?;
    ///
    /// if result.success {
    ///     println!("Renamed successfully");
    /// }
    /// ```
    pub async fn rename_metadata(
        &self,
        metadata_type: &str,
        old_full_name: &str,
        new_full_name: &str,
    ) -> Result<SaveResult> {
        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:met="http://soap.sforce.com/2006/04/metadata">
  <soapenv:Header>
    <met:SessionHeader>
      <met:sessionId>{session_id}</met:sessionId>
    </met:SessionHeader>
  </soapenv:Header>
  <soapenv:Body>
    <met:renameMetadata>
      <met:type>{metadata_type}</met:type>
      <met:oldFullName>{old_full_name}</met:oldFullName>
      <met:newFullName>{new_full_name}</met:newFullName>
    </met:renameMetadata>
  </soapenv:Body>
</soapenv:Envelope>"#,
            session_id = self.access_token,
            metadata_type = xml::escape(metadata_type),
            old_full_name = xml::escape(old_full_name),
            new_full_name = xml::escape(new_full_name),
        );

        let response = self
            .http_client
            .post(self.metadata_url())
            .headers(self.build_headers("renameMetadata"))
            .body(envelope)
            .send()
            .await?;

        let response_text = response.text().await?;

        if let Some(fault) = self.parse_soap_fault(&response_text) {
            return Err(Error::new(ErrorKind::SoapFault(fault.to_string())));
        }

        self.parse_rename_result(&response_text)
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

    /// Build a metadata element for SOAP body.
    fn build_metadata_element(
        &self,
        metadata_type: &str,
        metadata_obj: &serde_json::Value,
    ) -> String {
        let mut element = format!(
            "      <met:metadata xsi:type=\"met:{}\">\n",
            xml::escape(metadata_type)
        );

        if let Some(obj) = metadata_obj.as_object() {
            for (key, value) in obj {
                element.push_str(&self.build_xml_field(key, value, 8));
            }
        }

        element.push_str("      </met:metadata>");
        element
    }

    /// Build an XML field for a metadata object.
    fn build_xml_field(&self, key: &str, value: &serde_json::Value, indent: usize) -> String {
        let spaces = " ".repeat(indent);
        let escaped_key = xml::escape(key);

        match value {
            serde_json::Value::String(s) => {
                format!(
                    "{}<met:{}>{}</met:{}>\n",
                    spaces,
                    escaped_key,
                    xml::escape(s),
                    escaped_key
                )
            }
            serde_json::Value::Number(n) => {
                format!(
                    "{}<met:{}>{}</met:{}>\n",
                    spaces, escaped_key, n, escaped_key
                )
            }
            serde_json::Value::Bool(b) => {
                format!(
                    "{}<met:{}>{}</met:{}>\n",
                    spaces, escaped_key, b, escaped_key
                )
            }
            serde_json::Value::Null => {
                format!("{}<met:{} xsi:nil=\"true\"/>\n", spaces, escaped_key)
            }
            serde_json::Value::Object(obj) => {
                let mut result = format!("{}<met:{}>\n", spaces, escaped_key);
                for (nested_key, nested_value) in obj {
                    result.push_str(&self.build_xml_field(nested_key, nested_value, indent + 2));
                }
                result.push_str(&format!("{}</met:{}>\n", spaces, escaped_key));
                result
            }
            serde_json::Value::Array(arr) => {
                let mut result = String::new();
                for item in arr {
                    result.push_str(&self.build_xml_field(key, item, indent));
                }
                result
            }
        }
    }

    /// Parse SaveResult elements from SOAP response.
    fn parse_save_results(&self, xml: &str) -> Result<Vec<SaveResult>> {
        let mut results = Vec::new();
        let pattern = "<result";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</result>") {
                let block = &remaining[..end + "</result>".len()];

                let full_name = self.extract_element(block, "fullName").unwrap_or_default();
                let success = self
                    .extract_element(block, "success")
                    .map(|s| s == "true")
                    .unwrap_or(false);
                let errors = self.parse_metadata_errors(block);

                results.push(SaveResult {
                    full_name,
                    success,
                    errors,
                });

                search_from = &remaining[end + "</result>".len()..];
            } else {
                break;
            }
        }

        Ok(results)
    }

    /// Parse UpsertResult elements from SOAP response.
    fn parse_upsert_results(&self, xml: &str) -> Result<Vec<UpsertResult>> {
        let mut results = Vec::new();
        let pattern = "<result";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</result>") {
                let block = &remaining[..end + "</result>".len()];

                let full_name = self.extract_element(block, "fullName").unwrap_or_default();
                let success = self
                    .extract_element(block, "success")
                    .map(|s| s == "true")
                    .unwrap_or(false);
                let created = self
                    .extract_element(block, "created")
                    .map(|s| s == "true")
                    .unwrap_or(false);
                let errors = self.parse_metadata_errors(block);

                results.push(UpsertResult {
                    full_name,
                    success,
                    created,
                    errors,
                });

                search_from = &remaining[end + "</result>".len()..];
            } else {
                break;
            }
        }

        Ok(results)
    }

    /// Parse DeleteResult elements from SOAP response.
    fn parse_delete_results(&self, xml: &str) -> Result<Vec<DeleteResult>> {
        let mut results = Vec::new();
        let pattern = "<result";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</result>") {
                let block = &remaining[..end + "</result>".len()];

                let full_name = self.extract_element(block, "fullName").unwrap_or_default();
                let success = self
                    .extract_element(block, "success")
                    .map(|s| s == "true")
                    .unwrap_or(false);
                let errors = self.parse_metadata_errors(block);

                results.push(DeleteResult {
                    full_name,
                    success,
                    errors,
                });

                search_from = &remaining[end + "</result>".len()..];
            } else {
                break;
            }
        }

        Ok(results)
    }

    /// Parse ReadResult from SOAP response.
    fn parse_read_result(&self, xml: &str) -> Result<ReadResult> {
        let mut records = Vec::new();
        let pattern = "<result";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</result>") {
                let block = &remaining[..end + "</result>".len()];

                // Parse the metadata object as a JSON value for polymorphism
                let mut metadata_obj = serde_json::Map::new();

                // Extract fullName
                if let Some(full_name) = self.extract_element(block, "fullName") {
                    metadata_obj
                        .insert("fullName".to_string(), serde_json::Value::String(full_name));
                }

                // Extract other common fields
                if let Some(label) = self.extract_element(block, "label") {
                    metadata_obj.insert("label".to_string(), serde_json::Value::String(label));
                }

                // For now, we'll store the raw XML in a special field
                // This allows users to parse type-specific fields as needed
                metadata_obj.insert(
                    "_rawXml".to_string(),
                    serde_json::Value::String(block.to_string()),
                );

                records.push(serde_json::Value::Object(metadata_obj));

                search_from = &remaining[end + "</result>".len()..];
            } else {
                break;
            }
        }

        Ok(ReadResult { records })
    }

    /// Parse single SaveResult from rename operation.
    fn parse_rename_result(&self, xml: &str) -> Result<SaveResult> {
        let full_name = self.extract_element(xml, "fullName").unwrap_or_default();
        let success = self
            .extract_element(xml, "success")
            .map(|s| s == "true")
            .unwrap_or(false);
        let errors = self.parse_metadata_errors(xml);

        Ok(SaveResult {
            full_name,
            success,
            errors,
        })
    }

    /// Parse MetadataError elements from a result block.
    fn parse_metadata_errors(&self, xml: &str) -> Vec<MetadataError> {
        let mut errors = Vec::new();
        let pattern = "<errors>";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</errors>") {
                let block = &remaining[..end + "</errors>".len()];

                let status_code = self
                    .extract_element(block, "statusCode")
                    .unwrap_or_default();
                let message = self.extract_element(block, "message").unwrap_or_default();
                let fields = self.extract_elements(block, "fields");

                errors.push(MetadataError {
                    status_code,
                    message,
                    fields,
                });

                search_from = &remaining[end + "</errors>".len()..];
            } else {
                break;
            }
        }

        errors
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
    fn test_parse_save_results() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <createMetadataResponse>
                <result>
                    <fullName>MyObject__c</fullName>
                    <success>true</success>
                </result>
                <result>
                    <fullName>FailedObject__c</fullName>
                    <success>false</success>
                    <errors>
                        <statusCode>INVALID_FIELD</statusCode>
                        <message>Invalid field name</message>
                        <fields>Name</fields>
                    </errors>
                </result>
            </createMetadataResponse>
        "#;

        let results = client.parse_save_results(xml).unwrap();
        assert_eq!(results.len(), 2);

        // First result
        assert_eq!(results[0].full_name, "MyObject__c");
        assert!(results[0].success);
        assert_eq!(results[0].errors.len(), 0);

        // Second result with error
        assert_eq!(results[1].full_name, "FailedObject__c");
        assert!(!results[1].success);
        assert_eq!(results[1].errors.len(), 1);
        assert_eq!(results[1].errors[0].status_code, "INVALID_FIELD");
        assert_eq!(results[1].errors[0].message, "Invalid field name");
        assert_eq!(results[1].errors[0].fields, vec!["Name"]);
    }

    #[test]
    fn test_parse_upsert_results() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <upsertMetadataResponse>
                <result>
                    <fullName>CreatedObject__c</fullName>
                    <success>true</success>
                    <created>true</created>
                </result>
                <result>
                    <fullName>UpdatedObject__c</fullName>
                    <success>true</success>
                    <created>false</created>
                </result>
            </upsertMetadataResponse>
        "#;

        let results = client.parse_upsert_results(xml).unwrap();
        assert_eq!(results.len(), 2);

        // Created
        assert_eq!(results[0].full_name, "CreatedObject__c");
        assert!(results[0].success);
        assert!(results[0].created);

        // Updated
        assert_eq!(results[1].full_name, "UpdatedObject__c");
        assert!(results[1].success);
        assert!(!results[1].created);
    }

    #[test]
    fn test_parse_delete_results() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <deleteMetadataResponse>
                <result>
                    <fullName>DeletedObject__c</fullName>
                    <success>true</success>
                </result>
            </deleteMetadataResponse>
        "#;

        let results = client.parse_delete_results(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].full_name, "DeletedObject__c");
        assert!(results[0].success);
    }

    #[test]
    fn test_parse_read_result() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <readMetadataResponse>
                <result>
                    <fullName>Account</fullName>
                    <label>Account</label>
                </result>
                <result>
                    <fullName>Contact</fullName>
                    <label>Contact</label>
                </result>
            </readMetadataResponse>
        "#;

        let result = client.parse_read_result(xml).unwrap();
        assert_eq!(result.records.len(), 2);

        // Check first record
        if let Some(obj) = result.records[0].as_object() {
            assert_eq!(
                obj.get("fullName").and_then(|v| v.as_str()),
                Some("Account")
            );
            assert_eq!(obj.get("label").and_then(|v| v.as_str()), Some("Account"));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_parse_rename_result() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <renameMetadataResponse>
                <result>
                    <fullName>NewName__c</fullName>
                    <success>true</success>
                </result>
            </renameMetadataResponse>
        "#;

        let result = client.parse_rename_result(xml).unwrap();
        assert_eq!(result.full_name, "NewName__c");
        assert!(result.success);
    }

    #[test]
    fn test_parse_metadata_errors() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <result>
                <errors>
                    <statusCode>INVALID_FIELD</statusCode>
                    <message>Invalid field</message>
                    <fields>Name</fields>
                    <fields>Type</fields>
                </errors>
                <errors>
                    <statusCode>DUPLICATE_VALUE</statusCode>
                    <message>Duplicate value found</message>
                </errors>
            </result>
        "#;

        let errors = client.parse_metadata_errors(xml);
        assert_eq!(errors.len(), 2);

        // First error
        assert_eq!(errors[0].status_code, "INVALID_FIELD");
        assert_eq!(errors[0].message, "Invalid field");
        assert_eq!(errors[0].fields, vec!["Name", "Type"]);

        // Second error
        assert_eq!(errors[1].status_code, "DUPLICATE_VALUE");
        assert_eq!(errors[1].message, "Duplicate value found");
        assert_eq!(errors[1].fields.len(), 0);
    }

    #[test]
    fn test_build_metadata_element() {
        let client = MetadataClient::from_parts("url", "token");
        let metadata = serde_json::json!({
            "fullName": "MyObject__c",
            "label": "My Object",
            "pluralLabel": "My Objects"
        });

        let element = client.build_metadata_element("CustomObject", &metadata);
        assert!(element.contains("xsi:type=\"met:CustomObject\""));
        assert!(element.contains("<met:fullName>MyObject__c</met:fullName>"));
        assert!(element.contains("<met:label>My Object</met:label>"));
        assert!(element.contains("<met:pluralLabel>My Objects</met:pluralLabel>"));
    }

    #[test]
    fn test_build_metadata_element_with_escaping() {
        let client = MetadataClient::from_parts("url", "token");
        let metadata = serde_json::json!({
            "fullName": "Test<Object>",
            "label": "Test & Label"
        });

        let element = client.build_metadata_element("CustomObject", &metadata);
        assert!(element.contains("<met:fullName>Test&lt;Object&gt;</met:fullName>"));
        assert!(element.contains("<met:label>Test &amp; Label</met:label>"));
    }

    #[test]
    fn test_build_xml_field_string() {
        let client = MetadataClient::from_parts("url", "token");
        let value = serde_json::Value::String("test value".to_string());
        let field = client.build_xml_field("name", &value, 4);
        assert_eq!(field, "    <met:name>test value</met:name>\n");
    }

    #[test]
    fn test_build_xml_field_number() {
        let client = MetadataClient::from_parts("url", "token");
        let value = serde_json::Value::Number(42.into());
        let field = client.build_xml_field("count", &value, 4);
        assert_eq!(field, "    <met:count>42</met:count>\n");
    }

    #[test]
    fn test_build_xml_field_bool() {
        let client = MetadataClient::from_parts("url", "token");
        let value = serde_json::Value::Bool(true);
        let field = client.build_xml_field("enabled", &value, 4);
        assert_eq!(field, "    <met:enabled>true</met:enabled>\n");
    }

    #[test]
    fn test_build_xml_field_null() {
        let client = MetadataClient::from_parts("url", "token");
        let value = serde_json::Value::Null;
        let field = client.build_xml_field("optional", &value, 4);
        assert_eq!(field, "    <met:optional xsi:nil=\"true\"/>\n");
    }

    #[test]
    fn test_build_xml_field_nested() {
        let client = MetadataClient::from_parts("url", "token");
        let value = serde_json::json!({
            "inner": "value"
        });
        let field = client.build_xml_field("outer", &value, 4);
        assert!(field.contains("    <met:outer>\n"));
        assert!(field.contains("      <met:inner>value</met:inner>\n"));
        assert!(field.contains("    </met:outer>\n"));
    }
}
