//! Integration tests for db-only search endpoints. Require BACKEND_BASE_URL.

use std::env;

#[derive(Debug, serde::Deserialize)]
struct GameDto {
    id: String,
    name: String,
}

#[derive(Debug, serde::Deserialize)]
struct VenueDto {
    id: String,
    display_name: String,
}

#[derive(Debug, serde::Deserialize)]
struct PlayerDto {
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
        .query(&[("q", "har")])
        .send()
        .await
        .expect("request ok");
    assert!(res.status().is_success());
    let _games: Vec<GameDto> = res.json().await.expect("json ok");
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
        .query(&[("q", "coffee")])
        .send()
        .await
        .expect("request ok");
    assert!(res.status().is_success());
    let _venues: Vec<VenueDto> = res.json().await.expect("json ok");
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
        .query(&[("q", "mit")])
        .send()
        .await
        .expect("request ok");
    assert!(res.status().is_success());
    let _players: Vec<PlayerDto> = res.json().await.expect("json ok");
}
