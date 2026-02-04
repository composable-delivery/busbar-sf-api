//! Layouts API host function wrappers and registration.
use super::{bridge_host_fn, bridge_host_fn_no_input, BridgeState};
use crate::host_functions;
use busbar_sf_wasm_types::host_fn_names;
use extism::{UserData, ValType};

fn host_fn_compact_layouts_multi(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_compact_layouts_multi(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_describe_approval_layouts(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_approval_layouts(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_describe_compact_layouts(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_compact_layouts(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_describe_global_publisher_layouts(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_describe_global_publisher_layouts(
                &s.rest_client,
            ))
    })
}

fn host_fn_describe_layouts(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_layouts(&s.rest_client, r))
    })
}

fn host_fn_describe_named_layout(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_named_layout(
                &s.rest_client,
                r,
            ))
    })
}

/// Register all layouts API host functions.
pub(super) fn register<'a>(
    builder: extism::PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> extism::PluginBuilder<'a> {
    builder
        .with_function(
            host_fn_names::COMPACT_LAYOUTS_MULTI,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_compact_layouts_multi,
        )
        .with_function(
            host_fn_names::DESCRIBE_APPROVAL_LAYOUTS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_approval_layouts,
        )
        .with_function(
            host_fn_names::DESCRIBE_COMPACT_LAYOUTS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_compact_layouts,
        )
        .with_function(
            host_fn_names::DESCRIBE_GLOBAL_PUBLISHER_LAYOUTS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_global_publisher_layouts,
        )
        .with_function(
            host_fn_names::DESCRIBE_LAYOUTS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_layouts,
        )
        .with_function(
            host_fn_names::DESCRIBE_NAMED_LAYOUT,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_named_layout,
        )
}
