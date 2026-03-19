//! Convert ArangoDB smacktalk dump (zip) to a SurrealDB .surql import file.
//!
//! **Conventions:** Output follows [docs/SURREALDB_ID_CONVENTIONS.md](../../../docs/SURREALDB_ID_CONVENTIONS.md)
//! and [docs/SURREALDB_EDGES.md](../../../docs/SURREALDB_EDGES.md). All record IDs are emitted as
//! `type::thing("table", "key")` with the **raw key** only (no table prefix, no backticks/angle brackets),
//! so the backend's `WHERE id = type::thing('player', $key)` and edge lookups work without a follow-up migration.
//!
//! Usage:
//!   arango-to-surreal path/to/smacktalk.zip [-o output.surql]
//!
//! By default emits full production schema (DEFINE TABLE/FIELD/INDEX) then INSERTs
//! for a one-time production migration. Use --no-schema for INSERTs only.
//!
//! Reads the ArangoDB dump format (dump.json + per-collection .data.json.gz),
//! converts document and edge collections to SurrealQL INSERT statements,
//! and writes a single .surql file suitable for `surreal import`.
//!
//! **Production mode** (default): schema + data + application functions (same crate’s surreal-functions.surql)
//! so the first import is complete. Later iterations use migration scripts for schema/function changes.

/// Application functions (contest/player/analytics one-round-trip). Lives in this crate; first convert is complete.
const SURREAL_FUNCTIONS: &str = include_str!("../surreal-functions.surql");

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use flate2::read::GzDecoder;
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::{fs, io};
use uuid::Uuid;
use zip::ZipArchive;

fn is_uuid_str(s: &str) -> bool {
    Uuid::parse_str(s.trim()).is_ok()
}

/// Table name -> (old_key -> new_uuid). When set, we rewrite record ids and all refs so relationships are preserved.
pub type IdMaps = HashMap<String, HashMap<String, String>>;

/// ID strategy: we translate *format* (Arango "collection/key" → Surreal "table:key", table lowercased)
/// and preserve the *key* so all references stay valid. Optional --remap-player-ids remaps only players
/// to new UUIDs. Optional --remap-all-ids remaps every document and edge to new UUIDs so the whole
/// graph uses Surreal-style opaque ids; all relationships (out/in, creator_id, player_id, etc.) are rewritten.

/// Arango _id is "collection/key". Surreal record id format for internal use is "table:key".
/// Per SURREALDB_ID_CONVENTIONS: canonical app format is "table/key"; we emit type::thing("table", raw_key).
fn arango_id_to_surreal(arango_id: &str) -> Cow<'_, str> {
    if let Some((table, key)) = arango_id.split_once('/') {
        Cow::Owned(format!("{}:{}", table.trim(), key.trim()))
    } else {
        Cow::Borrowed(arango_id)
    }
}

/// Strip SurrealDB key delimiters (backticks, angle brackets) so we emit raw key in type::thing("table", "key").
/// Per docs/SURREALDB_ID_CONVENTIONS: "We pass key only (no backticks/angle brackets)".
fn raw_key_for_thing(key: &str) -> String {
    key.trim()
        .trim_matches('`')
        .trim_matches('\u{27e8}') // ⟨
        .trim_matches('\u{27e9}') // ⟩
        .to_string()
}

/// If the value is a string that looks like "table/key" or "table:key", return type::thing SurrealQL; otherwise None.
/// When id_maps is Some, any ref whose table has a map gets its key rewritten so relationships are preserved.
fn record_ref_to_surql(v: &Value, id_maps: Option<&IdMaps>) -> Option<String> {
    let s = v.as_str()?;
    let s = s.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("null") {
        return None;
    }
    if !s.contains(':') && !s.contains('/') {
        return None;
    }
    let mut rid = arango_id_to_surreal(s).into_owned();
    if let Some(maps) = id_maps {
        if let Some((tb, key)) = rid.split_once(':') {
            let tb_low = tb.trim().to_ascii_lowercase();
            let key = key.trim();
            if let Some(new_id) = maps.get(&tb_low).and_then(|m| m.get(key)) {
                rid = format!("{}:{}", tb_low, new_id);
            }
        }
    }
    if rid.contains(':') {
        Some(record_id_literal_normalized(&rid, true))
    } else {
        None
    }
}

