//! Binary/Blob data retrieval host function handlers.
use super::error::*;
use base64::{engine::general_purpose, Engine as _};
use busbar_sf_rest::SalesforceRestClient;
use busbar_sf_wasm_types::*;

pub async fn handle_get_blob(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: GetBlobRequest,
) -> BridgeResult<GetBlobResponse> {
    match rest.get_blob(&req.sobject, &req.id, &req.field).await {
        Ok(bytes) => {
            let encoded = general_purpose::STANDARD.encode(&bytes);
            BridgeResult::ok(GetBlobResponse {
                data_base64: encoded,
            })
        }
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_get_rich_text_image(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: GetRichTextImageRequest,
) -> BridgeResult<GetRichTextImageResponse> {
    match rest
        .get_rich_text_image(&req.sobject, &req.id, &req.field, &req.content_reference_id)
        .await
    {
        Ok(bytes) => {
            let encoded = general_purpose::STANDARD.encode(&bytes);
            BridgeResult::ok(GetRichTextImageResponse {
                data_base64: encoded,
            })
        }
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_get_relationship(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: GetRelationshipRequest,
) -> BridgeResult<serde_json::Value> {
    match rest
        .get_relationship::<serde_json::Value>(&req.sobject, &req.id, &req.relationship_name)
        .await
    {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}
