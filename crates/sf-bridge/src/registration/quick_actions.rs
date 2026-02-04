//! Quick Actions API host function wrappers and registration.
use super::{bridge_host_fn, bridge_host_fn_no_input, BridgeState};
use crate::host_functions;
use busbar_sf_wasm_types::host_fn_names;
use extism::{UserData, ValType};

fn host_fn_describe_custom_action(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_custom_action(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_describe_global_quick_action(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_global_quick_action(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_describe_quick_action(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_quick_action(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_describe_standard_action(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_standard_action(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_invoke_custom_action(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_invoke_custom_action(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_invoke_quick_action(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_invoke_quick_action(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_invoke_standard_action(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_invoke_standard_action(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_list_custom_action_types(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_list_custom_action_types(
                &s.rest_client,
            ))
    })
}

fn host_fn_list_custom_actions(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_list_custom_actions(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_list_global_quick_actions(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_list_global_quick_actions(
                &s.rest_client,
            ))
    })
}

fn host_fn_list_quick_actions(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_list_quick_actions(&s.rest_client, r))
    })
}

fn host_fn_list_standard_actions(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_list_standard_actions(&s.rest_client))
    })
}

/// Register all quick_actions API host functions.
pub(super) fn register<'a>(
    builder: extism::PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> extism::PluginBuilder<'a> {
    builder
        .with_function(
            host_fn_names::DESCRIBE_CUSTOM_ACTION,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_custom_action,
        )
        .with_function(
            host_fn_names::DESCRIBE_GLOBAL_QUICK_ACTION,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_global_quick_action,
        )
        .with_function(
            host_fn_names::DESCRIBE_QUICK_ACTION,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_quick_action,
        )
        .with_function(
            host_fn_names::DESCRIBE_STANDARD_ACTION,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_standard_action,
        )
        .with_function(
            host_fn_names::INVOKE_CUSTOM_ACTION,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_invoke_custom_action,
        )
        .with_function(
            host_fn_names::INVOKE_QUICK_ACTION,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_invoke_quick_action,
        )
        .with_function(
            host_fn_names::INVOKE_STANDARD_ACTION,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_invoke_standard_action,
        )
        .with_function(
            host_fn_names::LIST_CUSTOM_ACTION_TYPES,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_custom_action_types,
        )
        .with_function(
            host_fn_names::LIST_CUSTOM_ACTIONS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_custom_actions,
        )
        .with_function(
            host_fn_names::LIST_GLOBAL_QUICK_ACTIONS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_global_quick_actions,
        )
        .with_function(
            host_fn_names::LIST_QUICK_ACTIONS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_quick_actions,
        )
        .with_function(
            host_fn_names::LIST_STANDARD_ACTIONS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_standard_actions,
        )
}
