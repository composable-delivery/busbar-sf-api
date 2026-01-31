use crate::describe::DescribeMetadataResult;
use crate::error::{Error, ErrorKind, Result};

impl super::MetadataClient {
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
}
