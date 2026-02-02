//! Composite API operations.

use serde::{Deserialize, Serialize};

/// A composite request containing multiple subrequests.
#[derive(Debug, Clone, Serialize)]
pub struct CompositeRequest {
    #[serde(rename = "allOrNone")]
    pub all_or_none: bool,
    #[serde(rename = "collateSubrequests")]
    pub collate_subrequests: bool,
    #[serde(rename = "compositeRequest")]
    pub subrequests: Vec<CompositeSubrequest>,
}

/// A single subrequest within a composite request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeSubrequest {
    pub method: String,
    pub url: String,
    #[serde(rename = "referenceId")]
    pub reference_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

/// Response from a composite request.
#[derive(Debug, Clone, Deserialize)]
pub struct CompositeResponse {
    #[serde(rename = "compositeResponse")]
    pub responses: Vec<CompositeSubresponse>,
}

/// Response from a single subrequest.
#[derive(Debug, Clone, Deserialize)]
pub struct CompositeSubresponse {
    pub body: serde_json::Value,
    #[serde(rename = "httpHeaders")]
    pub http_headers: serde_json::Value,
    #[serde(rename = "httpStatusCode")]
    pub http_status_code: u16,
    #[serde(rename = "referenceId")]
    pub reference_id: String,
}

/// A composite batch request containing multiple independent subrequests.
///
/// Unlike the standard composite request, batch subrequests are executed independently
/// and cannot reference each other's results. Available since API v34.0.
#[derive(Debug, Clone, Serialize)]
pub struct CompositeBatchRequest {
    #[serde(rename = "batchRequests")]
    pub batch_requests: Vec<CompositeBatchSubrequest>,
    #[serde(rename = "haltOnError")]
    pub halt_on_error: bool,
}

/// A single subrequest within a composite batch request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeBatchSubrequest {
    pub method: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "richInput")]
    pub rich_input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "binaryPartName")]
    pub binary_part_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "binaryPartNameAlias")]
    pub binary_part_name_alias: Option<String>,
}

/// Response from a composite batch request.
#[derive(Debug, Clone, Deserialize)]
pub struct CompositeBatchResponse {
    #[serde(rename = "hasErrors")]
    pub has_errors: bool,
    pub results: Vec<CompositeBatchSubresponse>,
}

/// Response from a single batch subrequest.
#[derive(Debug, Clone, Deserialize)]
pub struct CompositeBatchSubresponse {
    #[serde(rename = "statusCode")]
    pub status_code: u16,
    pub result: serde_json::Value,
}

/// A composite tree request for creating record hierarchies.
///
/// Allows creation of parent records with nested child records in a single request.
/// Available since API v42.0.
#[derive(Debug, Clone, Serialize)]
pub struct CompositeTreeRequest {
    pub records: Vec<CompositeTreeRecord>,
}

/// A record in a composite tree request with optional nested child records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeTreeRecord {
    pub attributes: CompositeTreeAttributes,
    #[serde(rename = "referenceId")]
    pub reference_id: String,
    #[serde(flatten)]
    pub fields: serde_json::Map<String, serde_json::Value>,
}

/// Attributes for a record in a composite tree request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeTreeAttributes {
    #[serde(rename = "type")]
    pub sobject_type: String,
}

/// Response from a composite tree request.
#[derive(Debug, Clone, Deserialize)]
pub struct CompositeTreeResponse {
    #[serde(rename = "hasErrors")]
    pub has_errors: bool,
    pub results: Vec<CompositeTreeResult>,
}

/// Result of a single record creation in a composite tree request.
#[derive(Debug, Clone, Deserialize)]
pub struct CompositeTreeResult {
    #[serde(rename = "referenceId")]
    pub reference_id: String,
    pub id: Option<String>,
    #[serde(default)]
    pub errors: Vec<CompositeTreeError>,
}

