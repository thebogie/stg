//! Shared helpers for SurrealDB queries and result parsing.
//!
//! **ID conventions:** See `docs/SURREALDB_ID_CONVENTIONS.md` in the repo.
//! - Canonical string format in app/DTOs and APIs: `"table/key"` (slash, no backticks). Use `type::thing('table', $key)` in SQL with raw key.
//! - **Always** convert Thing to string via `thing_to_record_id()` so backticks and colon/slash are normalized consistently across backend (and frontend receives the same format from APIs).
//! - Deserialize ID columns as `Option<surrealdb::sql::Thing>`; convert to string only with `thing_to_record_id`.
//! - For `INSIDE $ids` bindings, use strings in `"table:key"` (colon) form — use `record_ids_to_inside_value()`.

use serde_json::Value;

/// Strip table prefix and SurrealDB id wrappers to get raw key for `type::thing('table', $key)`.
/// Accepts "table/key" or "table:key". Returns key only (no backticks/angle brackets).
#[must_use]
pub fn record_id_to_key(id: &str, table: &str) -> String {
    let prefix = format!("{}/", table);
    let prefix_colon = format!("{}:", table);
    let key = id
        .trim_start_matches(&prefix)
        .trim_start_matches(&prefix_colon)
        .trim_matches('`')
        .trim_matches('\u{27e8}') // ⟨
        .trim_matches('\u{27e9}'); // ⟩
    key.to_string()
}

/// Normalize SurrealDB Thing to canonical "table/key" for DTOs and map lookups.
/// Strips backticks so DB serialization (e.g. `player/`uuid``) matches API form (`player/uuid`)
/// for comparisons and consistent IDs across backend and frontend.
#[must_use]
pub fn thing_to_record_id(t: &Option<surrealdb::sql::Thing>) -> String {
    t.as_ref()
        .map(|x| x.to_string().replace(':', "/").replace('`', ""))
        .unwrap_or_default()
}

/// Convert a list of canonical record IDs ("table/key") to the form used for `INSIDE $ids` bindings.
/// SurrealDB expects "table:key" (colon) when binding a string array for INSIDE. Use this so all
/// call sites use one convention; see docs/SURREALDB_ID_CONVENTIONS.md.
#[must_use]
pub fn record_ids_to_inside_value(ids: &[String], _table: &str) -> Vec<String> {
    ids.iter().map(|s| s.replace('/', ":")).collect()
}

/// Extract integer from SurrealDB result: bare number, or `{ "count": n }` from `count()`.
#[must_use]
pub fn scalar_i64(v: &Value) -> i64 {
    if let Some(n) = v.as_i64() {
        return n;
    }
    if let Some(n) = v.as_u64() {
        return n as i64;
    }
    if let Some(obj) = v.as_object() {
        if let Some(n) = obj.get("count").and_then(|c| c.as_i64()) {
            return n;
        }
        if let Some(n) = obj.get("count").and_then(|c| c.as_u64()) {
            return n as i64;
        }
    }
    if let Some(arr) = v.as_array() {
        if let Some(first) = arr.first() {
            return scalar_i64(first);
        }
    }
    0
}

/// Extract float from SurrealDB result (e.g. `math::mean` as number or wrapped).
#[must_use]
pub fn scalar_f64(v: &Value) -> f64 {
    if let Some(n) = v.as_f64() {
        return n;
    }
    if let Some(n) = v.as_i64() {
        return n as f64;
    }
    if let Some(n) = v.as_u64() {
        return n as f64;
    }
    if let Some(obj) = v.as_object() {
        if let Some(n) = obj.get("count").and_then(|c| c.as_f64()) {
            return n;
        }
        if let Some(n) = obj.get("count").and_then(|c| c.as_i64()) {
            return n as f64;
        }
    }
    if let Some(arr) = v.as_array() {
        if let Some(first) = arr.first() {
            return scalar_f64(first);
        }
    }
    0.0
}
