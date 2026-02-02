//! Process & Approvals API host function wrappers and registration.
use super::{bridge_host_fn, bridge_host_fn_no_input, BridgeState};
use crate::host_functions;
use busbar_sf_wasm_types::host_fn_names;
use extism::{UserData, ValType};

fn host_fn_list_pending_approvals(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_list_pending_approvals(
                &s.rest_client,
            ))
    })
}

fn host_fn_list_process_rules(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_list_process_rules(&s.rest_client))
    })
}

fn host_fn_list_process_rules_for_sobject(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_list_process_rules_for_sobject(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_submit_approval(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_submit_approval(&s.rest_client, r))
    })
}

fn host_fn_trigger_process_rules(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_trigger_process_rules(
                &s.rest_client,
                r,
            ))
    })
}

/// Register all process API host functions.
pub(super) fn register<'a>(
    builder: extism::PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> extism::PluginBuilder<'a> {
    builder
        .with_function(
            host_fn_names::LIST_PENDING_APPROVALS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_pending_approvals,
        )
        .with_function(
            host_fn_names::LIST_PROCESS_RULES,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_process_rules,
        )
        .with_function(
            host_fn_names::LIST_PROCESS_RULES_FOR_SOBJECT,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_process_rules_for_sobject,
        )
        .with_function(
            host_fn_names::SUBMIT_APPROVAL,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_submit_approval,
        )
        .with_function(
            host_fn_names::TRIGGER_PROCESS_RULES,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_trigger_process_rules,
        )
}
