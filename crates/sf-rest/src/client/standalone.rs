use tracing::instrument;

use busbar_sf_client::security::{soql, url as url_security};

use crate::error::{Error, ErrorKind, Result};

impl super::SalesforceRestClient {
    /// Get all tabs available to the current user.
    ///
    /// Returns information about all tabs, including custom tabs.
    #[instrument(skip(self))]
    pub async fn tabs(&self) -> Result<Vec<serde_json::Value>> {
        self.client.rest_get("tabs").await.map_err(Into::into)
    }

    /// Get the current user's theme information.
    ///
    /// Returns theme colors, icons, and other UI customization data.
    #[instrument(skip(self))]
    pub async fn theme(&self) -> Result<serde_json::Value> {
        self.client.rest_get("theme").await.map_err(Into::into)
    }

    /// Get the app menu for a specific menu type.
    ///
    /// # Arguments
    /// * `app_menu_type` - One of: "AppSwitcher", "Salesforce1", "NetworkTabs"
    #[instrument(skip(self))]
    pub async fn app_menu(&self, app_menu_type: &str) -> Result<serde_json::Value> {
        let valid_types = ["AppSwitcher", "Salesforce1", "NetworkTabs"];
        if !valid_types.contains(&app_menu_type) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_PARAMETER".to_string(),
                message: format!(
                    "Invalid app menu type '{}'. Must be one of: AppSwitcher, Salesforce1, NetworkTabs",
                    app_menu_type
                ),
            }));
        }
        let path = format!("appMenu/{}", app_menu_type);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get recently viewed items for the current user.
    ///
    /// Returns a list of recently accessed records.
    #[instrument(skip(self))]
    pub async fn recent_items(&self) -> Result<Vec<serde_json::Value>> {
        self.client.rest_get("recent").await.map_err(Into::into)
    }

    /// Get relevant items for the current user.
    ///
    /// Returns items that Salesforce considers relevant based on the user's activity.
    #[instrument(skip(self))]
    pub async fn relevant_items(&self) -> Result<serde_json::Value> {
        self.client
            .rest_get("sobjects/relevantItems")
            .await
            .map_err(Into::into)
    }

    /// Get compact layouts for multiple SObject types.
    ///
    /// # Arguments
    /// * `sobject_list` - Comma-separated list of SObject API names (e.g., "Account,Contact")
    #[instrument(skip(self))]
    pub async fn compact_layouts(&self, sobject_list: &str) -> Result<serde_json::Value> {
        // Validate each SObject name in the comma-separated list
        for sobject in sobject_list.split(',') {
            let trimmed = sobject.trim();
            if !soql::is_safe_sobject_name(trimmed) {
                return Err(Error::new(ErrorKind::Salesforce {
                    error_code: "INVALID_SOBJECT".to_string(),
                    message: format!("Invalid SObject name: {}", trimmed),
                }));
            }
        }
        let encoded = url_security::encode_param(sobject_list);
        let path = format!("compactLayouts?q={}", encoded);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get the event schema for a platform event.
    ///
    /// # Arguments
    /// * `event_name` - The platform event API name (e.g., "MyEvent__e")
    #[instrument(skip(self))]
    pub async fn platform_event_schema(&self, event_name: &str) -> Result<serde_json::Value> {
        if !soql::is_safe_sobject_name(event_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_EVENT_NAME".to_string(),
                message: "Invalid platform event name".to_string(),
            }));
        }
        let path = format!("event/eventSchema/{}", event_name);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get Lightning Experience toggle metrics.
    ///
    /// Returns metrics about Lightning Experience vs Classic usage.
    #[instrument(skip(self))]
    pub async fn lightning_toggle_metrics(&self) -> Result<serde_json::Value> {
        self.client
            .rest_get("lightning/toggleMetrics")
            .await
            .map_err(Into::into)
    }

    /// Get Lightning Experience usage data.
    ///
    /// Returns Lightning Experience usage statistics.
    #[instrument(skip(self))]
    pub async fn lightning_usage(&self) -> Result<serde_json::Value> {
        self.client
            .rest_get("lightning/usage")
            .await
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::super::SalesforceRestClient;

    #[tokio::test]
    async fn test_tabs_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!([
            {"label": "Accounts", "name": "standard-Account", "url": "/001/o"}
        ]);

        Mock::given(method("GET"))
            .and(path_regex(".*/tabs$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client.tabs().await.expect("tabs should succeed");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["name"], "standard-Account");
    }

    #[tokio::test]
    async fn test_theme_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "themeItems": [{"name": "Account", "colors": []}]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/theme$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client.theme().await.expect("theme should succeed");
        assert!(result["themeItems"].is_array());
    }

    #[tokio::test]
    async fn test_app_menu_valid_type() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "appMenuItems": []
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/appMenu/AppSwitcher$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .app_menu("AppSwitcher")
            .await
            .expect("app_menu should succeed");
        assert!(result["appMenuItems"].is_array());
    }

    #[tokio::test]
    async fn test_app_menu_invalid_type() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.app_menu("InvalidType").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("INVALID_PARAMETER"));
    }

    #[tokio::test]
    async fn test_recent_items_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!([
            {"Id": "001xx000003Dgb2AAC", "Name": "Acme Corp"}
        ]);

        Mock::given(method("GET"))
            .and(path_regex(".*/recent$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .recent_items()
            .await
            .expect("recent_items should succeed");
        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn test_relevant_items_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "relevantItems": []
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/relevantItems$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .relevant_items()
            .await
            .expect("relevant_items should succeed");
        assert!(result["relevantItems"].is_array());
    }

    #[tokio::test]
    async fn test_compact_layouts_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "Account": {"compactLayouts": []}
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/compactLayouts.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .compact_layouts("Account")
            .await
            .expect("compact_layouts should succeed");
        assert!(result["Account"].is_object());
    }

    #[tokio::test]
    async fn test_compact_layouts_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.compact_layouts("Bad'; DROP--").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_platform_event_schema_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "name": "MyEvent__e",
            "fields": []
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/event/eventSchema/MyEvent__e$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .platform_event_schema("MyEvent__e")
            .await
            .expect("platform_event_schema should succeed");
        assert_eq!(result["name"], "MyEvent__e");
    }

    #[tokio::test]
    async fn test_platform_event_schema_invalid_name() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.platform_event_schema("Bad'; DROP--").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("INVALID_EVENT_NAME"));
    }

    #[tokio::test]
    async fn test_lightning_toggle_metrics_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "metricsData": []
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/lightning/toggleMetrics$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .lightning_toggle_metrics()
            .await
            .expect("lightning_toggle_metrics should succeed");
        assert!(result["metricsData"].is_array());
    }

    #[tokio::test]
    async fn test_lightning_usage_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "usageData": []
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/lightning/usage$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .lightning_usage()
            .await
            .expect("lightning_usage should succeed");
        assert!(result["usageData"].is_array());
    }
}
