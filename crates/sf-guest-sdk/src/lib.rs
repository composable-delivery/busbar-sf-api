//! # busbar-sf-guest-sdk
//!
//! Guest SDK for building WASM plugins that interact with Salesforce APIs
//! through the busbar bridge.
//!
//! This crate is compiled to `wasm32-unknown-unknown` and loaded by a host
//! running [`busbar-sf-bridge`]. All Salesforce operations are executed by
//! the host - this SDK just provides ergonomic wrappers around the host
//! function imports.
//!
//! ## Security
//!
//! Your plugin code **never sees Salesforce credentials**. The host manages
//! all authentication. You call functions like [`query`] and get results back.
//! There is no way to extract the access token from within the WASM sandbox.
//!
//! ## Example Plugin
//!
//! ```rust,ignore
//! use busbar_sf_guest_sdk::*;
//! use extism_pdk::*;
//!
//! #[plugin_fn]
//! pub fn run(_input: String) -> FnResult<Json<Vec<serde_json::Value>>> {
//!     let accounts = query("SELECT Id, Name FROM Account LIMIT 10")?;
//!     Ok(Json(accounts.records))
//! }
//! ```

pub use busbar_sf_wasm_types::*;
use extism_pdk::*;

// =============================================================================
// Host function imports
//
// These are provided by the sf-bridge host at runtime. The `extern "ExtismHost"`
// block declares them so the WASM module knows to import them.
// =============================================================================

#[host_fn]
extern "ExtismHost" {
    fn sf_query(input: Vec<u8>) -> Vec<u8>;
    fn sf_create(input: Vec<u8>) -> Vec<u8>;
    fn sf_get(input: Vec<u8>) -> Vec<u8>;
    fn sf_update(input: Vec<u8>) -> Vec<u8>;
    fn sf_delete(input: Vec<u8>) -> Vec<u8>;
    fn sf_upsert(input: Vec<u8>) -> Vec<u8>;
    fn sf_describe_global() -> Vec<u8>;
    fn sf_describe_sobject(input: Vec<u8>) -> Vec<u8>;
    fn sf_search(input: Vec<u8>) -> Vec<u8>;
    fn sf_composite(input: Vec<u8>) -> Vec<u8>;
    fn sf_create_multiple(input: Vec<u8>) -> Vec<u8>;
    fn sf_delete_multiple(input: Vec<u8>) -> Vec<u8>;
    fn sf_limits() -> Vec<u8>;
}

// =============================================================================
// Ergonomic wrappers
//
// These functions handle serialization/deserialization and provide a clean
// Rust API. Plugin authors use these instead of the raw host functions.
// =============================================================================

/// Execute a SOQL query.
///
/// Returns the first page of results. Check `done` and `next_records_url`
/// for pagination.
///
/// # Example
///
/// ```rust,ignore
/// let result = query("SELECT Id, Name FROM Account LIMIT 10")?;
/// for record in &result.records {
///     // process records...
/// }
/// ```
pub fn query(soql: &str) -> Result<QueryResponse, Error> {
    let request = QueryRequest {
        soql: soql.to_string(),
        include_deleted: false,
    };
    call_host_fn(|input| unsafe { sf_query(input) }, &request)
}

/// Execute a SOQL query including deleted/archived records.
pub fn query_all(soql: &str) -> Result<QueryResponse, Error> {
    let request = QueryRequest {
        soql: soql.to_string(),
        include_deleted: true,
    };
    call_host_fn(|input| unsafe { sf_query(input) }, &request)
}

/// Create a new record.
///
/// Returns the result including the new record's ID.
///
/// # Example
///
/// ```rust,ignore
/// let result = create("Account", &serde_json::json!({"Name": "Acme Corp"}))?;
/// let new_id = result.id;
/// ```
pub fn create(sobject: &str, record: &serde_json::Value) -> Result<CreateResponse, Error> {
    let request = CreateRequest {
        sobject: sobject.to_string(),
        record: record.clone(),
    };
    call_host_fn(|input| unsafe { sf_create(input) }, &request)
}

/// Get a record by ID.
///
/// # Example
///
/// ```rust,ignore
/// let record: serde_json::Value = get("Account", "001xx000003DgAAAS", None)?;
/// let name = &record["Name"];
/// ```
pub fn get(
    sobject: &str,
    id: &str,
    fields: Option<Vec<String>>,
) -> Result<serde_json::Value, Error> {
    let request = GetRequest {
        sobject: sobject.to_string(),
        id: id.to_string(),
        fields,
    };
    call_host_fn(|input| unsafe { sf_get(input) }, &request)
}

