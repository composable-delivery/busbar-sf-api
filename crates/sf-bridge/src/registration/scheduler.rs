//! Scheduler API host function wrappers and registration.
use super::{bridge_host_fn, bridge_host_fn_no_input, BridgeState};
use crate::host_functions;
use busbar_sf_wasm_types::host_fn_names;
use extism::{UserData, ValType};

fn host_fn_appointment_candidates(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_appointment_candidates(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_appointment_slots(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_appointment_slots(&s.rest_client, r))
    })
}

/// Register all scheduler API host functions.
pub(super) fn register<'a>(
    builder: extism::PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> extism::PluginBuilder<'a> {
    builder
        .with_function(
            host_fn_names::APPOINTMENT_CANDIDATES,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_appointment_candidates,
        )
        .with_function(
            host_fn_names::APPOINTMENT_SLOTS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_appointment_slots,
        )
}