/// Error details for a failed record creation in a composite tree request.
#[derive(Debug, Clone, Deserialize)]
pub struct CompositeTreeError {
    #[serde(rename = "statusCode")]
    pub status_code: String,
    pub message: String,
    pub fields: Vec<String>,
}

/// A composite graph request for executing multiple dependent operations.
///
/// Allows multiple independent graphs that each contain composite subrequests.
/// Available since API v50.0.
#[derive(Debug, Clone, Serialize)]
pub struct CompositeGraphRequest {
    pub graphs: Vec<GraphRequest>,
}

/// A single graph within a composite graph request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRequest {
    pub graph_id: String,
    pub composite_request: Vec<CompositeSubrequest>,
}

/// Response from a composite graph request.
#[derive(Debug, Clone, Deserialize)]
pub struct CompositeGraphResponse {
    pub graphs: Vec<GraphResponse>,
}

/// Response from a single graph.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphResponse {
    pub graph_id: String,
    pub graph_response: GraphResponseBody,
    pub is_successful: bool,
}

/// Body of a graph response containing the composite responses.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphResponseBody {
    #[serde(rename = "compositeResponse")]
    pub responses: Vec<CompositeSubresponse>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_composite_request_serialization() {
        let request = CompositeRequest {
            all_or_none: true,
            collate_subrequests: false,
            subrequests: vec![
                CompositeSubrequest {
                    method: "POST".to_string(),
                    url: "/services/data/v62.0/sobjects/Account".to_string(),
                    reference_id: "NewAccount".to_string(),
                    body: Some(json!({"Name": "Test Corp"})),
                },
                CompositeSubrequest {
                    method: "GET".to_string(),
                    url: "/services/data/v62.0/sobjects/Account/@{NewAccount.id}".to_string(),
                    reference_id: "GetAccount".to_string(),
                    body: None,
                },
            ],
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["allOrNone"], true);
        assert_eq!(json["collateSubrequests"], false);
        assert_eq!(json["compositeRequest"].as_array().unwrap().len(), 2);

        let first = &json["compositeRequest"][0];
        assert_eq!(first["method"], "POST");
        assert_eq!(first["referenceId"], "NewAccount");
        assert!(first["body"].is_object());

        // GET subrequest should omit null body
        let second = &json["compositeRequest"][1];
        assert_eq!(second["method"], "GET");
        assert!(second.get("body").is_none());
    }

    #[test]
    fn test_composite_response_deserialization() {
        let json = json!({
            "compositeResponse": [
                {
                    "body": {"id": "001xx000003Dgb2AAC", "success": true, "errors": []},
                    "httpHeaders": {"Location": "/services/data/v62.0/sobjects/Account/001xx"},
                    "httpStatusCode": 201,
                    "referenceId": "NewAccount"
                },
                {
                    "body": {"Id": "001xx000003Dgb2AAC", "Name": "Test Corp"},
                    "httpHeaders": {},
                    "httpStatusCode": 200,
                    "referenceId": "GetAccount"
                }
            ]
        });

        let response: CompositeResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.responses.len(), 2);
        assert_eq!(response.responses[0].http_status_code, 201);
        assert_eq!(response.responses[0].reference_id, "NewAccount");
        assert_eq!(response.responses[1].http_status_code, 200);
    }

    #[test]
    fn test_composite_batch_request_serialization() {
        let request = CompositeBatchRequest {
            batch_requests: vec![CompositeBatchSubrequest {
                method: "GET".to_string(),
                url: "/services/data/v62.0/sobjects/Account/001xx".to_string(),
                rich_input: None,
                binary_part_name: None,
                binary_part_name_alias: None,
            }],
            halt_on_error: true,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["haltOnError"], true);
        assert_eq!(json["batchRequests"].as_array().unwrap().len(), 1);
        // Optional fields should be omitted
        assert!(json["batchRequests"][0].get("richInput").is_none());
    }

    #[test]
    fn test_composite_batch_response_deserialization() {
        let json = json!({
            "hasErrors": true,
            "results": [
                {"statusCode": 200, "result": {"Id": "001xx", "Name": "Acme"}},
                {"statusCode": 404, "result": [{"errorCode": "NOT_FOUND", "message": "not found"}]}
            ]
        });

        let response: CompositeBatchResponse = serde_json::from_value(json).unwrap();
        assert!(response.has_errors);
        assert_eq!(response.results.len(), 2);
        assert_eq!(response.results[0].status_code, 200);
        assert_eq!(response.results[1].status_code, 404);
    }

    #[test]
    fn test_composite_tree_request_serialization() {
        let mut fields = serde_json::Map::new();
        fields.insert("Name".to_string(), json!("Parent Account"));

        let request = CompositeTreeRequest {
            records: vec![CompositeTreeRecord {
                attributes: CompositeTreeAttributes {
                    sobject_type: "Account".to_string(),
                },
                reference_id: "ref1".to_string(),
                fields,
            }],
        };

        let json = serde_json::to_value(&request).unwrap();
        let record = &json["records"][0];
        assert_eq!(record["attributes"]["type"], "Account");
        assert_eq!(record["referenceId"], "ref1");
        assert_eq!(record["Name"], "Parent Account");
    }

    #[test]
    fn test_composite_tree_response_with_errors() {
        let json = json!({
            "hasErrors": true,
            "results": [
                {
                    "referenceId": "ref1",
                    "id": null,
                    "errors": [
                        {
                            "statusCode": "REQUIRED_FIELD_MISSING",
                            "message": "Required fields are missing: [Name]",
                            "fields": ["Name"]
                        }
                    ]
                }
            ]
        });

        let response: CompositeTreeResponse = serde_json::from_value(json).unwrap();
        assert!(response.has_errors);
        assert!(response.results[0].id.is_none());
        assert_eq!(response.results[0].errors.len(), 1);
        assert_eq!(
            response.results[0].errors[0].status_code,
            "REQUIRED_FIELD_MISSING"
        );
        assert_eq!(response.results[0].errors[0].fields, vec!["Name"]);
    }

    #[test]
    fn test_composite_tree_response_success() {
        let json = json!({
            "hasErrors": false,
            "results": [
                {"referenceId": "ref1", "id": "001xx000003Dgb2AAC", "errors": []}
            ]
        });

        let response: CompositeTreeResponse = serde_json::from_value(json).unwrap();
        assert!(!response.has_errors);
        assert_eq!(
            response.results[0].id,
            Some("001xx000003Dgb2AAC".to_string())
        );
        assert!(response.results[0].errors.is_empty());
    }

    #[test]
    fn test_composite_graph_request_serialization() {
        let request = CompositeGraphRequest {
            graphs: vec![GraphRequest {
                graph_id: "graph1".to_string(),
                composite_request: vec![CompositeSubrequest {
                    method: "POST".to_string(),
                    url: "/services/data/v62.0/sobjects/Account".to_string(),
                    reference_id: "Account1".to_string(),
                    body: Some(json!({"Name": "Test"})),
                }],
            }],
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["graphs"][0]["graphId"], "graph1");
        assert_eq!(json["graphs"][0]["compositeRequest"][0]["method"], "POST");
    }

    #[test]
    fn test_composite_graph_response_deserialization() {
        let json = json!({
            "graphs": [
                {
                    "graphId": "graph1",
                    "graphResponse": {
                        "compositeResponse": [
                            {
                                "body": {"id": "001xx1", "success": true, "errors": []},
                                "httpHeaders": {},
                                "httpStatusCode": 201,
                                "referenceId": "Account1"
                            }
                        ]
                    },
                    "isSuccessful": true
                }
            ]
        });

        let response: CompositeGraphResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.graphs.len(), 1);
        assert!(response.graphs[0].is_successful);
        assert_eq!(response.graphs[0].graph_id, "graph1");
        assert_eq!(response.graphs[0].graph_response.responses.len(), 1);
        assert_eq!(
            response.graphs[0].graph_response.responses[0].http_status_code,
            201
        );
    }
}