/// Update a record.
///
/// # Example
///
/// ```rust,ignore
/// update("Account", "001xx000003DgAAAS", &serde_json::json!({"Name": "New Name"}))?;
/// ```
pub fn update(sobject: &str, id: &str, record: &serde_json::Value) -> Result<(), Error> {
    let request = UpdateRequest {
        sobject: sobject.to_string(),
        id: id.to_string(),
        record: record.clone(),
    };
    call_host_fn(|input| unsafe { sf_update(input) }, &request)
}

/// Delete a record.
///
/// # Example
///
/// ```rust,ignore
/// delete("Account", "001xx000003DgAAAS")?;
/// ```
pub fn delete(sobject: &str, id: &str) -> Result<(), Error> {
    let request = DeleteRequest {
        sobject: sobject.to_string(),
        id: id.to_string(),
    };
    call_host_fn(|input| unsafe { sf_delete(input) }, &request)
}

/// Upsert a record using an external ID.
///
/// Creates the record if it doesn't exist, updates it if it does.
pub fn upsert(
    sobject: &str,
    external_id_field: &str,
    external_id_value: &str,
    record: &serde_json::Value,
) -> Result<UpsertResponse, Error> {
    let request = UpsertRequest {
        sobject: sobject.to_string(),
        external_id_field: external_id_field.to_string(),
        external_id_value: external_id_value.to_string(),
        record: record.clone(),
    };
    call_host_fn(|input| unsafe { sf_upsert(input) }, &request)
}

/// Get metadata for all SObjects in the org.
pub fn describe_global() -> Result<serde_json::Value, Error> {
    let output = unsafe { sf_describe_global() }
        .map_err(|e| Error::msg(format!("host function error: {e}")))?;
    let result: BridgeResult<serde_json::Value> = serde_json::from_slice(&output)
        .map_err(|e| Error::msg(format!("deserialize error: {e}")))?;
    result
        .into_result()
        .map_err(|e| Error::msg(e.to_string()))
}

/// Get metadata for a specific SObject.
pub fn describe_sobject(sobject: &str) -> Result<serde_json::Value, Error> {
    let request = DescribeSObjectRequest {
        sobject: sobject.to_string(),
    };
    call_host_fn(|input| unsafe { sf_describe_sobject(input) }, &request)
}

/// Execute a SOSL full-text search.
pub fn search(sosl: &str) -> Result<SearchResponse, Error> {
    let request = SearchRequest {
        sosl: sosl.to_string(),
    };
    call_host_fn(|input| unsafe { sf_search(input) }, &request)
}

/// Execute a composite API request.
///
/// Allows multiple subrequests in a single API call. Subrequests can
/// reference results from earlier subrequests using `@{referenceId}`.
pub fn composite(request: &CompositeRequest) -> Result<CompositeResponse, Error> {
    call_host_fn(|input| unsafe { sf_composite(input) }, request)
}

/// Create multiple records in a single request (up to 200).
pub fn create_multiple(
    sobject: &str,
    records: Vec<serde_json::Value>,
    all_or_none: bool,
) -> Result<Vec<CollectionResult>, Error> {
    let request = CreateMultipleRequest {
        sobject: sobject.to_string(),
        records,
        all_or_none,
    };
    call_host_fn(|input| unsafe { sf_create_multiple(input) }, &request)
}

/// Delete multiple records in a single request (up to 200).
pub fn delete_multiple(
    ids: Vec<String>,
    all_or_none: bool,
) -> Result<Vec<CollectionResult>, Error> {
    let request = DeleteMultipleRequest { ids, all_or_none };
    call_host_fn(|input| unsafe { sf_delete_multiple(input) }, &request)
}

/// Get API limits for the org.
pub fn limits() -> Result<serde_json::Value, Error> {
    let output = unsafe { sf_limits() }
        .map_err(|e| Error::msg(format!("host function error: {e}")))?;
    let result: BridgeResult<serde_json::Value> = serde_json::from_slice(&output)
        .map_err(|e| Error::msg(format!("deserialize error: {e}")))?;
    result
        .into_result()
        .map_err(|e| Error::msg(e.to_string()))
}

// =============================================================================
// Internal helpers
// =============================================================================

/// Call a host function with serialization/deserialization.
fn call_host_fn<Req, Resp>(
    host_fn: impl FnOnce(Vec<u8>) -> Result<Vec<u8>, Error>,
    request: &Req,
) -> Result<Resp, Error>
where
    Req: serde::Serialize,
    Resp: serde::de::DeserializeOwned,
{
    let input = serde_json::to_vec(request)
        .map_err(|e| Error::msg(format!("serialize error: {e}")))?;
    let output = host_fn(input)?;
    let result: BridgeResult<Resp> = serde_json::from_slice(&output)
        .map_err(|e| Error::msg(format!("deserialize error: {e}")))?;
    result
        .into_result()
        .map_err(|e| Error::msg(e.to_string()))
}
