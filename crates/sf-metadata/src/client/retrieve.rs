use busbar_sf_client::security::xml;
use std::time::Duration;
use tokio::time::sleep;

use crate::error::{Error, ErrorKind, Result};
use crate::retrieve::{PackageManifest, RetrieveResult};

impl super::MetadataClient {
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
}
