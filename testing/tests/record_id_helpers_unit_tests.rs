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
        (
            "player:u'6454dd46-3ce2-44ab-ba69-e5d50e6a4d12'",
            "player/6454dd46-3ce2-44ab-ba69-e5d50e6a4d12",
        ),
        (
            "player/u'6454dd46-3ce2-44ab-ba69-e5d50e6a4d12'",
            "player/6454dd46-3ce2-44ab-ba69-e5d50e6a4d12",
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(normalize_record_id_string(input), expected, "input={input}");
    }
}

#[test]
fn record_id_to_key_strips_prefix_and_wrappers() {
    let venue_cases = vec![
        ("venue/abc", "abc"),
        ("venue:abc", "abc"),
        ("venue:`abc`", "abc"),
        ("venue:⟨abc⟩", "abc"),
        ("venue/⟨abc⟩", "abc"),
    ];
    for (id, expected_key) in venue_cases {
        assert_eq!(record_id_to_key(id, "venue"), expected_key, "id={id}");
    }
    let uuid = "6454dd46-3ce2-44ab-ba69-e5d50e6a4d12";
    assert_eq!(
        record_id_to_key(&format!("player:u'{uuid}'"), "player"),
        uuid
    );
    assert_eq!(
        record_id_to_key(&format!("player/u'{uuid}'"), "player"),
        uuid
    );
}

#[test]
fn record_id_from_row_extracts_string_id_and_normalizes() {
    let v = json!({ "id": "game:`⟨abc⟩`" });
    assert_eq!(record_id_from_row(&v, None).as_deref(), Some("game/abc"));
}

#[test]
fn record_id_from_row_extracts_thing_object_tb_id_string() {
    let v = json!({ "id": { "tb": "player", "id": "123" } });
    assert_eq!(record_id_from_row(&v, None).as_deref(), Some("player/123"));
}

#[test]
fn record_id_from_row_extracts_thing_object_tb_id_uuid_obj() {
    let v =
        json!({ "id": { "tb": "game", "id": { "uuid": "550e8400-e29b-41d4-a716-446655440000" } } });
    assert_eq!(
        record_id_from_row(&v, None).as_deref(),
        Some("game/550e8400-e29b-41d4-a716-446655440000")
    );
}
