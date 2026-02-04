//! Host function implementations - organized by API category.
//!
//! These modules contain the business logic for each bridge operation.
//! They are pure async functions that take typed requests and return typed
//! responses. The Extism wiring (memory management, serialization at the
//! ABI boundary) is handled in the parent module.

mod error;

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

// Re-export error sanitization functions

// Re-export handler functions based on features
#[cfg(feature = "rest")]
pub(crate) use binary::*;
#[cfg(feature = "rest")]
pub(crate) use collections::*;
#[cfg(feature = "rest")]
pub(crate) use composite::*;
#[cfg(feature = "rest")]
pub(crate) use consent::*;
#[cfg(feature = "rest")]
pub(crate) use embedded_service::*;
#[cfg(feature = "rest")]
pub(crate) use knowledge::*;
#[cfg(feature = "rest")]
pub(crate) use layouts::*;
#[cfg(feature = "rest")]
pub(crate) use list_views::*;
#[cfg(feature = "rest")]
pub(crate) use process::*;
#[cfg(feature = "rest")]
pub(crate) use quick_actions::*;
#[cfg(feature = "rest")]
pub(crate) use rest::*;
#[cfg(feature = "rest")]
pub(crate) use scheduler::*;
#[cfg(feature = "rest")]
pub(crate) use search::*;
#[cfg(feature = "rest")]
pub(crate) use standalone::*;
#[cfg(feature = "rest")]
pub(crate) use user_password::*;

#[cfg(feature = "bulk")]
pub(crate) use bulk::*;

#[cfg(feature = "tooling")]
pub(crate) use tooling::*;

#[cfg(feature = "metadata")]
pub(crate) use metadata::*;
