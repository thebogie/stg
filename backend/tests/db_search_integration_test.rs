//! Integration tests for db-only search endpoints. Require BACKEND_BASE_URL.

use std::env;

#[derive(Debug, serde::Deserialize)]
struct GameDto {
    #[serde(rename = "_id")]
    id: String,
    name: String,
}

#[derive(Debug, serde::Deserialize)]
struct VenueDto {
    #[serde(rename = "_id")]
    id: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Debug, serde::Deserialize)]
struct PlayerDto {
    #[serde(rename = "_id")]
    id: String,
    handle: String,
}

fn base_url() -> Option<String> {
    env::var("BACKEND_URL").ok()
}

fn skip() -> bool {
    base_url().is_none()
}

#[tokio::test]
async fn games_db_search_returns_success() {
    if skip() {
        return;
    }
    let base = base_url().unwrap();
    let url = format!("{}/api/games/db_search", base);
    let res = reqwest::Client::new()
        .get(&url)
        .query(&[("query", "har")])
        .send()
        .await
        .unwrap_or_else(|e| panic!("Failed to send request to {}: {}", url, e));
    assert!(
        res.status().is_success(),
        "Expected success status, got {} for {}",
        res.status(),
        url
    );
    let _games: Vec<GameDto> = res.json().await.unwrap_or_else(|e| {
        panic!("Failed to parse JSON response from {}: {}", url, e);
    });
}

#[tokio::test]
async fn venues_db_search_returns_success() {
    if skip() {
        return;
    }
    let base = base_url().unwrap();
    let url = format!("{}/api/venues/db_search", base);
    let res = reqwest::Client::new()
        .get(&url)
        .query(&[("query", "coffee")])
        .send()
        .await
        .unwrap_or_else(|e| panic!("Failed to send request to {}: {}", url, e));
    assert!(
        res.status().is_success(),
        "Expected success status, got {} for {}",
        res.status(),
        url
    );
    let _venues: Vec<VenueDto> = res.json().await.unwrap_or_else(|e| {
        panic!("Failed to parse JSON response from {}: {}", url, e);
    });
}

#[tokio::test]
async fn players_db_search_returns_success() {
    if skip() {
        return;
    }
    let base = base_url().unwrap();
    let url = format!("{}/api/players/db_search", base);
    let res = reqwest::Client::new()
        .get(&url)
        .query(&[("query", "mit")])
        .send()
        .await
        .unwrap_or_else(|e| panic!("Failed to send request to {}: {}", url, e));
    assert!(
        res.status().is_success(),
        "Expected success status, got {} for {}",
        res.status(),
        url
    );
    let _players: Vec<PlayerDto> = res.json().await.unwrap_or_else(|e| {
        panic!("Failed to parse JSON response from {}: {}", url, e);
    });
}
