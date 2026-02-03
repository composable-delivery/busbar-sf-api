//! Metadata API host function wrappers and registration.
use super::{bridge_host_fn, bridge_host_fn_no_input, BridgeState};
use crate::host_functions;
use busbar_sf_wasm_types::host_fn_names;
use extism::{UserData, ValType};

fn host_fn_metadata_check_deploy_status(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s: &BridgeState, r| {
        let client = s.metadata_client();
        s.handle
            .block_on(host_functions::handle_metadata_check_deploy_status(
                &client, r,
            ))
    })
}

fn host_fn_metadata_check_retrieve_status(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s: &BridgeState, r| {
        let client = s.metadata_client();
        s.handle
            .block_on(host_functions::handle_metadata_check_retrieve_status(
                &client, r,
            ))
    })
}

fn host_fn_metadata_deploy(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s: &BridgeState, r| {
        let client = s.metadata_client();
        s.handle
            .block_on(host_functions::handle_metadata_deploy(&client, r))
    })
}

fn host_fn_metadata_describe(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s: &BridgeState| {
        let client = s.metadata_client();
        s.handle
            .block_on(host_functions::handle_metadata_describe(&client))
    })
}

fn host_fn_metadata_list(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s: &BridgeState, r| {
        let client = s.metadata_client();
        s.handle
            .block_on(host_functions::handle_metadata_list(&client, r))
    })
}

fn host_fn_metadata_retrieve(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s: &BridgeState, r| {
        let client = s.metadata_client();
        s.handle
            .block_on(host_functions::handle_metadata_retrieve(&client, r))
    })
}

/// Register all metadata API host functions.
pub(super) fn register<'a>(
    builder: extism::PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> extism::PluginBuilder<'a> {
    builder
        .with_function(
            host_fn_names::METADATA_CHECK_DEPLOY_STATUS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_metadata_check_deploy_status,
        )
        .with_function(
            host_fn_names::METADATA_CHECK_RETRIEVE_STATUS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_metadata_check_retrieve_status,
        )
        .with_function(
            host_fn_names::METADATA_DEPLOY,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_metadata_deploy,
        )
        .with_function(
            host_fn_names::METADATA_DESCRIBE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_metadata_describe,
        )
        .with_function(
            host_fn_names::METADATA_LIST,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_metadata_list,
        )
        .with_function(
            host_fn_names::METADATA_RETRIEVE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_metadata_retrieve,
        )
}