/// For document tables, known reference fields (table, field) -> target table for type::thing.
/// Backend expects these as record ids. Other fields stay as-is.
fn is_record_ref_field(table: &str, field: &str) -> bool {
    matches!(
        (table, field),
        ("contest", "creator_id")
            | ("rating_latest", "player_id")
            | ("rating_history", "player_id")
    )
}

/// Allowed document fields per table (production schema). Unknown fields are omitted so SCHEMAFULL import never fails.
fn allowed_document_field(table: &str, field: &str) -> bool {
    let f = field.to_ascii_lowercase();
    let allowed: &[&str] = match table {
        // IMPORTANT: do not allow a document's "id" field through; we always set Surreal's record id explicitly.
        "player" => &["firstname", "lastname", "handle", "email", "password", "createdat", "created_at", "isadmin", "is_admin", "accesstoken"],
        "game" => &["name", "year_published", "bgg_id", "description", "source", "createdat"],
        "venue" => &["display_name", "displayname", "formatted_address", "formattedaddress", "place_id", "lat", "lng", "timezone", "source"],
        "contest" => &["name", "start", "stop", "creator_id", "created_at", "startoffset"],
        // rating_* scope_id is a string in our backend (and schema); do not coerce to record id.
        "rating_latest" => &["player_id", "scope_type", "scope_id", "rating", "rd", "games_played", "updated_at", "last_period_end"],
        "rating_history" => &["player_id", "scope_type", "scope_id", "rating", "rd", "period_end", "created_at"],
        "schema_migrations" => &["appliedat", "name"],
        "migration_lock" => &[],
        _ => return true, // edge tables use different path; document tables not in list reject all
    };
    allowed.contains(&f.as_str())
}

/// Known datetime fields: (table, field). When value is string or number, emit type::datetime("...") so SurrealDB stores datetime type.
fn is_datetime_field(table: &str, field: &str) -> bool {
    matches!(
        (table, field),
        ("player", "createdAt")
            | ("player", "createdat")
            | ("game", "createdAt")
            | ("game", "createdat")
            | ("contest", "start")
            | ("contest", "stop")
            | ("contest", "created_at")
            | ("rating_latest", "updated_at")
            | ("rating_latest", "last_period_end")
            | ("rating_history", "created_at")
            | ("rating_history", "period_end")
            | ("schema_migrations", "appliedAt")
    )
}

/// Normalize a JSON value (string or number) to RFC3339 UTC for SurrealDB. Ensures consistent datetime storage and correct comparison with time::now().
fn normalize_datetime_value(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => {
            let s = s.trim();
            if s.is_empty() {
                return None;
            }
            // RFC3339 (with Z or +00:00, optional fractional seconds)
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return Some(dt.with_timezone(&Utc).to_rfc3339());
            }
            // ISO8601 with space instead of T
            if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
                let dt = Utc.from_utc_datetime(&naive);
                return Some(dt.to_rfc3339());
            }
            if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
                let dt = Utc.from_utc_datetime(&naive);
                return Some(dt.to_rfc3339());
            }
            // With fractional seconds, no TZ (treat as UTC)
            if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
                let dt = Utc.from_utc_datetime(&naive);
                return Some(dt.to_rfc3339());
            }
            if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
                let dt = Utc.from_utc_datetime(&naive);
                return Some(dt.to_rfc3339());
            }
            None
        }
        Value::Number(n) => {
            let t = n.as_f64().or_else(|| n.as_i64().map(|i| i as f64))?;
            // Assume milliseconds if >= 1e12, else seconds (Unix timestamp)
            let secs = if t >= 1e12 { (t / 1000.0) as i64 } else { t as i64 };
            Utc.timestamp_opt(secs, 0)
                .single()
                .map(|dt| dt.to_rfc3339())
        }
        _ => None,
    }
}

/// SurrealQL type::datetime("...") from a normalized RFC3339 string. Escapes for use inside double quotes.
fn datetime_to_surql(s: &str) -> String {
    format!(
        "type::datetime(\"{}\")",
        escape_surql_string(s.trim())
    )
}

/// Escape a string for use inside double-quoted SurrealQL string.
fn escape_surql_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

/// Serialize a JSON value to SurrealQL literal (for CONTENT / INSERT values).
fn value_to_surql(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("\"{}\"", escape_surql_string(s)),
        Value::Array(a) => {
            let inner: Vec<String> = a.iter().map(value_to_surql).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Object(o) => {
            let pairs: Vec<String> = o
                .iter()
                .map(|(k, val)| format!("{}: {}", escape_key(k), value_to_surql(val)))
                .collect();
            format!("{{ {} }}", pairs.join(", "))
        }
    }
}

