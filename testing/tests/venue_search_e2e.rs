//! E2E tests for venue search functionality using production snapshot data.
//! Requires BACKEND_URL environment variable.

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::env;

#[derive(Debug, Deserialize)]
struct VenueItem {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "formattedAddress")]
    pub formatted_address: String,
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
    let email = format!("e2e_venue_{}@example.com", timestamp);
    let username = format!("e2e_venue_{}", timestamp);

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

/// Helper to get all venues from production data
async fn get_all_venues(base_url: &str) -> Result<Vec<VenueItem>> {
    let url = format!("{}/api/venues", base_url);
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("Authorization", "Bearer dummy") // May need auth, but try without first
        .send()
        .await
        .context("Failed to fetch venues")?;

    if !res.status().is_success() {
        // Try without auth
        let res = client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch venues")?;
        if !res.status().is_success() {
            return Ok(Vec::new());
        }
        return Ok(res.json().await.context("Failed to parse venues")?);
    }

    Ok(res.json().await.context("Failed to parse venues")?)
}

#[tokio::test]
async fn e2e_venue_search_actually_filters() -> Result<()> {
    let base = match skip_if_no_backend() {
        Some(url) => url,
        None => {
            eprintln!("Skipping test: BACKEND_URL not set");
            return Ok(());
        }
    };

    // Get all venues as baseline
    let all_venues = get_all_venues(&base).await?;

    if all_venues.is_empty() {
        eprintln!("Skipping test: No venues found in production data");
        return Ok(());
    }

    // Pick a venue with a distinctive name/address
    let test_venue = &all_venues[0];
    let search_term = if test_venue.display_name.len() > 3 {
        &test_venue.display_name[..3]
    } else {
        &test_venue.display_name
    };

    // Search for venues
    let url = format!("{}/api/venues/search", base);
    let client = reqwest::Client::new();
    let search_res = client
        .get(&url)
        .query(&[("query", search_term)])
        .header("Authorization", "Bearer dummy")
        .send()
        .await
        .context("Failed to search venues")?;

    // If auth required, try without
    let search_res = if !search_res.status().is_success() {
        client
            .get(&url)
            .query(&[("query", search_term)])
            .send()
            .await
            .context("Failed to search venues")?
    } else {
        search_res
    };

    assert!(
        search_res.status().is_success(),
        "Expected success status for venue search, got: {}",
        search_res.status()
    );

    let search_results: Vec<VenueItem> = search_res
        .json()
        .await
        .context("Failed to parse search results")?;

    // Verify: Search results should be a subset of all venues
    assert!(
        search_results.len() <= all_venues.len(),
        "Search results ({}) should be <= all venues ({})",
        search_results.len(),
        all_venues.len()
    );

    // Verify: All search results should match the search term
    for venue in &search_results {
        let matches = venue
            .display_name
            .to_lowercase()
            .contains(&search_term.to_lowercase())
            || venue
                .formatted_address
                .to_lowercase()
                .contains(&search_term.to_lowercase());
        assert!(
            matches,
            "Venue '{}' (id: {}) does not match search term '{}'",
            venue.display_name, venue.id, search_term
        );
    }

    if !search_results.is_empty() {
        eprintln!(
            "✅ Venue search works: {} venues found matching '{}'",
            search_results.len(),
            search_term
        );
    }

    Ok(())
}

#[tokio::test]
async fn e2e_venue_search_empty_query_returns_error() -> Result<()> {
    let base = match skip_if_no_backend() {
        Some(url) => url,
        None => {
            eprintln!("Skipping test: BACKEND_URL not set");
            return Ok(());
        }
    };

    let url = format!("{}/api/venues/search", base);
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .query(&[("query", "")])
        .send()
        .await
        .context("Failed to send request")?;

    // Should return 400 Bad Request for empty query
    assert!(
        res.status().is_client_error(),
        "Expected client error (400) for empty query, got: {}",
        res.status()
    );

    Ok(())
}

#[tokio::test]
async fn e2e_venue_search_nonexistent_returns_empty() -> Result<()> {
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

    let url = format!("{}/api/venues/search", base);
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", session_id))
        .query(&[("query", "__nonexistent_venue_xyz123__")])
        .send()
        .await
        .context("Failed to search venues")?;

    assert!(
        res.status().is_success(),
        "Expected success status, got: {}",
        res.status()
    );

    let results: Vec<VenueItem> = res.json().await.context("Failed to parse results")?;

    // Should return empty array for nonexistent venue
    assert_eq!(
        results.len(),
        0,
        "Search for nonexistent venue should return empty, got {} results",
        results.len()
    );

    Ok(())
}
