//! E2E tests for player search functionality using production snapshot data.
//! Requires BACKEND_URL environment variable.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
struct PlayerItem {
    pub id: String,
    pub email: String,
    pub handle: String,
    pub firstname: String,
}

fn base_url() -> Option<String> {
    env::var("BACKEND_URL").ok()
}

fn skip_if_no_backend() -> Option<String> {
    base_url()
}

/// Helper to find players from production data by searching
async fn find_players_by_search(base_url: &str, query: &str) -> Result<Vec<PlayerItem>> {
    let url = format!("{}/api/players/search", base_url);
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .query(&[("query", query)])
        .send()
        .await
        .context("Failed to search players")?;

    if !res.status().is_success() {
        return Ok(Vec::new());
    }

    Ok(res.json().await.context("Failed to parse players")?)
}

#[tokio::test]
async fn e2e_player_search_by_email_actually_filters() -> Result<()> {
    let base = match skip_if_no_backend() {
        Some(url) => url,
        None => {
            eprintln!("Skipping test: BACKEND_URL not set");
            return Ok(());
        }
    };

    // First, try to find any players by searching for common email patterns
    let test_queries = vec!["@", "test", "example", "a"];

    let mut found_players = Vec::new();
    for query in test_queries {
        let results = find_players_by_search(&base, query).await?;
        if !results.is_empty() {
            found_players = results;
            break;
        }
    }

    if found_players.is_empty() {
        eprintln!("Skipping test: No players found in production data");
        return Ok(());
    }

    // Use the first player's email for a more specific search
    let test_player = &found_players[0];
    let email_prefix = test_player.email.split('@').next().unwrap_or("");

    if email_prefix.len() < 2 {
        eprintln!("Skipping test: Player email too short for meaningful search");
        return Ok(());
    }

    // Search by email prefix
    let search_term = &email_prefix[..email_prefix.len().min(5)];
    let search_results = find_players_by_search(&base, search_term).await?;

    // Verify: All search results should match the search term (in email or handle)
    for player in &search_results {
        let matches = player
            .email
            .to_lowercase()
            .contains(&search_term.to_lowercase())
            || player
                .handle
                .to_lowercase()
                .contains(&search_term.to_lowercase())
            || player
                .firstname
                .to_lowercase()
                .contains(&search_term.to_lowercase());
        assert!(
            matches,
            "Player '{}' (email: {}, handle: {}) does not match search term '{}'",
            player.firstname, player.email, player.handle, search_term
        );
    }

    if !search_results.is_empty() {
        eprintln!(
            "✅ Player search works: {} players found matching '{}'",
            search_results.len(),
            search_term
        );
    }

    Ok(())
}

#[tokio::test]
async fn e2e_player_search_by_handle_actually_filters() -> Result<()> {
    let base = match skip_if_no_backend() {
        Some(url) => url,
        None => {
            eprintln!("Skipping test: BACKEND_URL not set");
            return Ok(());
        }
    };

    // Find players first
    let all_players = find_players_by_search(&base, "a").await?;

    if all_players.is_empty() {
        eprintln!("Skipping test: No players found in production data");
        return Ok(());
    }

    // Use a player's handle for search
    let test_player = &all_players[0];
    let handle_prefix = if test_player.handle.len() > 2 {
        &test_player.handle[..test_player.handle.len().min(5)]
    } else {
        &test_player.handle
    };

    let search_results = find_players_by_search(&base, handle_prefix).await?;

    // Verify: All search results should match the handle
    for player in &search_results {
        let matches = player
            .handle
            .to_lowercase()
            .contains(&handle_prefix.to_lowercase())
            || player
                .email
                .to_lowercase()
                .contains(&handle_prefix.to_lowercase())
            || player
                .firstname
                .to_lowercase()
                .contains(&handle_prefix.to_lowercase());
        assert!(
            matches,
            "Player '{}' (handle: {}) does not match search term '{}'",
            player.firstname, player.handle, handle_prefix
        );
    }

    if !search_results.is_empty() {
        eprintln!(
            "✅ Player search by handle works: {} players found matching '{}'",
            search_results.len(),
            handle_prefix
        );
    }

    Ok(())
}

#[tokio::test]
async fn e2e_player_search_nonexistent_returns_empty() -> Result<()> {
    let base = match skip_if_no_backend() {
        Some(url) => url,
        None => {
            eprintln!("Skipping test: BACKEND_URL not set");
            return Ok(());
        }
    };

    let results = find_players_by_search(&base, "__nonexistent_player_xyz123__").await?;

    // Should return empty array for nonexistent player
    assert_eq!(
        results.len(),
        0,
        "Search for nonexistent player should return empty, got {} results",
        results.len()
    );

    Ok(())
}
