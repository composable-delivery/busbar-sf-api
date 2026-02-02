//! Composite API host function wrappers and registration.
use super::{bridge_host_fn, bridge_host_fn_no_input, BridgeState};
use crate::host_functions;
use busbar_sf_wasm_types::host_fn_names;
use extism::{UserData, ValType};

fn host_fn_composite(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_composite(&s.rest_client, r))
    })
}

fn host_fn_composite_batch(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_composite_batch(&s.rest_client, r))
    })
}

fn host_fn_composite_graph(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_composite_graph(&s.rest_client, r))
    })
}

fn host_fn_composite_tree(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_composite_tree(&s.rest_client, r))
    })
}

/// Register all composite API host functions.
pub(super) fn register<'a>(
    builder: extism::PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> extism::PluginBuilder<'a> {
    builder
        .with_function(
            host_fn_names::COMPOSITE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_composite,
        )
        .with_function(
            host_fn_names::COMPOSITE_BATCH,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_composite_batch,
        )
        .with_function(
            host_fn_names::COMPOSITE_GRAPH,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_composite_graph,
        )
        .with_function(
            host_fn_names::COMPOSITE_TREE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_composite_tree,
        )
}
