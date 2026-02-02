//! Host function registration modules.
//!
//! Each module contains the Extism wrapper functions and registration logic
//! for a specific API category.

// Re-export helpers from parent module for use in submodules
pub(self) use super::{bridge_host_fn, bridge_host_fn_no_input, BridgeState};

use extism::{PluginBuilder, UserData};

#[cfg(feature = "rest")]
mod binary;
#[cfg(feature = "rest")]
mod collections;
#[cfg(feature = "rest")]
mod composite;
#[cfg(feature = "rest")]
mod consent;
#[cfg(feature = "rest")]
mod embedded_service;
#[cfg(feature = "rest")]
mod knowledge;
#[cfg(feature = "rest")]
mod layouts;
#[cfg(feature = "rest")]
mod list_views;
#[cfg(feature = "rest")]
mod process;
#[cfg(feature = "rest")]
mod quick_actions;
#[cfg(feature = "rest")]
mod rest;
#[cfg(feature = "rest")]
mod scheduler;
#[cfg(feature = "rest")]
mod search;
#[cfg(feature = "rest")]
mod standalone;
#[cfg(feature = "rest")]
mod user_password;

#[cfg(feature = "bulk")]
mod bulk;

#[cfg(feature = "tooling")]
mod tooling;

#[cfg(feature = "metadata")]
mod metadata;

/// Register all enabled host functions based on feature flags.
pub(crate) fn register_all<'a>(
    mut builder: PluginBuilder<'a>,
    user_data: &UserData<BridgeState>,
) -> PluginBuilder<'a> {
    #[cfg(feature = "rest")]
    {
        builder = rest::register(builder, user_data);
        builder = composite::register(builder, user_data);
        builder = collections::register(builder, user_data);
        builder = process::register(builder, user_data);
        builder = list_views::register(builder, user_data);
        builder = quick_actions::register(builder, user_data);
        builder = layouts::register(builder, user_data);
        builder = knowledge::register(builder, user_data);
        builder = standalone::register(builder, user_data);
        builder = user_password::register(builder, user_data);
        builder = scheduler::register(builder, user_data);
        builder = consent::register(builder, user_data);
        builder = binary::register(builder, user_data);
        builder = embedded_service::register(builder, user_data);
        builder = search::register(builder, user_data);
    }

    #[cfg(feature = "bulk")]
    {
        builder = bulk::register(builder, user_data);
    }

    #[cfg(feature = "tooling")]
    {
        builder = tooling::register(builder, user_data);
    }

    #[cfg(feature = "metadata")]
    {
        builder = metadata::register(builder, user_data);
    }

    builder
}
