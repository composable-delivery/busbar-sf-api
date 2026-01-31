use crate::deploy::{DeployOptions, DeployResult};
use crate::error::{Error, ErrorKind, Result};
use crate::types::TestLevel;
use base64::{engine::general_purpose, Engine as _};
use busbar_sf_client::security::xml;
use std::time::Duration;
use tokio::time::sleep;

impl super::MetadataClient {
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

    /// Cancel an in-progress deployment.
    ///
    /// Requests cancellation of a deployment identified by its async process ID.
    /// Note that cancellation is asynchronous — this method returns immediately,
    /// but you must call `check_deploy_status()` to see when the deployment
    /// actually reaches `Canceled` or `Canceling` status.
    ///
    /// Available since API v30.0.
    pub async fn cancel_deploy(
        &self,
        async_process_id: &str,
    ) -> Result<crate::deploy::CancelDeployResult> {
        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <soap:Header>
    <SessionHeader xmlns="http://soap.sforce.com/2006/04/metadata">
      <sessionId>{session_id}</sessionId>
    </SessionHeader>
  </soap:Header>
  <soap:Body>
    <cancelDeploy xmlns="http://soap.sforce.com/2006/04/metadata">
      <String>{process_id}</String>
    </cancelDeploy>
  </soap:Body>
</soap:Envelope>"#,
            session_id = self.access_token,
            process_id = xml::escape(async_process_id),
        );

        let response = self
            .http_client
            .post(self.metadata_url())
            .headers(self.build_headers("cancelDeploy"))
            .body(envelope)
            .send()
            .await?;

        let response_text = response.text().await?;

        if let Some(fault) = self.parse_soap_fault(&response_text) {
            return Err(Error::new(ErrorKind::SoapFault(fault.to_string())));
        }

        self.parse_cancel_deploy_result(&response_text)
    }

    /// Quick-deploy a recently validated deployment without re-running Apex tests.
    ///
    /// First deploy with `checkOnly=true` to validate and run tests. If validation
    /// succeeds, call this method with the validation deploy ID to quick-deploy
    /// without re-running tests.
    ///
    /// Available since API v33.0.
    pub async fn deploy_recent_validation(&self, validation_id: &str) -> Result<String> {
        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <soap:Header>
    <SessionHeader xmlns="http://soap.sforce.com/2006/04/metadata">
      <sessionId>{session_id}</sessionId>
    </SessionHeader>
  </soap:Header>
  <soap:Body>
    <deployRecentValidation xmlns="http://soap.sforce.com/2006/04/metadata">
      <validationId>{validation_id}</validationId>
    </deployRecentValidation>
  </soap:Body>
</soap:Envelope>"#,
            session_id = self.access_token,
            validation_id = xml::escape(validation_id),
        );

        let response = self
            .http_client
            .post(self.metadata_url())
            .headers(self.build_headers("deployRecentValidation"))
            .body(envelope)
            .send()
            .await?;

        let response_text = response.text().await?;

        if let Some(fault) = self.parse_soap_fault(&response_text) {
            return Err(Error::new(ErrorKind::SoapFault(fault.to_string())));
        }

        self.extract_element(&response_text, "id").ok_or_else(|| {
            Error::new(ErrorKind::InvalidResponse(
                "No async process ID in deployRecentValidation response".to_string(),
            ))
        })
    }
}
