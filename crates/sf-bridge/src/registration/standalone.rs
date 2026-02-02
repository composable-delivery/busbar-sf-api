//! Standalone API host function wrappers and registration.
use super::{bridge_host_fn, bridge_host_fn_no_input, BridgeState};
use crate::host_functions;
use busbar_sf_wasm_types::host_fn_names;
use extism::{UserData, ValType};

fn host_fn_app_menu(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_app_menu(&s.rest_client, r))
    })
}

fn host_fn_lightning_toggle_metrics(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_lightning_toggle_metrics(
                &s.rest_client,
            ))
    })
}

fn host_fn_lightning_usage(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_lightning_usage(&s.rest_client))
    })
}

fn host_fn_platform_event_schema(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_platform_event_schema(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_recent_items(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_recent_items(&s.rest_client))
    })
}

fn host_fn_relevant_items(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_relevant_items(&s.rest_client))
    })
}

fn host_fn_tabs(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_tabs(&s.rest_client))
    })
}

fn host_fn_theme(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_theme(&s.rest_client))
    })
}

/// Register all standalone API host functions.
pub(super) fn register<'a>(
    builder: extism::PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> extism::PluginBuilder<'a> {
    builder
        .with_function(
            host_fn_names::APP_MENU,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_app_menu,
        )
        .with_function(
            host_fn_names::LIGHTNING_TOGGLE_METRICS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_lightning_toggle_metrics,
        )
        .with_function(
            host_fn_names::LIGHTNING_USAGE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_lightning_usage,
        )
        .with_function(
            host_fn_names::PLATFORM_EVENT_SCHEMA,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_platform_event_schema,
        )
        .with_function(
            host_fn_names::RECENT_ITEMS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_recent_items,
        )
        .with_function(
            host_fn_names::RELEVANT_ITEMS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_relevant_items,
        )
        .with_function(
            host_fn_names::TABS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_tabs,
        )
        .with_function(
            host_fn_names::THEME,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_theme,
        )
}