fn escape_key(k: &str) -> String {
    if k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        k.to_string()
    } else {
        format!("\"{}\"", escape_surql_string(k))
    }
}

/// Prefer camelCase when the same key appears with different casing (e.g. createdat vs createdAt).
fn prefer_camel_case_key<'a>(a: &'a str, b: &'a str) -> &'a str {
    let a_camel = a
        .chars()
        .zip(a.chars().skip(1))
        .any(|(c, d)| c.is_ascii_lowercase() && d.is_ascii_uppercase());
    let b_camel = b
        .chars()
        .zip(b.chars().skip(1))
        .any(|(c, d)| c.is_ascii_lowercase() && d.is_ascii_uppercase());
    if a_camel && !b_camel {
        a
    } else if b_camel && !a_camel {
        b
    } else {
        a
    }
}

/// Build a SurrealQL record object: { id: type::thing("table", "key"), ...fields }.
/// When id_maps contains this table, use the mapped new key so all document ids are rewritten.
fn doc_to_surql_object(
    table: &str,
    doc: &serde_json::Map<String, Value>,
    id_maps: Option<&IdMaps>,
) -> String {
    let key = doc
        .get("_key")
        .map(arango_key_to_string)
        .unwrap_or_default();
    let effective_key: &str = id_maps
        .and_then(|m| m.get(table))
        .and_then(|m| m.get(&key))
        .map(String::as_str)
        .unwrap_or(&key);
    let id_val = record_id_literal(&format!("{}:{}", table, effective_key));
    let mut parts = vec![format!("id: {}", id_val)];

    // Dedupe keys by lowercase (Arango may have both createdat and createdAt); prefer camelCase.
    let mut by_lower: std::collections::HashMap<String, (String, &Value)> =
        std::collections::HashMap::new();
    for (k, v) in doc {
        if k == "_id" || k == "_key" || k == "_rev" || k == "_label" {
            continue;
        }
        let low = k.to_ascii_lowercase();
        match by_lower.get_mut(&low) {
            None => {
                by_lower.insert(low, (k.clone(), v));
            }
            Some((existing_k, existing_v)) => {
                let prefer = prefer_camel_case_key(k, existing_k);
                if prefer == k {
                    *existing_k = k.clone();
                    *existing_v = v;
                }
            }
        }
    }

    for (_, (k, v)) in by_lower {
        // Emit only schema-defined fields so SCHEMAFULL import never fails on unknown dump fields.
        if !allowed_document_field(table, &k) {
            continue;
        }
        // "id" is reserved for the record id (we already set it as the first part); never emit it as a normal field.
        if k.eq_ignore_ascii_case("id") {
            continue;
        }
        // Omit null values so SurrealDB option<T> fields accept "absent" instead of explicit null (avoids schema rejection).
        if v.is_null() {
            continue;
        }
        let val_surql = if is_record_ref_field(table, &k) {
            if let Some(thing) = record_ref_to_surql(v, id_maps) {
                thing
            } else {
                value_to_surql(v)
            }
        } else if is_datetime_field(table, &k) {
            if let Some(normalized) = normalize_datetime_value(v) {
                datetime_to_surql(&normalized)
            } else if let Some(s) = v.as_str() {
                datetime_to_surql(s)
            } else {
                value_to_surql(v)
            }
        } else {
            value_to_surql(v)
        };
        parts.push(format!("{}: {}", escape_key(&k), val_surql));
    }
    format!("{{ {} }}", parts.join(", "))
}

/// Document collection: convert one Arango doc to one Surreal record.
fn convert_document(
    table: &str,
    doc: &serde_json::Map<String, Value>,
    id_maps: Option<&IdMaps>,
) -> String {
    doc_to_surql_object(table, doc, id_maps)
}

/// Format a "table:key" record id as SurrealQL type::thing("table", "key") so SurrealDB stores a record id, not a string.
/// Uses raw key only (no backticks/angle brackets). Backend uses type::thing('table', $key) with raw key; indexes match.
fn record_id_literal(rid: &str) -> String {
    record_id_literal_normalized(rid, false)
}

