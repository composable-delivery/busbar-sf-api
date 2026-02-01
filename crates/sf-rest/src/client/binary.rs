use serde::de::DeserializeOwned;
use tracing::instrument;

use busbar_sf_client::security::{soql, url as url_security};

use crate::error::{Error, ErrorKind, Result};

impl super::SalesforceRestClient {
    /// Get binary blob content from an SObject field (e.g., Attachment body, Document body).
    #[instrument(skip(self))]
    pub async fn get_blob(&self, sobject: &str, id: &str, blob_field: &str) -> Result<Vec<u8>> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !url_security::is_valid_salesforce_id(id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        if !soql::is_safe_field_name(blob_field) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_FIELD".to_string(),
                message: "Invalid field name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/{}/{}", sobject, id, blob_field);
        let url = self.client.rest_url(&path);
        let request = self.client.get(&url);
        let response = self.client.execute(request).await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Get a rich text image from an SObject field.
    #[instrument(skip(self))]
    pub async fn get_rich_text_image(
        &self,
        sobject: &str,
        id: &str,
        field_name: &str,
        content_reference_id: &str,
    ) -> Result<Vec<u8>> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !url_security::is_valid_salesforce_id(id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        if !soql::is_safe_field_name(field_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_FIELD".to_string(),
                message: "Invalid field name".to_string(),
            }));
        }
        if !url_security::is_valid_salesforce_id(content_reference_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid content reference ID format".to_string(),
            }));
        }
        let path = format!(
            "sobjects/{}/{}/richTextImageFields/{}/{}",
            sobject, id, field_name, content_reference_id
        );
        let url = self.client.rest_url(&path);
        let request = self.client.get(&url);
        let response = self.client.execute(request).await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Get related records via a relationship field.
    #[instrument(skip(self))]
    pub async fn get_relationship<T: DeserializeOwned>(
        &self,
        sobject: &str,
        id: &str,
        relationship_name: &str,
    ) -> Result<T> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !url_security::is_valid_salesforce_id(id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        if !soql::is_safe_field_name(relationship_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_FIELD".to_string(),
                message: "Invalid relationship name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/{}/{}", sobject, id, relationship_name);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get basic info about an SObject type (describe + recent items).
    #[instrument(skip(self))]
    pub async fn get_sobject_basic_info(&self, sobject: &str) -> Result<super::SObjectInfo> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}", sobject);
        self.client.rest_get(&path).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::super::SalesforceRestClient;

    #[tokio::test]
    async fn test_get_blob_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .get_blob("Bad'; DROP--", "001xx000003Dgb2AAC", "Body")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_get_blob_invalid_id() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.get_blob("Attachment", "bad-id", "Body").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_ID"));
    }

    #[tokio::test]
    async fn test_get_blob_invalid_field() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .get_blob("Attachment", "001xx000003Dgb2AAC", "Bad'; DROP--")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_FIELD"));
    }

    #[tokio::test]
    async fn test_get_rich_text_image_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .get_rich_text_image(
                "Bad'; DROP--",
                "001xx000003Dgb2AAC",
                "RichText__c",
                "0P0xx000000001XABC",
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_get_relationship_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .get_relationship::<serde_json::Value>("Bad'; DROP--", "001xx000003Dgb2AAC", "Contacts")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_get_relationship_invalid_id() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .get_relationship::<serde_json::Value>("Account", "bad-id", "Contacts")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_ID"));
    }

    #[tokio::test]
    async fn test_get_relationship_invalid_name() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .get_relationship::<serde_json::Value>("Account", "001xx000003Dgb2AAC", "Bad'; DROP--")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_FIELD"));
    }

    #[tokio::test]
    async fn test_get_sobject_basic_info_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.get_sobject_basic_info("Bad'; DROP--").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_get_blob_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let binary_content = vec![0x89, 0x50, 0x4E, 0x47]; // PNG magic bytes

        Mock::given(method("GET"))
            .and(path_regex(
                ".*/sobjects/Attachment/001xx000003Dgb2AAC/Body$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(binary_content.clone()))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .get_blob("Attachment", "001xx000003Dgb2AAC", "Body")
            .await
            .expect("get_blob should succeed");
        assert_eq!(result, binary_content);
    }

    #[tokio::test]
    async fn test_get_relationship_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "totalSize": 1,
            "done": true,
            "records": [{"Id": "003xx000001Svf0AAC", "Name": "John Doe"}]
        });

        Mock::given(method("GET"))
            .and(path_regex(
                ".*/sobjects/Account/001xx000003Dgb2AAC/Contacts$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result: serde_json::Value = client
            .get_relationship("Account", "001xx000003Dgb2AAC", "Contacts")
            .await
            .expect("get_relationship should succeed");
        assert_eq!(result["totalSize"], 1);
    }

    #[tokio::test]
    async fn test_get_sobject_basic_info_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "objectDescribe": {
                "name": "Account",
                "label": "Account",
                "keyPrefix": "001",
                "urls": {"sobject": "/services/data/v62.0/sobjects/Account"},
                "custom": false,
                "createable": true,
                "updateable": true,
                "deletable": true,
                "queryable": true,
                "searchable": true
            },
            "recentItems": []
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .get_sobject_basic_info("Account")
            .await
            .expect("get_sobject_basic_info should succeed");
        assert_eq!(result.object_describe.name, "Account");
        assert!(result.object_describe.createable);
    }
}
