//! Integration tests for contest search endpoint.
//!
//! These tests require a running backend server. Set BACKEND_URL environment variable
//! to run these tests (e.g., `BACKEND_URL=http://localhost:8080 cargo test`).
//!
//! These tests are marked with #[ignore] by default. To run them:
//! `cargo test -- --ignored` or `cargo nextest run -- --ignored`

use anyhow::{Context, Result};
use std::env;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ContestSearchItemDto {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct ContestSearchResponseDto {
    pub items: Vec<ContestSearchItemDto>,
    pub count: u64,
}

fn base_url() -> Result<String> {
    env::var("BACKEND_URL").context(
        "BACKEND_URL environment variable not set. These tests require a running backend server.",
    )
}

#[tokio::test]
#[ignore] // Requires running backend server - run with `cargo test -- --ignored`
async fn contests_search_accepts_empty_query_and_returns_json() -> Result<()> {
    let base = base_url()?;
    let url = format!("{}/api/contests/search", base);

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .query(&[("page", "1"), ("per_page", "10")])
        .send()
        .await
        .context("Failed to send request to backend")?;

    assert!(
        res.status().is_success(),
        "Unexpected status: {}. Response: {:?}",
        res.status(),
        res.text().await.unwrap_or_default()
    );

    let body: ContestSearchResponseDto =
        res.json().await.context("Failed to parse JSON response")?;
    assert!(
        body.count >= body.items.len() as u64,
        "Count ({}) should be >= items length ({})",
        body.count,
        body.items.len()
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Requires running backend server
async fn contests_search_handles_game_ids_param() -> Result<()> {
    let base = base_url()?;
    let url = format!("{}/api/contests/search", base);

    // Use a clearly non-existent ID to ensure the backend does not 500 on parsing.
    let non_existent_game_id = "game/__does_not_exist__";
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .query(&[
            ("game_ids", non_existent_game_id),
            ("page", "1"),
            ("per_page", "5"),
        ])
        .send()
        .await
        .context("Failed to send request to backend")?;

    assert!(
        res.status().is_success(),
        "Unexpected status: {}. Response: {:?}",
        res.status(),
        res.text().await.unwrap_or_default()
    );

    let body: ContestSearchResponseDto =
        res.json().await.context("Failed to parse JSON response")?;
    // With an invalid game id, zero or more items may be returned depending on data; only assert shape.
    assert!(
        body.count >= body.items.len() as u64,
        "Count ({}) should be >= items length ({})",
        body.count,
        body.items.len()
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Requires running backend server
async fn contests_search_supports_date_range_params() -> Result<()> {
    let base = base_url()?;
    let url = format!("{}/api/contests/search", base);

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .query(&[
            ("start_from", "2020-01-01T00:00:00Z"),
            ("start_to", "2100-01-01T00:00:00Z"),
            ("page", "1"),
            ("per_page", "5"),
        ])
        .send()
        .await
        .context("Failed to send request to backend")?;

    assert!(
        res.status().is_success(),
        "Unexpected status: {}. Response: {:?}",
        res.status(),
        res.text().await.unwrap_or_default()
    );

    let body: ContestSearchResponseDto =
        res.json().await.context("Failed to parse JSON response")?;
    assert!(
        body.count >= body.items.len() as u64,
        "Count ({}) should be >= items length ({})",
        body.count,
        body.items.len()
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Requires running backend server
async fn contests_search_supports_venue_param() -> Result<()> {
    let base = base_url()?;
    let url = format!("{}/api/contests/search", base);

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .query(&[
            ("venue_id", "venue/__does_not_exist__"),
            ("page", "1"),
            ("per_page", "5"),
        ])
        .send()
        .await
        .context("Failed to send request to backend")?;

    assert!(
        res.status().is_success(),
        "Unexpected status: {}. Response: {:?}",
        res.status(),
        res.text().await.unwrap_or_default()
    );

    let body: ContestSearchResponseDto =
        res.json().await.context("Failed to parse JSON response")?;
    assert!(
        body.count >= body.items.len() as u64,
        "Count ({}) should be >= items length ({})",
        body.count,
        body.items.len()
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Requires running backend server
async fn contests_search_supports_player_param() -> Result<()> {
    let base = base_url()?;
    let url = format!("{}/api/contests/search", base);

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .query(&[
            ("player_id", "player/__does_not_exist__"),
            ("page", "1"),
            ("per_page", "5"),
        ])
        .send()
        .await
        .context("Failed to send request to backend")?;

    assert!(
        res.status().is_success(),
        "Unexpected status: {}. Response: {:?}",
        res.status(),
        res.text().await.unwrap_or_default()
    );

    let body: ContestSearchResponseDto =
        res.json().await.context("Failed to parse JSON response")?;
    assert!(
        body.count >= body.items.len() as u64,
        "Count ({}) should be >= items length ({})",
        body.count,
        body.items.len()
    );

    Ok(())
}