/// Like record_id_literal but optionally lowercases the table name so edge refs (e.g. "Contest/123") match document tables ("contest").
/// Emits type::thing("table", "raw_key") per SURREALDB_ID_CONVENTIONS and SURREALDB_EDGES.
fn record_id_literal_normalized(rid: &str, lowercase_table: bool) -> String {
    match rid.split_once(':') {
        Some((tb, key)) => {
            let table = if lowercase_table {
                tb.trim().to_ascii_lowercase()
            } else {
                tb.trim().to_string()
            };
            let raw = raw_key_for_thing(key);
            if is_uuid_str(&raw) {
                format!(
                    "type::record(\"{}\", type::uuid(\"{}\"))",
                    escape_surql_string(&table),
                    escape_surql_string(&raw)
                )
            } else {
                format!(
                    "type::record(\"{}\", \"{}\")",
                    escape_surql_string(&table),
                    escape_surql_string(&raw)
                )
            }
        }
        None => format!("\"{}\"", escape_surql_string(rid)),
    }
}

/// Coerce Arango _key to string; dump may have _key as number (e.g. contest 10860534) so document id matches edge refs.
fn arango_key_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => String::new(),
        _ => v.to_string(),
    }
}

/// Apply id_maps to a single rid "table:key" (rewrite key if table has a map).
fn remap_rid(rid: &str, id_maps: Option<&IdMaps>) -> String {
    let rid = arango_id_to_surreal(rid).into_owned();
    match id_maps {
        None => rid,
        Some(maps) => {
            if let Some((tb, key)) = rid.split_once(':') {
                let tb_low = tb.trim().to_ascii_lowercase();
                let key = key.trim();
                if let Some(new_id) = maps.get(&tb_low).and_then(|m| m.get(key)) {
                    return format!("{}:{}", tb_low, new_id);
                }
            }
            rid
        }
    }
}

/// Edge collection: convert to relation row. out/in are rewritten via id_maps when set.
/// When remap_all_edge_ids, edge record id is a new UUID so every edge gets a Surreal-style id.
fn convert_edge(
    table: &str,
    doc: &serde_json::Map<String, Value>,
    id_maps: Option<&IdMaps>,
    remap_all_edge_ids: bool,
) -> String {
    let key = doc
        .get("_key")
        .map(arango_key_to_string)
        .unwrap_or_default();
    let from_str = doc.get("_from").map(arango_key_to_string).filter(|s| s.contains('/') || s.contains(':'));
    let to_str = doc.get("_to").map(arango_key_to_string).filter(|s| s.contains('/') || s.contains(':'));
    let from = from_str.as_ref().map(|s| remap_rid(s, id_maps));
    let to = to_str.as_ref().map(|s| remap_rid(s, id_maps));
    // IN = subject (_from), OUT = object (_to): [Subject]-in->(edge)-out->[Object]
    let in_val = from
        .as_ref()
        .map(|s| record_id_literal_normalized(s, true))
        .unwrap_or_else(|| "\"\"".to_string());
    let out_val = to
        .as_ref()
        .map(|s| record_id_literal_normalized(s, true))
        .unwrap_or_else(|| "\"\"".to_string());
    let edge_id_val = if remap_all_edge_ids {
        record_id_literal(&format!("{}:{}", table, Uuid::new_v4()))
    } else {
        record_id_literal(&format!("{}:{}", table, key))
    };
    // WARNING: Migration Note — SurrealQL reserves "in" and "out"; use backticks so INSERT parses correctly.
    let mut parts = vec![
        format!("id: {}", edge_id_val),
        format!("`out`: {}", out_val),
        format!("`in`: {}", in_val),
    ];
    for (k, v) in doc {
        if k == "_id" || k == "_key" || k == "_rev" || k == "_from" || k == "_to" || k == "_label" {
            continue;
        }
        if v.is_null() {
            continue;
        }
        parts.push(format!("{}: {}", escape_key(k), value_to_surql(v)));
    }
    format!("{{ {} }}", parts.join(", "))
}

const DOCUMENT_TABLES: &[&str] = &[
    "player",
    "game",
    "venue",
    "contest",
    "rating_latest",
    "rating_history",
    "schema_migrations",
    "migration_lock",
];

/// ArangoDB edge collections → SurrealDB relation tables. All must have _from (source) and _to (target).
/// Mapping: Arango _from/_to (collection/key) → Surreal `out`/`in` (type::thing). Table names in refs are lowercased.
///
/// | Arango collection | _from   | _to    | Surreal `out`   | Surreal `in`   |
/// |-------------------|---------|--------|-----------------|----------------|
/// | played_at         | contest | venue  | record<contest> (IN) | record<venue> (OUT)  |
/// | played_with       | contest | game   | record<contest> (IN) | record<game> (OUT)   |
/// | resulted_in       | contest | player | record<contest> (IN) | record<player> (OUT) |
const EDGE_TABLES: &[&str] = &["played_at", "played_with", "resulted_in"];

