//! Integration tests for venue update endpoint. Require BACKEND_BASE_URL and tokens to exercise auth paths.

use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
struct VenueDto {
    id: String,
    display_name: String,
    formatted_address: String,
    place_id: String,
    lat: f64,
    lng: f64,
    timezone: String,
    source: String,
}

fn base_url() -> Option<String> {
    env::var("BACKEND_URL").ok()
}
fn admin_token() -> Option<String> {
    env::var("ADMIN_BEARER_TOKEN").ok()
}
fn user_token() -> Option<String> {
    env::var("USER_BEARER_TOKEN").ok()
}

#[tokio::test]
async fn venue_update_requires_auth() {
    if base_url().is_none() {
        return;
    }
    let base = base_url().unwrap();
    let url = format!("{}/api/venues/{}", base, "__does_not_exist__");

    let body = VenueDto {
        id: "venue/__does_not_exist__".to_string(),
        display_name: "Test Venue".to_string(),
        formatted_address: "123 St".to_string(),
        place_id: "x".to_string(),
        lat: 0.0,
        lng: 0.0,
        timezone: "UTC".to_string(),
        source: "Database".to_string(),
    };

    let res = reqwest::Client::new()
        .put(&url)
        .json(&body)
        .send()
        .await
        .expect("request ok");
    assert!(res.status().as_u16() == 401 || res.status().as_u16() == 403);
}

#[tokio::test]
async fn venue_update_forbidden_for_non_admin() {
    if base_url().is_none() {
        return;
    }
    if user_token().is_none() {
        return;
    }
    let base = base_url().unwrap();
    let url = format!("{}/api/venues/{}", base, "__does_not_exist__");

    let body = VenueDto {
        id: "venue/__does_not_exist__".to_string(),
        display_name: "Test Venue".to_string(),
        formatted_address: "123 St".to_string(),
        place_id: "x".to_string(),
        lat: 0.0,
        lng: 0.0,
        timezone: "UTC".to_string(),
        source: "Database".to_string(),
    };

    let res = reqwest::Client::new()
        .put(&url)
        .bearer_auth(user_token().unwrap())
        .json(&body)
        .send()
        .await
        .expect("request ok");
    assert!(res.status().as_u16() == 403 || res.status().as_u16() == 404);
}

#[tokio::test]
async fn venue_update_admin_happy_path_shape() {
    if base_url().is_none() {
        return;
    }
    if admin_token().is_none() {
        return;
    }
    // This test only validates status and JSON shape, not persistence, since it uses a placeholder id.
    let base = base_url().unwrap();
    let url = format!("{}/api/venues/{}", base, "__does_not_exist__");

    let body = VenueDto {
        id: "venue/__does_not_exist__".to_string(),
        display_name: "Test Venue".to_string(),
        formatted_address: "123 St".to_string(),
        place_id: "x".to_string(),
        lat: 10.123,
        lng: -20.456,
        timezone: "UTC".to_string(),
        source: "Database".to_string(),
    };

    let res = reqwest::Client::new()
        .put(&url)
        .bearer_auth(admin_token().unwrap())
        .json(&body)
        .send()
        .await
        .expect("request ok");
    // Depending on whether the venue exists, could be 200 or 404; assert non-500 and JSON if 200
    assert!(!res.status().is_server_error());
    if res.status().is_success() {
        let updated: VenueDto = res.json().await.expect("json ok");
        assert!(!updated.display_name.is_empty());
    }
}
