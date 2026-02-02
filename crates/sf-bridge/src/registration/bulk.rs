//! Bulk API host function wrappers and registration.
use super::{bridge_host_fn, bridge_host_fn_no_input, BridgeState};
use crate::host_functions;
use busbar_sf_wasm_types::host_fn_names;
use extism::{UserData, ValType};

fn host_fn_bulk_abort_ingest_job(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_abort_ingest_job(
                &s.bulk_client,
                r,
            ))
    })
}

fn host_fn_bulk_abort_query_job(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_abort_query_job(
                &s.bulk_client,
                r,
            ))
    })
}

fn host_fn_bulk_close_ingest_job(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_close_ingest_job(
                &s.bulk_client,
                r,
            ))
    })
}

fn host_fn_bulk_create_ingest_job(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_create_ingest_job(
                &s.bulk_client,
                r,
            ))
    })
}

fn host_fn_bulk_delete_ingest_job(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_delete_ingest_job(
                &s.bulk_client,
                r,
            ))
    })
}

fn host_fn_bulk_get_all_ingest_jobs(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_bulk_get_all_ingest_jobs(
                &s.bulk_client,
            ))
    })
}

fn host_fn_bulk_get_ingest_job(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_get_ingest_job(
                &s.bulk_client,
                r,
            ))
    })
}

fn host_fn_bulk_get_job_results(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_get_job_results(
                &s.bulk_client,
                r,
            ))
    })
}

fn host_fn_bulk_get_query_results(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_get_query_results(
                &s.bulk_client,
                r,
            ))
    })
}

fn host_fn_bulk_upload_job_data(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_upload_job_data(
                &s.bulk_client,
                r,
            ))
    })
}

/// Register all bulk API host functions.
pub(super) fn register<'a>(
    builder: extism::PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> extism::PluginBuilder<'a> {
    builder
        .with_function(
            host_fn_names::BULK_ABORT_INGEST_JOB,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_abort_ingest_job,
        )
        .with_function(
            host_fn_names::BULK_ABORT_QUERY_JOB,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_abort_query_job,
        )
        .with_function(
            host_fn_names::BULK_CLOSE_INGEST_JOB,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_close_ingest_job,
        )
        .with_function(
            host_fn_names::BULK_CREATE_INGEST_JOB,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_create_ingest_job,
        )
        .with_function(
            host_fn_names::BULK_DELETE_INGEST_JOB,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_delete_ingest_job,
        )
        .with_function(
            host_fn_names::BULK_GET_ALL_INGEST_JOBS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_get_all_ingest_jobs,
        )
        .with_function(
            host_fn_names::BULK_GET_INGEST_JOB,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_get_ingest_job,
        )
        .with_function(
            host_fn_names::BULK_GET_JOB_RESULTS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_get_job_results,
        )
        .with_function(
            host_fn_names::BULK_GET_QUERY_RESULTS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_get_query_results,
        )
        .with_function(
            host_fn_names::BULK_UPLOAD_JOB_DATA,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_upload_job_data,
        )
}
