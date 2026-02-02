use busbar_sf_client::security::xml;

use crate::error::{Error, ErrorKind, Result};
use crate::list::MetadataComponent;

impl super::MetadataClient {
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
}
