//! Shared helpers for SurrealDB queries and result parsing.
//!
//! **ID conventions:** See `docs/SURREALDB_ID_CONVENTIONS.md` in the repo.
//! - Canonical string format in app/DTOs and APIs: `"table/key"` (slash, no backticks).
//! - **Single-record lookup:** use `select_one_by_record_id(db, table, id)` — it handles UUID vs string keys and Thing binding so all tabs work without per-repo translation.
//! - **Reading id from query rows:** use `record_id_from_row()` / `record_id_from_field()` so backticks and angle brackets are normalized.
//! - **Thing → string for DTOs:** use `thing_to_record_id()` so the frontend always gets `"table/key"`.
//! - **INSIDE bindings:** use `record_ids_to_inside_value()` for `"table:key"` colon form.

use serde_json::Value;

/// SurrealQL: strip backticks from id (e.g. "contest:`key`" → "contest:key"). Compare to bound value in "table:key" colon form.
pub const SURREALQL_STRIP_BACKTICKS_ID: &str = "string::replace(string::concat(id), '`', '')";

/// SurrealQL: full normalize to "table/key" (strip backticks, colon→slash, angle brackets). Compare to $id_canonical in "table/key" form.
pub const SURREALQL_NORMALIZE_ID_FOR_COMPARE: &str = concat!(
    "string::replace(string::replace(string::replace(string::replace(string::concat(id), '`', ''), ':', '/'), '",
    "\u{27e8}",
    "', ''), '",
    "\u{27e9}",
    "', '')"
);

/// Angle-bracket chars SurrealDB may use in record id string form (e.g. `contest:⟨uuid⟩`).
const ID_ANGLE_OPEN: char = '\u{27e8}';  // ⟨
const ID_ANGLE_CLOSE: char = '\u{27e9}'; // ⟩

fn json_id_part_to_string(v: &Value) -> Option<String> {
    v.as_str()
        .map(String::from)
        .or_else(|| v.as_i64().map(|n| n.to_string()))
        .or_else(|| v.as_u64().map(|n| n.to_string()))
}

/// Normalize a record id string to canonical `"table/key"` (slash, no backticks, no angle brackets).
/// Use when you have a string that might be `"table:key"`, `"table:⟨key⟩"`, or with backticks.
#[must_use]
pub fn normalize_record_id_string(s: &str) -> String {
    s.replace(':', "/")
        .replace('`', "")
        .replace(ID_ANGLE_OPEN, "")
        .replace(ID_ANGLE_CLOSE, "")
}

/// Extract and normalize record id from a SurrealDB row (or any object with an id-like field).
/// Checks `"id"`, `"_id"`, and `"player_id"` (for analytics-style rows). Handles value as string
/// (`"table:key"` or `"table:⟨key⟩"`), as Thing object `{ tb, id }`, or as bare number when
/// `default_table_for_bare_number` is `Some("player")`. Returns canonical `"table/key"` with
/// backticks and angle brackets stripped. Use across the project for consistent ID handling.
#[must_use]
pub fn record_id_from_row(v: &Value, default_table_for_bare_number: Option<&str>) -> Option<String> {
    let id_val = v.get("id").or_else(|| v.get("_id")).or_else(|| v.get("player_id"))?;
    if let Some(s) = id_val.as_str() {
        return Some(normalize_record_id_string(s));
    }
    if let Some(table) = default_table_for_bare_number {
        if let Some(n) = id_val.as_i64() {
            return Some(format!("{}/{}", table, n));
        }
        if let Some(n) = id_val.as_u64() {
            return Some(format!("{}/{}", table, n));
        }
    }
    if let Some(tb) = id_val.get("tb").and_then(|x| x.as_str()) {
        if let Some(id_part) = id_val.get("id").and_then(json_id_part_to_string) {
            let key = id_part
                .trim_matches('`')
                .replace(ID_ANGLE_OPEN, "")
                .replace(ID_ANGLE_CLOSE, "");
            return Some(format!("{}/{}", tb, key));
        }
    }
    None
}

/// Build a SurrealDB RecordId from a canonical record id (`"table/key"`) for use in query bindings.
/// Use when doing `WHERE id = $rid`: the Rust SDK sends bound strings as quoted literals, so
/// binding a RecordId ensures record-to-record comparison and matches stored record ids.
#[must_use]
pub fn record_id_to_thing(id: &str, table: &str) -> surrealdb::types::RecordId {
    let key = record_id_to_key(id, table);
    surrealdb::types::RecordId::new(table, key)
}

/// Strip table prefix and SurrealDB id wrappers to get raw key for `type::record('table', $key)`.
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

/// Normalize SurrealDB RecordId to canonical "table/key" for DTOs and map lookups.
#[must_use]
pub fn thing_to_record_id(t: &Option<surrealdb::types::RecordId>) -> String {
    t.as_ref().map(record_id_to_canonical).unwrap_or_default()
}

/// Convert a RecordId to canonical "table/key" string.
#[must_use]
pub fn record_id_to_canonical(rid: &surrealdb::types::RecordId) -> String {
    use surrealdb::types::RecordIdKey;
    let table = rid.table.as_str();
    let key_str = match &rid.key {
        RecordIdKey::String(s) => s.clone(),
        RecordIdKey::Number(n) => n.to_string(), // surrealdb_types::Number implements Display
        RecordIdKey::Uuid(u) => u.to_string(),
        _ => return format!("{}:", table),
    };
    format!("{}/{}", table, key_str)
}

