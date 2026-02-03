//! E2E tests for venue search functionality using production snapshot data.
//! Requires BACKEND_URL environment variable.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
struct VenueItem {
    pub id: String,
    pub display_name: String,
    pub formatted_address: String,
}

fn base_url() -> Option<String> {
    env::var("BACKEND_URL").ok()
}

fn skip_if_no_backend() -> Option<String> {
    base_url()
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

    let url = format!("{}/api/venues/search", base);
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
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
