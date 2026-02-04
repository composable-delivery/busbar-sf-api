//! User password API host function wrappers and registration.
use super::{bridge_host_fn, BridgeState};
use crate::host_functions;
use busbar_sf_wasm_types::host_fn_names;
use extism::{UserData, ValType};

fn host_fn_get_user_password_status(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_user_password_status(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_reset_user_password(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_reset_user_password(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_set_user_password(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_set_user_password(&s.rest_client, r))
    })
}

/// Register all user_password API host functions.
pub(super) fn register<'a>(
    builder: extism::PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> extism::PluginBuilder<'a> {
    builder
        .with_function(
            host_fn_names::GET_USER_PASSWORD_STATUS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_user_password_status,
        )
        .with_function(
            host_fn_names::RESET_USER_PASSWORD,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_reset_user_password,
        )
        .with_function(
            host_fn_names::SET_USER_PASSWORD,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_set_user_password,
        )
}
