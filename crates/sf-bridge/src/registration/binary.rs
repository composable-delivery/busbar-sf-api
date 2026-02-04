//! Binary API host function wrappers and registration.
use super::{bridge_host_fn, BridgeState};
use crate::host_functions;
use busbar_sf_wasm_types::host_fn_names;
use extism::{UserData, ValType};

fn host_fn_get_blob(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_blob(&s.rest_client, r))
    })
}

fn host_fn_get_relationship(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_relationship(&s.rest_client, r))
    })
}

fn host_fn_get_rich_text_image(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_rich_text_image(
                &s.rest_client,
                r,
            ))
    })
}

/// Register all binary API host functions.
pub(super) fn register<'a>(
    builder: extism::PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> extism::PluginBuilder<'a> {
    builder
        .with_function(
            host_fn_names::GET_BLOB,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_blob,
        )
        .with_function(
            host_fn_names::GET_RELATIONSHIP,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_relationship,
        )
        .with_function(
            host_fn_names::GET_RICH_TEXT_IMAGE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_rich_text_image,
        )
}
