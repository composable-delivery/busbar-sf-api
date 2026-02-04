//! Metadata API host function handlers.
use super::error::*;
use base64::{engine::general_purpose, Engine as _};
use busbar_sf_metadata::MetadataClient;
use busbar_sf_wasm_types::*;

/// Deploy a metadata package.
pub(crate) async fn handle_metadata_deploy(
    client: &MetadataClient,
    request: MetadataDeployRequest,
) -> BridgeResult<MetadataDeployResponse> {
    let zip_bytes = match general_purpose::STANDARD.decode(&request.zip_base64) {
        Ok(b) => b,
        Err(e) => return BridgeResult::err("INVALID_REQUEST", format!("invalid base64: {e}")),
    };

    let test_level = match &request.options.test_level {
        Some(tl) => match parse_test_level(tl) {
            Ok(level) => Some(level),
            Err(msg) => return BridgeResult::err("INVALID_REQUEST", msg),
        },
        None => None,
    };

    let options = busbar_sf_metadata::DeployOptions {
        check_only: request.options.check_only,
        rollback_on_error: request.options.rollback_on_error,
        test_level,
        run_tests: request.options.run_tests,
        ..Default::default()
    };

    match client.deploy(&zip_bytes, options).await {
        Ok(async_process_id) => BridgeResult::ok(MetadataDeployResponse { async_process_id }),
        Err(e) => {
            let (code, message) = sanitize_metadata_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Check the status of a metadata deployment.
pub(crate) async fn handle_metadata_check_deploy_status(
    client: &MetadataClient,
    request: MetadataCheckDeployStatusRequest,
) -> BridgeResult<MetadataDeployResult> {
    match client
        .check_deploy_status(&request.async_process_id, request.include_details)
        .await
    {
        Ok(result) => BridgeResult::ok(MetadataDeployResult {
            id: result.id,
            done: result.done,
            status: format!("{:?}", result.status),
            success: result.success,
            error_message: result.error_message,
            number_component_errors: result.number_components_errors as i32,
            number_components_deployed: result.number_components_deployed as i32,
            number_components_total: result.number_components_total as i32,
            number_test_errors: result.number_tests_errors as i32,
            number_tests_completed: result.number_tests_completed as i32,
            number_tests_total: result.number_tests_total as i32,
        }),
        Err(e) => {
            let (code, message) = sanitize_metadata_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Retrieve metadata as a zip package.
pub(crate) async fn handle_metadata_retrieve(
    client: &MetadataClient,
    request: MetadataRetrieveRequest,
) -> BridgeResult<MetadataRetrieveResponse> {
    let result = if request.is_packaged {
        let package_name = match &request.package_name {
            Some(name) => name.as_str(),
            None => {
                return BridgeResult::err(
                    "INVALID_REQUEST",
                    "package_name is required when is_packaged is true",
                )
            }
        };
        client.retrieve_packaged(package_name).await
    } else {
        let mut manifest = busbar_sf_metadata::PackageManifest::new(request.api_version.clone());
        for t in &request.types {
            manifest = manifest.add_type(t.name.clone(), t.members.clone());
        }
        client.retrieve_unpackaged(&manifest).await
    };

    match result {
        Ok(async_process_id) => BridgeResult::ok(MetadataRetrieveResponse { async_process_id }),
        Err(e) => {
            let (code, message) = sanitize_metadata_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Check the status of a metadata retrieve operation.
pub(crate) async fn handle_metadata_check_retrieve_status(
    client: &MetadataClient,
    request: MetadataCheckRetrieveStatusRequest,
) -> BridgeResult<MetadataRetrieveResult> {
    match client
        .check_retrieve_status(&request.async_process_id, request.include_zip)
        .await
    {
        Ok(result) => BridgeResult::ok(MetadataRetrieveResult {
            id: result.id,
            done: result.done,
            status: format!("{:?}", result.status),
            success: result.success,
            zip_base64: result.zip_file,
            error_message: result.error_message,
        }),
        Err(e) => {
            let (code, message) = sanitize_metadata_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// List metadata components of a given type.
pub(crate) async fn handle_metadata_list(
    client: &MetadataClient,
    request: MetadataListRequest,
) -> BridgeResult<Vec<MetadataComponentInfo>> {
    match client
        .list_metadata(&request.metadata_type, request.folder.as_deref())
        .await
    {
        Ok(components) => BridgeResult::ok(
            components
                .into_iter()
                .map(|c| MetadataComponentInfo {
                    full_name: c.full_name,
                    file_name: c.file_name.unwrap_or_default(),
                    component_type: c.metadata_type,
                    id: c.id.unwrap_or_default(),
                    namespace_prefix: c.namespace_prefix,
                    last_modified_date: c.last_modified_date,
                })
                .collect(),
        ),
        Err(e) => {
            let (code, message) = sanitize_metadata_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Describe available metadata types.
pub(crate) async fn handle_metadata_describe(
    client: &MetadataClient,
) -> BridgeResult<MetadataDescribeResult> {
    match client.describe_metadata().await {
        Ok(result) => BridgeResult::ok(MetadataDescribeResult {
            metadata_objects: result
                .metadata_objects
                .into_iter()
                .map(|m| MetadataTypeInfo {
                    xml_name: m.xml_name,
                    directory_name: m.directory_name.unwrap_or_default(),
                    suffix: m.suffix,
                    in_folder: m.in_folder,
                    meta_file: m.meta_file,
                    child_xml_names: m.child_xml_names,
                })
                .collect(),
            organization_namespace: result.organization_namespace.unwrap_or_default(),
            partial_save_allowed: result.partial_save_allowed,
            test_required: result.test_required,
        }),
        Err(e) => {
            let (code, message) = sanitize_metadata_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

// =============================================================================
// Utility functions
// =============================================================================

fn parse_test_level(s: &str) -> Result<busbar_sf_metadata::TestLevel, String> {
    match s {
        "NoTestRun" => Ok(busbar_sf_metadata::TestLevel::NoTestRun),
        "RunLocalTests" => Ok(busbar_sf_metadata::TestLevel::RunLocalTests),
        "RunAllTestsInOrg" => Ok(busbar_sf_metadata::TestLevel::RunAllTestsInOrg),
        "RunSpecifiedTests" => Ok(busbar_sf_metadata::TestLevel::RunSpecifiedTests),
        _ => Err(format!("invalid test level: {s}")),
    }
}