/// Emit full production schema: DEFINE TABLE SCHEMAFULL, DEFINE FIELD with types, DEFINE INDEX.
/// Use for one-time production migration so the next version of STG runs on SurrealDB with strict structural integrity.
fn emit_production_schema(w: &mut impl Write, v3_schema: bool) -> Result<()> {
    writeln!(w, "-- Production schema: 1:1 from ArangoDB with strict types and indexes.")?;
    if v3_schema {
        writeln!(w, "-- v3: do NOT DEFINE FIELD `id` as a normal field; Surreal record IDs are not plain strings.")?;
    }
    writeln!(w, "-- WARNING: Migration Note — Apply to empty namespace/database before INSERT.")?;
    writeln!(w)?;

    // Document tables: SCHEMAFULL + fields
    writeln!(w, "DEFINE TABLE player SCHEMAFULL;")?;
    writeln!(w, "DEFINE FIELD firstname ON player TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD lastname ON player TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD handle ON player TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD email ON player TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD password ON player TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD createdAt ON player TYPE option<datetime>;")?;
    writeln!(w, "DEFINE FIELD created_at ON player TYPE option<datetime>;")?;
    writeln!(w, "DEFINE FIELD isAdmin ON player TYPE option<bool>;")?;
    writeln!(w, "DEFINE FIELD is_admin ON player TYPE option<bool>;")?;
    writeln!(w, "DEFINE FIELD accessToken ON player TYPE option<string>;")?;
    writeln!(w, "DEFINE INDEX player_email ON player COLUMNS email;")?;
    writeln!(w)?;

    writeln!(w, "DEFINE TABLE game SCHEMAFULL;")?;
    writeln!(w, "DEFINE FIELD name ON game TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD year_published ON game TYPE option<int>;")?;
    writeln!(w, "DEFINE FIELD bgg_id ON game TYPE option<int>;")?;
    writeln!(w, "DEFINE FIELD description ON game TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD source ON game TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD createdAt ON game TYPE option<datetime>;")?;
    writeln!(w)?;

    writeln!(w, "DEFINE TABLE venue SCHEMAFULL;")?;
    writeln!(w, "DEFINE FIELD display_name ON venue TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD displayName ON venue TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD formatted_address ON venue TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD formattedAddress ON venue TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD place_id ON venue TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD lat ON venue TYPE option<float>;")?;
    writeln!(w, "DEFINE FIELD lng ON venue TYPE option<float>;")?;
    writeln!(w, "DEFINE FIELD timezone ON venue TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD source ON venue TYPE option<string>;")?;
    writeln!(w)?;

    writeln!(w, "DEFINE TABLE contest SCHEMAFULL;")?;
    writeln!(w, "DEFINE FIELD name ON contest TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD start ON contest TYPE option<datetime>;")?;
    writeln!(w, "DEFINE FIELD stop ON contest TYPE option<datetime>;")?;
    writeln!(w, "DEFINE FIELD creator_id ON contest TYPE option<record<player>>;")?;
    writeln!(w, "DEFINE FIELD created_at ON contest TYPE option<datetime>;")?;
    writeln!(w, "DEFINE FIELD startoffset ON contest TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD startOffset ON contest TYPE option<string>;")?;
    writeln!(w, "DEFINE INDEX contest_start ON contest COLUMNS start;")?;
    writeln!(w, "DEFINE INDEX contest_stop ON contest COLUMNS stop;")?;
    writeln!(w)?;

    writeln!(w, "DEFINE TABLE rating_latest SCHEMAFULL;")?;
    writeln!(w, "DEFINE FIELD player_id ON rating_latest TYPE option<record<player>>;")?;
    writeln!(w, "DEFINE FIELD scope_type ON rating_latest TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD scope_id ON rating_latest TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD rating ON rating_latest TYPE option<float>;")?;
    writeln!(w, "DEFINE FIELD rd ON rating_latest TYPE option<float>;")?;
    writeln!(w, "DEFINE FIELD volatility ON rating_latest TYPE option<float>;")?;
    writeln!(w, "DEFINE FIELD games_played ON rating_latest TYPE option<int>;")?;
    writeln!(w, "DEFINE FIELD updated_at ON rating_latest TYPE option<datetime>;")?;
    writeln!(w, "DEFINE FIELD last_period_end ON rating_latest TYPE option<datetime>;")?;
    writeln!(w, "DEFINE INDEX rating_latest_scope_player ON rating_latest COLUMNS scope_type, player_id, scope_id;")?;
    writeln!(w)?;

    writeln!(w, "DEFINE TABLE rating_history SCHEMAFULL;")?;
    writeln!(w, "DEFINE FIELD player_id ON rating_history TYPE option<record<player>>;")?;
    writeln!(w, "DEFINE FIELD scope_type ON rating_history TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD scope_id ON rating_history TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD period_end ON rating_history TYPE option<datetime>;")?;
    writeln!(w, "DEFINE FIELD rating ON rating_history TYPE option<float>;")?;
    writeln!(w, "DEFINE FIELD rd ON rating_history TYPE option<float>;")?;
    writeln!(w, "DEFINE FIELD volatility ON rating_history TYPE option<float>;")?;
    writeln!(w, "DEFINE FIELD period_games ON rating_history TYPE option<int>;")?;
    writeln!(w, "DEFINE FIELD wins ON rating_history TYPE option<int>;")?;
    writeln!(w, "DEFINE FIELD losses ON rating_history TYPE option<int>;")?;
    writeln!(w, "DEFINE FIELD draws ON rating_history TYPE option<int>;")?;
    writeln!(w, "DEFINE FIELD created_at ON rating_history TYPE option<datetime>;")?;
    writeln!(w, "DEFINE INDEX rating_history_player_scope ON rating_history COLUMNS player_id, scope_type;")?;
    writeln!(w, "DEFINE INDEX rating_history_period ON rating_history COLUMNS period_end;")?;
    writeln!(w)?;

    writeln!(w, "DEFINE TABLE schema_migrations SCHEMAFULL;")?;
    writeln!(w, "DEFINE FIELD appliedAt ON schema_migrations TYPE option<datetime>;")?;
    writeln!(w, "DEFINE FIELD name ON schema_migrations TYPE option<string>;")?;
    writeln!(w)?;

    writeln!(w, "DEFINE TABLE migration_lock SCHEMAFULL;")?;
    writeln!(w)?;

    // Edge/relation tables: out = source, in = target (Arango _from → out, _to → in). Use backticks for reserved words.
    writeln!(w, "DEFINE TABLE played_at SCHEMAFULL;")?;
    writeln!(w, "DEFINE FIELD `in` ON played_at TYPE record<contest>;")?;
    writeln!(w, "DEFINE FIELD `out` ON played_at TYPE record<venue>;")?;
    writeln!(w, "DEFINE INDEX played_at_in ON played_at COLUMNS `in`;")?;
    writeln!(w, "DEFINE INDEX played_at_out ON played_at COLUMNS `out`;")?;
    writeln!(w)?;

    writeln!(w, "DEFINE TABLE played_with SCHEMAFULL;")?;
    writeln!(w, "DEFINE FIELD `in` ON played_with TYPE record<contest>;")?;
    writeln!(w, "DEFINE FIELD `out` ON played_with TYPE record<game>;")?;
    writeln!(w, "DEFINE INDEX played_with_in ON played_with COLUMNS `in`;")?;
    writeln!(w, "DEFINE INDEX played_with_out ON played_with COLUMNS `out`;")?;
    writeln!(w)?;

    writeln!(w, "DEFINE TABLE resulted_in SCHEMAFULL;")?;
    writeln!(w, "DEFINE FIELD `in` ON resulted_in TYPE record<contest>;")?;
    writeln!(w, "DEFINE FIELD `out` ON resulted_in TYPE record<player>;")?;
    writeln!(w, "DEFINE FIELD place ON resulted_in TYPE option<int>;")?;
    writeln!(w, "DEFINE FIELD result ON resulted_in TYPE option<string>;")?;
    writeln!(w, "DEFINE FIELD points ON resulted_in TYPE option<int>;")?;
    writeln!(w, "DEFINE INDEX resulted_in_in ON resulted_in COLUMNS `in`;")?;
    writeln!(w, "DEFINE INDEX resulted_in_out ON resulted_in COLUMNS `out`;")?;
    writeln!(w)?;

    Ok(())
}

