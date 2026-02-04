//! REST API host function wrappers and registration.
use super::{bridge_host_fn, bridge_host_fn_no_input, BridgeState};
use crate::host_functions;
use busbar_sf_wasm_types::host_fn_names;
use extism::{UserData, ValType};

fn host_fn_create(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_create(&s.rest_client, r))
    })
}

fn host_fn_delete(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_delete(&s.rest_client, r))
    })
}

fn host_fn_describe_global(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_describe_global(&s.rest_client))
    })
}

fn host_fn_describe_sobject(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_sobject(&s.rest_client, r))
    })
}

fn host_fn_get(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get(&s.rest_client, r))
    })
}

fn host_fn_get_deleted(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_deleted(&s.rest_client, r))
    })
}

fn host_fn_get_updated(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_updated(&s.rest_client, r))
    })
}

fn host_fn_limits(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_limits(&s.rest_client))
    })
}

fn host_fn_query(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_query(&s.rest_client, r))
    })
}

fn host_fn_query_more(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_query_more(&s.rest_client, r))
    })
}

fn host_fn_search(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_search(&s.rest_client, r))
    })
}

fn host_fn_update(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_update(&s.rest_client, r))
    })
}

fn host_fn_upsert(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_upsert(&s.rest_client, r))
    })
}

fn host_fn_versions(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_versions(&s.rest_client))
    })
}

/// Register all rest API host functions.
pub(super) fn register<'a>(
    builder: extism::PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> extism::PluginBuilder<'a> {
    builder
        .with_function(
            host_fn_names::CREATE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_create,
        )
        .with_function(
            host_fn_names::DELETE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_delete,
        )
        .with_function(
            host_fn_names::DESCRIBE_GLOBAL,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_global,
        )
        .with_function(
            host_fn_names::DESCRIBE_SOBJECT,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_sobject,
        )
        .with_function(
            host_fn_names::GET,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get,
        )
        .with_function(
            host_fn_names::GET_DELETED,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_deleted,
        )
        .with_function(
            host_fn_names::GET_UPDATED,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_updated,
        )
        .with_function(
            host_fn_names::LIMITS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_limits,
        )
        .with_function(
            host_fn_names::QUERY,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_query,
        )
        .with_function(
            host_fn_names::QUERY_MORE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_query_more,
        )
        .with_function(
            host_fn_names::SEARCH,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_search,
        )
        .with_function(
            host_fn_names::UPDATE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_update,
        )
        .with_function(
            host_fn_names::UPSERT,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_upsert,
        )
        .with_function(
            host_fn_names::VERSIONS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_versions,
        )
}
