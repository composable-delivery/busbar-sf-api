//! Knowledge API host function wrappers and registration.
use super::{bridge_host_fn, bridge_host_fn_no_input, BridgeState};
use crate::host_functions;
use busbar_sf_wasm_types::host_fn_names;
use extism::{UserData, ValType};

fn host_fn_data_categories(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_data_categories(&s.rest_client, r))
    })
}

fn host_fn_data_category_groups(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_data_category_groups(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_knowledge_articles(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_knowledge_articles(&s.rest_client, r))
    })
}

fn host_fn_knowledge_settings(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_knowledge_settings(&s.rest_client))
    })
}

/// Register all knowledge API host functions.
pub(super) fn register<'a>(
    builder: extism::PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> extism::PluginBuilder<'a> {
    builder
        .with_function(
            host_fn_names::DATA_CATEGORIES,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_data_categories,
        )
        .with_function(
            host_fn_names::DATA_CATEGORY_GROUPS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_data_category_groups,
        )
        .with_function(
            host_fn_names::KNOWLEDGE_ARTICLES,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_knowledge_articles,
        )
        .with_function(
            host_fn_names::KNOWLEDGE_SETTINGS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_knowledge_settings,
        )
}