fn collection_kind(name: &str) -> Option<bool> {
    if DOCUMENT_TABLES.contains(&name) {
        return Some(false);
    }
    if EDGE_TABLES.contains(&name) {
        return Some(true);
    }
    None
}

/// Build old_key -> new_uuid map for one collection. Used for --remap-player-ids (one table) or --remap-all-ids (all doc tables).
fn build_table_id_map(gz_bytes: &[u8]) -> Result<HashMap<String, String>> {
    let dec = GzDecoder::new(gz_bytes);
    let reader = BufReader::new(dec);
    let mut map = HashMap::new();
    for line in reader.lines() {
        let line = line.context("read line")?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let doc: Value = serde_json::from_str(line).context("parse JSON")?;
        let obj = doc.as_object().context("expected JSON object")?;
        let key = obj
            .get("_key")
            .map(arango_key_to_string)
            .unwrap_or_default();
        if !key.is_empty() {
            map.insert(key, Uuid::new_v4().to_string());
        }
    }
    Ok(map)
}

/// Build IdMaps for the given tables from by_name (collection name -> gz bytes). Fails if a required table is missing.
fn build_id_maps(
    by_name: &HashMap<String, Vec<u8>>,
    tables: &[&str],
) -> Result<IdMaps> {
    let mut out = IdMaps::new();
    for &table in tables {
        let gz = by_name
            .get(table)
            .with_context(|| format!("remap requires {} collection in the dump", table))?;
        let map = build_table_id_map(gz)?;
        eprintln!("  Remapping {} {} keys to new UUIDs", map.len(), table);
        out.insert(table.to_string(), map);
    }
    Ok(out)
}

