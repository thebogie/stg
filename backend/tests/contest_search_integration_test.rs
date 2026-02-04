//! Integration tests for contest search endpoint.
//!
//! These tests require a running backend server. Set BACKEND_URL environment variable
//! to run these tests (e.g., `BACKEND_URL=http://localhost:8080 cargo test`).
//!
//! These tests are marked with #[ignore] by default. To run them:
//! `cargo test -- --ignored` or `cargo nextest run -- --ignored`

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::env;

#[derive(Debug, Deserialize)]
struct ContestSearchItemDto {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct ContestSearchResponseDto {
    pub items: Vec<ContestSearchItemDto>,
    pub total: u64,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

fn base_url() -> Result<String> {
    env::var("BACKEND_URL").context(
        "BACKEND_URL environment variable not set. These tests require a running backend server.",
    )
}

/// Helper to get an authenticated session for API tests
async fn get_authenticated_session(base_url: &str) -> Result<String> {
    #[derive(Debug, Deserialize)]
    struct LoginResponse {
        pub player: serde_json::Value,
        pub session_id: String,
    }

    let client = reqwest::Client::new();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let email = format!("contest_test_{}@example.com", timestamp);
    let username = format!("contest_test_{}", timestamp);

    // Try to register a new user
    let register_url = format!("{}/api/players/register", base_url);
    let register_res = client
        .post(&register_url)
        .json(&json!({
            "username": username,
            "email": email.clone(),
            "password": "password123"
        }))
        .send()
        .await
        .context("Failed to register")?;

    // If registration fails, try to login instead (user might already exist)
    if !register_res.status().is_success() {
        let login_url = format!("{}/api/players/login", base_url);
        let login_res = client
            .post(&login_url)
            .json(&json!({
                "email": email,
                "password": "password123"
            }))
            .send()
            .await
            .context("Failed to login")?;

        if login_res.status().is_success() {
            let login_body: LoginResponse =
                login_res.json().await.context("Failed to parse login")?;
            return Ok(login_body.session_id);
        }
        return Err(anyhow::anyhow!("Failed to register or login test user"));
    }

    // Login with newly registered user
    let login_url = format!("{}/api/players/login", base_url);
    let login_res = client
        .post(&login_url)
        .json(&json!({
            "email": email,
            "password": "password123"
        }))
        .send()
        .await
        .context("Failed to login")?;

    if login_res.status().is_success() {
        let login_body: LoginResponse = login_res.json().await.context("Failed to parse login")?;
        return Ok(login_body.session_id);
    }

    Err(anyhow::anyhow!("Failed to authenticate test user"))
}

#[tokio::test]
#[ignore] // Requires running backend server - run with `cargo test -- --ignored`
async fn contests_search_accepts_empty_query_and_returns_json() -> Result<()> {
    let base = base_url()?;
    let session_id = get_authenticated_session(&base).await?;
    let url = format!("{}/api/contests/search", base);

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", session_id))
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
        body.total >= body.items.len() as u64,
        "Total ({}) should be >= items length ({})",
        body.total,
        body.items.len()
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Requires running backend server
async fn contests_search_handles_game_ids_param() -> Result<()> {
    let base = base_url()?;
    let session_id = get_authenticated_session(&base).await?;
    let url = format!("{}/api/contests/search", base);

    // Use a clearly non-existent ID to ensure the backend does not 500 on parsing.
    let non_existent_game_id = "game/__does_not_exist__";
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", session_id))
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
        body.total >= body.items.len() as u64,
        "Total ({}) should be >= items length ({})",
        body.total,
        body.items.len()
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Requires running backend server
async fn contests_search_supports_date_range_params() -> Result<()> {
    let base = base_url()?;
    let session_id = get_authenticated_session(&base).await?;
    let url = format!("{}/api/contests/search", base);

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", session_id))
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
        body.total >= body.items.len() as u64,
        "Total ({}) should be >= items length ({})",
        body.total,
        body.items.len()
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Requires running backend server
async fn contests_search_supports_venue_param() -> Result<()> {
    let base = base_url()?;
    let session_id = get_authenticated_session(&base).await?;
    let url = format!("{}/api/contests/search", base);

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", session_id))
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
        body.total >= body.items.len() as u64,
        "Total ({}) should be >= items length ({})",
        body.total,
        body.items.len()
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Requires running backend server
async fn contests_search_supports_player_param() -> Result<()> {
    let base = base_url()?;
    let session_id = get_authenticated_session(&base).await?;
    let url = format!("{}/api/contests/search", base);

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", session_id))
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
        body.total >= body.items.len() as u64,
        "Total ({}) should be >= items length ({})",
        body.total,
        body.items.len()
    );

    Ok(())
}