/// Extract and normalize a record id from a row field (e.g. edge `out` or `in`).
/// Use for edge tables where the row has `out`/`in` as string or Thing. Returns canonical `"table/key"`.
/// For INSIDE bindings, use `.map(|s| s.replace('/', ':'))` on the result or use `record_ids_to_inside_value`.
#[must_use]
pub fn record_id_from_field(v: &Value, field_name: &str) -> Option<String> {
    let id_val = v.get(field_name)?;
    if let Some(s) = id_val.as_str() {
        return Some(normalize_record_id_string(s));
    }
    if let Some(tb) = id_val.get("tb").and_then(|x| x.as_str()) {
        if let Some(id_part) = id_val.get("id").and_then(json_id_part_to_string) {
            let key = id_part
                .trim_matches('`')
                .replace(ID_ANGLE_OPEN, "")
                .replace(ID_ANGLE_CLOSE, "");
            return Some(format!("{}/{}", tb, key));
        }
    }
    None
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

// ---------------------------------------------------------------------------
// Single-record lookup (one place for UUID/Thing/backtick handling)
// ---------------------------------------------------------------------------

/// Tables whose record id key is stored as UUID in SurrealDB (created with type::record('table', $key) where key is a UUID string).
const UUID_KEY_TABLES: &[&str] = &["contest", "player", "game"];

fn table_uses_uuid_key(table: &str) -> bool {
    UUID_KEY_TABLES.contains(&table)
}

/// Allowed table names for `select_one_by_record_id` (query safety).
const SELECT_ONE_TABLES: &[&str] = &["contest", "player", "game", "venue"];

fn table_allowed_for_select_one(table: &str) -> bool {
    SELECT_ONE_TABLES.contains(&table)
}

/// Fetch one row by canonical record id. Tries UUID lookup first for tables that use UUID keys, then Thing binding.
/// Use this for all single-record lookups so backticks, UUID vs string, and Thing binding are handled in one place.
/// Returns the first row as JSON or None. Table must be one of: contest, player, game, venue.
pub async fn select_one_by_record_id(
    db: &crate::db::Db,
    table: &str,
    id: &str,
) -> Option<serde_json::Value> {
    let key = record_id_to_key(id, table);
    if key.is_empty() || !table_allowed_for_select_one(table) {
        return None;
    }
    // Same as analytics/contest lookup: type::record(table, $key) with key as string (no type::uuid)
    let q = format!(
        "SELECT * FROM {} WHERE id = type::record('{}', $key) LIMIT 1",
        table, table
    );
    match db.query(&q).bind(("key", key.clone())).await {
        Ok(mut r) => {
            let rows: Vec<serde_json::Value> = r.take(0).unwrap_or_default();
            if let Some(row) = rows.into_iter().next() {
                return Some(row);
            }
            log::debug!(
                "select_one_by_record_id: type::record(table,$key) returned 0 rows for table={} key={}",
                table, key
            );
        }
        Err(e) => {
            log::debug!(
                "select_one_by_record_id: type::record(table,$key) query failed for table={} key={}: {}",
                table, key, e
            );
        }
    }
    // Try UUID-typed thing for tables that use UUID keys (some stores may use type::uuid)
    if table_uses_uuid_key(table) && uuid::Uuid::parse_str(&key).is_ok() {
        let q = format!(
            "SELECT * FROM {} WHERE id = type::record('{}', type::uuid($key)) LIMIT 1",
            table, table
        );
        if let Ok(mut r) = db.query(q).bind(("key", key.clone())).await {
            let rows: Vec<serde_json::Value> = r.take(0).unwrap_or_default();
            if let Some(row) = rows.into_iter().next() {
                return Some(row);
            }
        }
    }
    // Fallback: match list-query format — stored id string is "table:`key`"; strip backticks and compare to "table:key"
    let id_colon = format!("{}:{}", table, key);
    let q = format!(
        "SELECT * FROM {} WHERE {} = $id_colon LIMIT 1",
        table,
        SURREALQL_STRIP_BACKTICKS_ID
    );
    if let Ok(mut r) = db.query(q).bind(("id_colon", id_colon.clone())).await {
        let rows: Vec<serde_json::Value> = r.take(0).unwrap_or_default();
        if let Some(row) = rows.into_iter().next() {
            return Some(row);
        }
    }
    // Fallback: bind Thing for record-to-record comparison
    let rid = record_id_to_thing(id, table);
    let q = format!("SELECT * FROM {} WHERE id = $rid LIMIT 1", table);
    if let Ok(mut r) = db.query(q).bind(("rid", rid)).await {
        let rows: Vec<serde_json::Value> = r.take(0).unwrap_or_default();
        if let Some(row) = rows.into_iter().next() {
            return Some(row);
        }
    }
    // Fallback: full record id as string so server parses type::record("table:key") and compares
    let q = format!("SELECT * FROM {} WHERE id = type::record($id_colon) LIMIT 1", table);
    if let Ok(mut r) = db.query(q).bind(("id_colon", id_colon)).await {
        let rows: Vec<serde_json::Value> = r.take(0).unwrap_or_default();
        if let Some(row) = rows.into_iter().next() {
            return Some(row);
        }
    }
    // Fallback: normalize stored id to "table/key" and compare to canonical (handles backticks/angle brackets)
    let id_canonical = format!("{}/{}", table, key);
    let q = format!(
        "SELECT * FROM {} WHERE {} = $id_canonical LIMIT 1",
        table,
        SURREALQL_NORMALIZE_ID_FOR_COMPARE
    );
    if let Ok(mut r) = db.query(q).bind(("id_canonical", id_canonical)).await {
        let rows: Vec<serde_json::Value> = r.take(0).unwrap_or_default();
        if let Some(row) = rows.into_iter().next() {
            return Some(row);
        }
    }
    log::debug!(
        "select_one_by_record_id: no row for table={} id={} key={} (tried type::record(table,$key), type::uuid, strip_backticks, thing_rid, type::record(id_colon), normalize)",
        table, id, key
    );
    None
}