/// Process one collection: convert each doc/edge and write INSERT.
fn process_collection(
    table: &str,
    is_edge: bool,
    gz_bytes: &[u8],
    out: &mut impl Write,
    id_maps: Option<&IdMaps>,
    remap_all_ids: bool,
) -> Result<usize> {
    let dec = GzDecoder::new(gz_bytes);
    let reader = BufReader::new(dec);
    let mut rows = Vec::new();
    for line in reader.lines() {
        let line = line.context("read line")?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let doc: Value = serde_json::from_str(line).context("parse JSON line")?;
        let obj = doc.as_object().context("expected JSON object").cloned().unwrap_or_default();
        let row = if is_edge {
            convert_edge(table, &obj, id_maps, remap_all_ids)
        } else {
            convert_document(table, &obj, id_maps)
        };
        rows.push(row);
    }
    if rows.is_empty() {
        return Ok(0);
    }
    writeln!(out, "INSERT INTO {} [", table)?;
    for (i, row) in rows.iter().enumerate() {
        let suffix = if i == rows.len() - 1 { "" } else { "," };
        writeln!(out, "  {}{}", row, suffix)?;
    }
    writeln!(out, "];")?;
    Ok(rows.len())
}

fn run(
    zip_path: &Path,
    out_path: Option<&Path>,
    emit_schema: bool,
    production: bool,
    remap_player_ids: bool,
    remap_all_ids: bool,
) -> Result<()> {
    let zip_bytes = fs::read(zip_path).with_context(|| format!("read zip: {}", zip_path.display()))?;
    let mut zip = ZipArchive::new(io::Cursor::new(zip_bytes)).context("open zip")?;

    let default_out = zip_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(zip_path.file_stem().unwrap_or(Default::default()))
        .with_extension("surql");
    let out_path = out_path.unwrap_or(&default_out);

    let mut collections: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..zip.len() {
        let name = zip.by_index(i).context("zip entry")?.name().to_string();
        if !name.ends_with(".data.json.gz") {
            continue;
        }
        let base = Path::new(&name)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let stem = base.strip_suffix(".data.json.gz").unwrap_or(base);
        let raw = stem
            .rsplit_once('_')
            .map(|(c, _)| c.to_string())
            .unwrap_or_else(|| stem.to_string());
        let collection = raw.to_ascii_lowercase();
        if collection_kind(&collection).is_none() {
            continue;
        }
        let mut buf = Vec::new();
        let mut entry = zip.by_index(i).context("zip entry")?;
        io::copy(&mut entry, &mut buf).context("read zip entry")?;
        collections.push((collection, buf));
    }

    // Order: document tables first, then edge tables
    let order: Vec<String> = DOCUMENT_TABLES
        .iter()
        .copied()
        .chain(EDGE_TABLES.iter().copied())
        .map(str::to_string)
        .collect();
    let mut by_name: std::collections::HashMap<String, Vec<u8>> = collections.into_iter().collect();
    let mut w = fs::File::create(out_path).with_context(|| format!("create output: {}", out_path.display()))?;

    writeln!(w, "-- Generated by arango-to-surreal from {}", zip_path.display())?;
    writeln!(w, "-- Use (v3): surreal import --endpoint <url> --namespace <ns> --database <db> --username root --password <pass> {}", out_path.display())?;
    if production {
        writeln!(w, "-- Production schema: DEFINE TABLE/FIELD/INDEX for strict 1:1 migration.")?;
    }
    if remap_all_ids {
        writeln!(w, "-- All record IDs remapped to new UUIDs (--remap-all-ids); relationships preserved.")?;
    } else if remap_player_ids {
        writeln!(w, "-- Player IDs remapped to new UUIDs (--remap-player-ids).")?;
    }
    writeln!(w)?;

    let id_maps: Option<IdMaps> = if remap_all_ids {
        eprintln!("Building ID maps for all document tables...");
        Some(build_id_maps(&by_name, DOCUMENT_TABLES)?)
    } else if remap_player_ids {
        eprintln!("Building ID map for player table...");
        Some(build_id_maps(&by_name, &["player"])?)
    } else {
        None
    };
    let id_maps_ref = id_maps.as_ref();

    if production {
        emit_production_schema(&mut w, true)?; // v3: id fields TYPE string
    } else if emit_schema {
        for table in &order {
            writeln!(w, "DEFINE TABLE {} SCHEMAFULL;", table)?;
        }
        writeln!(w)?;
    }

    let mut total = 0usize;
    for table in &order {
        let Some(gz) = by_name.remove(table) else { continue };
        let is_edge = collection_kind(table).unwrap();
        let count = process_collection(table, is_edge, &gz, &mut w, id_maps_ref, remap_all_ids)?;
        total += count;
        eprintln!("  {}: {} records", table, count);
    }

    if production {
        writeln!(w)?;
        writeln!(w, "-- Application functions (contest/player/analytics one-round-trip). First convert includes these; later use migration scripts for changes.")?;
        writeln!(w)?;
        w.write_all(SURREAL_FUNCTIONS.as_bytes())
            .context("write surreal-functions to output")?;
        writeln!(w)?;
        eprintln!("  + application functions (fn::contest_row, fn::contest_with_edges, etc.)");
    }

    eprintln!("Total: {} records written to {}", total, out_path.display());
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut zip_path: Option<&Path> = None;
    let mut out_path: Option<&Path> = None;
    let mut emit_schema = false;
    let mut production = true; // default: emit full schema + data for one-time production migration
    let mut remap_player_ids = false;
    let mut remap_all_ids = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                if i < args.len() {
                    out_path = Some(Path::new(&args[i]));
                    i += 1;
                }
            }
            "--schema" => {
                emit_schema = true;
                production = false;
                i += 1;
            }
            "--no-schema" => {
                production = false;
                i += 1;
            }
            "--production" => {
                production = true;
                i += 1;
            }
            "--remap-player-ids" => {
                remap_player_ids = true;
                i += 1;
            }
            "--remap-all-ids" => {
                remap_all_ids = true;
                i += 1;
            }
            s if !s.starts_with('-') => {
                zip_path = Some(Path::new(s));
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    let zip_path = match zip_path {
        Some(p) => p,
        None => {
            eprintln!("Usage: arango-to-surreal <smacktalk.zip> [-o output.surql] [--no-schema | --schema] [--remap-player-ids | --remap-all-ids]");
            eprintln!("  By default: emit full schema (DEFINE TABLE/FIELD/INDEX) then INSERTs for one-time production migration.");
            eprintln!("  --no-schema   Emit INSERTs only (no DEFINE statements).");
            eprintln!("  --schema      Emit minimal DEFINE TABLE only (no fields/indexes, no data).");
            eprintln!("  --remap-player-ids  New UUID per player; rewrites player refs only.");
            eprintln!("  --remap-all-ids     New UUID per document and edge; rewrites all refs (relationships preserved).");
            std::process::exit(1);
        }
    };
    if !zip_path.exists() {
        anyhow::bail!("File not found: {}", zip_path.display());
    }
    run(zip_path, out_path, emit_schema, production, remap_player_ids, remap_all_ids)
}
