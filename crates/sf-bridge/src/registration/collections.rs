//! Collections API host function wrappers and registration.
use super::{bridge_host_fn, BridgeState};
use crate::host_functions;
use busbar_sf_wasm_types::host_fn_names;
use extism::{UserData, ValType};

fn host_fn_create_multiple(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_create_multiple(&s.rest_client, r))
    })
}

fn host_fn_delete_multiple(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_delete_multiple(&s.rest_client, r))
    })
}

fn host_fn_get_multiple(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_multiple(&s.rest_client, r))
    })
}

fn host_fn_update_multiple(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_update_multiple(&s.rest_client, r))
    })
}

/// Register all collections API host functions.
pub(super) fn register<'a>(
    builder: extism::PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> extism::PluginBuilder<'a> {
    builder
        .with_function(
            host_fn_names::CREATE_MULTIPLE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_create_multiple,
        )
        .with_function(
            host_fn_names::DELETE_MULTIPLE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_delete_multiple,
        )
        .with_function(
            host_fn_names::GET_MULTIPLE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_multiple,
        )
        .with_function(
            host_fn_names::UPDATE_MULTIPLE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_update_multiple,
        )
}
