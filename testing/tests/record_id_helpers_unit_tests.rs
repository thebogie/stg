//! Unit-style tests for record id normalization helpers.
//!
//! These are pure string/JSON-shape tests (no DB/network) but live in the `testing` crate
//! per project convention.

use backend::surreal_helpers::{normalize_record_id_string, record_id_from_row, record_id_to_key};
use serde_json::json;

#[test]
fn normalize_record_id_string_handles_colon_backticks_and_angles() {
    let cases = vec![
        ("game:abc", "game/abc"),
        ("game:`abc`", "game/abc"),
        ("game:⟨abc⟩", "game/abc"),
        ("game:`⟨abc⟩`", "game/abc"),
        ("game/abc", "game/abc"),
    ];
    for (input, expected) in cases {
        assert_eq!(normalize_record_id_string(input), expected, "input={input}");
    }
}

#[test]
fn record_id_to_key_strips_prefix_and_wrappers() {
    let table = "venue";
    let cases = vec![
        ("venue/abc", "abc"),
        ("venue:abc", "abc"),
        ("venue:`abc`", "abc"),
        ("venue:⟨abc⟩", "abc"),
        ("venue/⟨abc⟩", "abc"),
    ];
    for (id, expected_key) in cases {
        assert_eq!(record_id_to_key(id, table), expected_key, "id={id}");
    }
}

#[test]
fn record_id_from_row_extracts_string_id_and_normalizes() {
    let v = json!({ "id": "game:`⟨abc⟩`" });
    assert_eq!(
        record_id_from_row(&v, None).as_deref(),
        Some("game/abc")
    );
}

#[test]
fn record_id_from_row_extracts_thing_object_tb_id_string() {
    let v = json!({ "id": { "tb": "player", "id": "123" } });
    assert_eq!(
        record_id_from_row(&v, None).as_deref(),
        Some("player/123")
    );
}

#[test]
fn record_id_from_row_extracts_thing_object_tb_id_uuid_obj() {
    let v = json!({ "id": { "tb": "game", "id": { "uuid": "550e8400-e29b-41d4-a716-446655440000" } } });
    assert_eq!(
        record_id_from_row(&v, None).as_deref(),
        Some("game/550e8400-e29b-41d4-a716-446655440000")
    );
}

