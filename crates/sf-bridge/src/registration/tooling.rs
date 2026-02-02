//! Tooling API host function wrappers and registration.
use super::{bridge_host_fn, bridge_host_fn_no_input, BridgeState};
use crate::host_functions;
use busbar_sf_wasm_types::host_fn_names;
use extism::{UserData, ValType};

fn host_fn_tooling_create(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_tooling_create(&s.tooling_client, r))
    })
}

fn host_fn_tooling_delete(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_tooling_delete(&s.tooling_client, r))
    })
}

fn host_fn_tooling_execute_anonymous(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_tooling_execute_anonymous(
                &s.tooling_client,
                r,
            ))
    })
}

fn host_fn_tooling_get(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_tooling_get(&s.tooling_client, r))
    })
}

fn host_fn_tooling_query(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_tooling_query(&s.tooling_client, r))
    })
}

/// Register all tooling API host functions.
pub(super) fn register<'a>(
    builder: extism::PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> extism::PluginBuilder<'a> {
    builder
        .with_function(
            host_fn_names::TOOLING_CREATE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_tooling_create,
        )
        .with_function(
            host_fn_names::TOOLING_DELETE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_tooling_delete,
        )
        .with_function(
            host_fn_names::TOOLING_EXECUTE_ANONYMOUS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_tooling_execute_anonymous,
        )
        .with_function(
            host_fn_names::TOOLING_GET,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_tooling_get,
        )
        .with_function(
            host_fn_names::TOOLING_QUERY,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_tooling_query,
        )
}
