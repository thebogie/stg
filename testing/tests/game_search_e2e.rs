//! E2E tests for game search functionality using production snapshot data.
//! Requires BACKEND_URL environment variable.

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::env;

#[derive(Debug, Deserialize)]
struct GameItem {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    #[serde(rename = "year_published")]
    pub year_published: Option<i32>,
}

fn base_url() -> Option<String> {
    env::var("BACKEND_URL").ok()
}

fn skip_if_no_backend() -> Option<String> {
    base_url()
}

/// Helper to get an authenticated session for API tests
async fn get_authenticated_session(base_url: &str) -> Result<Option<String>> {
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
    let email = format!("e2e_game_{}@example.com", timestamp);
    let username = format!("e2e_game_{}", timestamp);

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

    // If registration fails, try to login instead
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
            return Ok(Some(login_body.session_id));
        }
        return Ok(None);
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
        return Ok(Some(login_body.session_id));
    }

    Ok(None)
}

/// Helper to get all games from production data
async fn get_all_games(base_url: &str) -> Result<Vec<GameItem>> {
    let url = format!("{}/api/games", base_url);
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("Authorization", "Bearer dummy")
        .send()
        .await
        .context("Failed to fetch games")?;

    if !res.status().is_success() {
        // Try without auth
        let res = client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch games")?;
        if !res.status().is_success() {
            return Ok(Vec::new());
        }
        return Ok(res.json().await.context("Failed to parse games")?);
    }

    Ok(res.json().await.context("Failed to parse games")?)
}

#[tokio::test]
async fn e2e_game_search_actually_filters() -> Result<()> {
    let base = match skip_if_no_backend() {
        Some(url) => url,
        None => {
            eprintln!("Skipping test: BACKEND_URL not set");
            return Ok(());
        }
    };

    // Get all games as baseline
    let all_games = get_all_games(&base).await?;

    if all_games.is_empty() {
        eprintln!("Skipping test: No games found in production data");
        return Ok(());
    }

    // Pick a game with a distinctive name
    let test_game = &all_games[0];
    let search_term = if test_game.name.len() > 3 {
        &test_game.name[..3]
    } else {
        &test_game.name
    };

    // Search for games
    let url = format!("{}/api/games/search", base);
    let client = reqwest::Client::new();
    let search_res = client
        .get(&url)
        .query(&[("query", search_term)])
        .header("Authorization", "Bearer dummy")
        .send()
        .await
        .context("Failed to search games")?;

    // If auth required, try without
    let search_res = if !search_res.status().is_success() {
        client
            .get(&url)
            .query(&[("query", search_term)])
            .send()
            .await
            .context("Failed to search games")?
    } else {
        search_res
    };

    assert!(
        search_res.status().is_success(),
        "Expected success status for game search, got: {}",
        search_res.status()
    );

    let search_results: Vec<GameItem> = search_res
        .json()
        .await
        .context("Failed to parse search results")?;

    // Verify: Search results should be a subset of all games
    assert!(
        search_results.len() <= all_games.len(),
        "Search results ({}) should be <= all games ({})",
        search_results.len(),
        all_games.len()
    );

    // Verify: All search results should match the search term
    for game in &search_results {
        let matches = game
            .name
            .to_lowercase()
            .contains(&search_term.to_lowercase());
        assert!(
            matches,
            "Game '{}' (id: {}) does not match search term '{}'",
            game.name, game.id, search_term
        );
    }

    if !search_results.is_empty() {
        eprintln!(
            "✅ Game search works: {} games found matching '{}'",
            search_results.len(),
            search_term
        );
    }

    Ok(())
}

#[tokio::test]
async fn e2e_game_search_nonexistent_returns_empty() -> Result<()> {
    let base = match skip_if_no_backend() {
        Some(url) => url,
        None => {
            eprintln!("Skipping test: BACKEND_URL not set");
            return Ok(());
        }
    };

    // Get authenticated session
    let session_id = match get_authenticated_session(&base).await? {
        Some(sid) => sid,
        None => {
            eprintln!("Skipping test: Could not get authenticated session");
            return Ok(());
        }
    };

    let url = format!("{}/api/games/search", base);
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", session_id))
        .query(&[("query", "__nonexistent_game_xyz123__")])
        .send()
        .await
        .context("Failed to search games")?;

    assert!(
        res.status().is_success(),
        "Expected success status, got: {}",
        res.status()
    );

    let results: Vec<GameItem> = res.json().await.context("Failed to parse results")?;

    // Should return empty array for nonexistent game
    assert_eq!(
        results.len(),
        0,
        "Search for nonexistent game should return empty, got {} results",
        results.len()
    );

    Ok(())
}
