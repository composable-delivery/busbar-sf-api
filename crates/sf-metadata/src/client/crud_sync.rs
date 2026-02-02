use busbar_sf_client::security::xml;

use crate::error::{Error, ErrorKind, Result};
use crate::types::{DeleteResult, ReadResult, SaveResult, UpsertResult};

impl super::MetadataClient {
    /// Create one or more metadata components.
    ///
    /// Synchronous CRUD operation. Maximum 10 components per call.
    /// Does NOT support ApexClass or ApexTrigger (use deploy/retrieve).
    ///
    /// Available since API version 30.0.
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
    /// Synchronous operation. Does NOT support ApexClass or ApexTrigger.
    ///
    /// Available since API version 30.0.
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
    /// Synchronous CRUD operation. Maximum 10 components per call.
    /// Does NOT support ApexClass or ApexTrigger.
    ///
    /// Available since API version 30.0.
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
    /// Synchronous CRUD operation. Maximum 10 components per call.
    /// Does NOT support ApexClass or ApexTrigger.
    ///
    /// Available since API version 30.0.
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
    /// Synchronous CRUD operation. Maximum 10 components per call.
    /// Does NOT support ApexClass or ApexTrigger.
    ///
    /// Available since API version 30.0.
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
    /// Synchronous CRUD operation. Only one component per call.
    /// Does NOT support ApexClass or ApexTrigger.
    ///
    /// Available since API version 30.0.
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
}

#[cfg(test)]
mod tests {
    use super::super::MetadataClient;

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
        let value = serde_json::Value::String("test value".to_string());
        let field = MetadataClient::build_xml_field("name", &value, 4);
        assert_eq!(field, "    <met:name>test value</met:name>\n");
    }

    #[test]
    fn test_build_xml_field_number() {
        let value = serde_json::Value::Number(42.into());
        let field = MetadataClient::build_xml_field("count", &value, 4);
        assert_eq!(field, "    <met:count>42</met:count>\n");
    }

    #[test]
    fn test_build_xml_field_bool() {
        let value = serde_json::Value::Bool(true);
        let field = MetadataClient::build_xml_field("enabled", &value, 4);
        assert_eq!(field, "    <met:enabled>true</met:enabled>\n");
    }

    #[test]
    fn test_build_xml_field_null() {
        let value = serde_json::Value::Null;
        let field = MetadataClient::build_xml_field("optional", &value, 4);
        assert_eq!(field, "    <met:optional xsi:nil=\"true\"/>\n");
    }

    #[test]
    fn test_build_xml_field_nested() {
        let value = serde_json::json!({
            "inner": "value"
        });
        let field = MetadataClient::build_xml_field("outer", &value, 4);
        assert!(field.contains("    <met:outer>\n"));
        assert!(field.contains("      <met:inner>value</met:inner>\n"));
        assert!(field.contains("    </met:outer>\n"));
    }
}
