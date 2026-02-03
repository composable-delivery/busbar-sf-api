//! Search API host function wrappers and registration.
use super::{bridge_host_fn, bridge_host_fn_no_input, BridgeState};
use crate::host_functions;
use busbar_sf_wasm_types::host_fn_names;
use extism::{UserData, ValType};

fn host_fn_parameterized_search(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_parameterized_search(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_search_result_layouts(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_search_result_layouts(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_search_scope_order(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_search_scope_order(&s.rest_client))
    })
}

fn host_fn_search_suggestions(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_search_suggestions(&s.rest_client, r))
    })
}

/// Register all search API host functions.
pub(super) fn register<'a>(
    builder: extism::PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> extism::PluginBuilder<'a> {
    builder
        .with_function(
            host_fn_names::PARAMETERIZED_SEARCH,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_parameterized_search,
        )
        .with_function(
            host_fn_names::SEARCH_RESULT_LAYOUTS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_search_result_layouts,
        )
        .with_function(
            host_fn_names::SEARCH_SCOPE_ORDER,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_search_scope_order,
        )
        .with_function(
            host_fn_names::SEARCH_SUGGESTIONS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_search_suggestions,
        )
}
