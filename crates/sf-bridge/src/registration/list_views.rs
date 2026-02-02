//! List Views API host function wrappers and registration.
use super::{bridge_host_fn, bridge_host_fn_no_input, BridgeState};
use crate::host_functions;
use busbar_sf_wasm_types::host_fn_names;
use extism::{UserData, ValType};

fn host_fn_describe_list_view(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_list_view(&s.rest_client, r))
    })
}

fn host_fn_execute_list_view(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_execute_list_view(&s.rest_client, r))
    })
}

fn host_fn_get_list_view(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_list_view(&s.rest_client, r))
    })
}

fn host_fn_list_views(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_list_views(&s.rest_client, r))
    })
}

/// Register all list_views API host functions.
pub(super) fn register<'a>(
    builder: extism::PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> extism::PluginBuilder<'a> {
    builder
        .with_function(
            host_fn_names::DESCRIBE_LIST_VIEW,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_list_view,
        )
        .with_function(
            host_fn_names::EXECUTE_LIST_VIEW,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_execute_list_view,
        )
        .with_function(
            host_fn_names::GET_LIST_VIEW,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_list_view,
        )
        .with_function(
            host_fn_names::LIST_VIEWS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_views,
        )
}
