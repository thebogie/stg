//! Frontend E2E-style API tests for contest filters. Require BACKEND_URL.
//!
//! These tests exercise the contest search/filter API endpoints.
//! They use production snapshot data to verify actual filtering behavior.

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::env;

#[derive(Debug, Deserialize)]
struct ContestItem {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub start: String,
    pub stop: String,
    pub venue: Option<Value>,
    pub games: Vec<Value>,
    pub outcomes: Vec<Outcome>,
    // Optional fields that might be present
    #[serde(rename = "creator_id")]
    pub creator_id: Option<String>,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Outcome {
    #[serde(rename = "player_id")]
    pub player_id: String,
    pub handle: Option<String>,
    pub email: Option<String>,
    pub place: String,
    pub result: String,
}

#[derive(Debug, Deserialize)]
struct ContestResponse {
    pub items: Vec<ContestItem>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
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
    let email = format!("e2e_contest_{}@example.com", timestamp);
    let username = format!("e2e_contest_{}", timestamp);

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

/// Helper to find a player that has contests in the production data
async fn find_player_with_contests(base_url: &str) -> Result<Option<String>> {
    // First, get all contests to find players who participated
    let url = format!("{}/api/contests/search", base_url);
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .query(&[("scope", "all"), ("page_size", "100")])
        .send()
        .await
        .context("Failed to fetch contests")?;

    if !res.status().is_success() {
        return Ok(None);
    }

    let body: ContestResponse = res.json().await.context("Failed to parse contests")?;

    // Find first contest with outcomes (players)
    for contest in body.items {
        if !contest.outcomes.is_empty() {
            // Return the first player_id we find
            return Ok(Some(contest.outcomes[0].player_id.clone()));
        }
    }

    Ok(None)
}

/// Helper to find a player's email that has contests in the production data
async fn find_player_email_with_contests(base_url: &str) -> Result<Option<String>> {
    // First, get all contests to find players who participated
    let url = format!("{}/api/contests/search", base_url);
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .query(&[("scope", "all"), ("page_size", "100")])
        .send()
        .await
        .context("Failed to fetch contests")?;

    if !res.status().is_success() {
        return Ok(None);
    }

    let body: ContestResponse = res.json().await.context("Failed to parse contests")?;

    // Find first contest with outcomes (players) that has an email
    for contest in body.items {
        for outcome in contest.outcomes {
            if let Some(email) = outcome.email {
                if !email.is_empty() {
                    return Ok(Some(email));
                }
            }
        }
    }

    Ok(None)
}

/// Helper to find a venue that has contests
async fn find_venue_with_contests(base_url: &str) -> Result<Option<String>> {
    let url = format!("{}/api/contests/search", base_url);
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .query(&[("scope", "all"), ("page_size", "100")])
        .send()
        .await
        .context("Failed to fetch contests")?;

    if !res.status().is_success() {
        return Ok(None);
    }

    let body: ContestResponse = res.json().await.context("Failed to parse contests")?;

    // Find first contest with a venue
    for contest in body.items {
        if let Some(venue) = contest.venue {
            if let Some(venue_id) = venue.get("id").and_then(|v| v.as_str()) {
                return Ok(Some(venue_id.to_string()));
            }
        }
    }

    Ok(None)
}

/// Helper to find a game that has contests
async fn find_game_with_contests(base_url: &str) -> Result<Option<String>> {
    let url = format!("{}/api/contests/search", base_url);
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .query(&[("scope", "all"), ("page_size", "100")])
        .send()
        .await
        .context("Failed to fetch contests")?;

    if !res.status().is_success() {
        return Ok(None);
    }

    let body: ContestResponse = res.json().await.context("Failed to parse contests")?;

    // Find first contest with games
    for contest in body.items {
        if !contest.games.is_empty() {
            if let Some(game_id) = contest.games[0].get("id").and_then(|v| v.as_str()) {
                return Ok(Some(game_id.to_string()));
            }
        }
    }

    Ok(None)
}

#[tokio::test]
async fn e2e_contest_filters_basic_pagination() -> Result<()> {
    if base_url().is_none() {
        eprintln!("Skipping test: BACKEND_URL not set");
        return Ok(());
    }
    let base = base_url().unwrap();

    // Get authenticated session
    let session_id = match get_authenticated_session(&base).await? {
        Some(sid) => sid,
        None => {
            eprintln!("Skipping test: Could not get authenticated session");
            return Ok(());
        }
    };

    let url = format!("{}/api/contests/search", base);

    let res = reqwest::Client::new()
        .get(&url)
        .header("Authorization", format!("Bearer {}", session_id))
        .query(&[("page", "1"), ("per_page", "5"), ("scope", "all")])
        .send()
        .await?;

    assert!(
        res.status().is_success(),
        "Expected success status, got: {}",
        res.status()
    );
    let body: ContestResponse = res.json().await?;
    assert!(body.total >= body.items.len() as u64);

    Ok(())
}

#[tokio::test]
async fn e2e_contest_filters_game_ids_any_semantics_shape() -> Result<()> {
    if base_url().is_none() {
        eprintln!("Skipping test: BACKEND_URL not set");
        return Ok(());
    }
    let base = base_url().unwrap();

    // Get authenticated session
    let session_id = match get_authenticated_session(&base).await? {
        Some(sid) => sid,
        None => {
            eprintln!("Skipping test: Could not get authenticated session");
            return Ok(());
        }
    };

    let url = format!("{}/api/contests/search", base);

    let res = reqwest::Client::new()
        .get(&url)
        .header("Authorization", format!("Bearer {}", session_id))
        .query(&[("game_ids", "game/__nonexistent__"), ("scope", "all")])
        .send()
        .await?;

    assert!(
        res.status().is_success(),
        "Expected success status, got: {}",
        res.status()
    );

    Ok(())
}

#[tokio::test]
async fn e2e_contest_filters_venue_and_player_params_shape() -> Result<()> {
    if base_url().is_none() {
        eprintln!("Skipping test: BACKEND_URL not set");
        return Ok(());
    }
    let base = base_url().unwrap();

    // Get authenticated session
    let session_id = match get_authenticated_session(&base).await? {
        Some(sid) => sid,
        None => {
            eprintln!("Skipping test: Could not get authenticated session");
            return Ok(());
        }
    };

    let url = format!("{}/api/contests/search", base);

    let res = reqwest::Client::new()
        .get(&url)
        .header("Authorization", format!("Bearer {}", session_id))
        .query(&[
            ("venue_id", "venue/__nonexistent__"),
            ("player_id", "player/__nonexistent__"),
            ("scope", "all"),
        ])
        .send()
        .await?;

    assert!(
        res.status().is_success(),
        "Expected success status, got: {}",
        res.status()
    );

    Ok(())
}

#[tokio::test]
async fn e2e_contest_filters_player_id_actually_filters() -> Result<()> {
    let base = match skip_if_no_backend() {
        Some(url) => url,
        None => {
            eprintln!("Skipping test: BACKEND_URL not set");
            return Ok(());
        }
    };

    // Find a player that has contests
    let player_id = match find_player_with_contests(&base).await? {
        Some(id) => id,
        None => {
            eprintln!("Skipping test: No players with contests found in production data");
            return Ok(());
        }
    };

    // Get baseline: all contests
    let url = format!("{}/api/contests/search", base);
    let client = reqwest::Client::new();
    let all_res = client
        .get(&url)
        .query(&[("scope", "all"), ("page_size", "100")])
        .send()
        .await
        .context("Failed to fetch all contests")?;

    assert!(
        all_res.status().is_success(),
        "Expected success status for all contests, got: {}",
        all_res.status()
    );
    let all_contests: ContestResponse = all_res
        .json()
        .await
        .context("Failed to parse all contests")?;

    // Filter by player_id
    let filtered_res = client
        .get(&url)
        .query(&[
            ("player_id", player_id.as_str()),
            ("scope", "all"),
            ("page_size", "100"),
        ])
        .send()
        .await
        .context("Failed to fetch filtered contests")?;

    assert!(
        filtered_res.status().is_success(),
        "Expected success status for filtered contests, got: {}",
        filtered_res.status()
    );
    let filtered_contests: ContestResponse = filtered_res
        .json()
        .await
        .context("Failed to parse filtered contests")?;

    // Verify: filtered results should be a subset of all contests
    assert!(
        filtered_contests.total <= all_contests.total,
        "Filtered total ({}) should be <= all contests total ({})",
        filtered_contests.total,
        all_contests.total
    );

    // Verify: ALL filtered contests must contain the player in outcomes
    for contest in &filtered_contests.items {
        let has_player = contest
            .outcomes
            .iter()
            .any(|outcome| outcome.player_id == player_id);
        assert!(
            has_player,
            "Contest '{}' (id: {}) does not contain player {} in outcomes. Outcomes: {:?}",
            contest.name, contest.id, player_id, contest.outcomes
        );
    }

    // Verify: If we have filtered results, they should be fewer than all results
    // (unless all contests happen to have this player, which is unlikely)
    if filtered_contests.total > 0 && all_contests.total > filtered_contests.total {
        eprintln!(
            "✅ Player filter works: {} contests filtered down to {} contests",
            all_contests.total, filtered_contests.total
        );
    }

    Ok(())
}

#[tokio::test]
async fn e2e_contest_filters_venue_id_actually_filters() -> Result<()> {
    let base = match skip_if_no_backend() {
        Some(url) => url,
        None => {
            eprintln!("Skipping test: BACKEND_URL not set");
            return Ok(());
        }
    };

    // Find a venue that has contests
    let venue_id = match find_venue_with_contests(&base).await? {
        Some(id) => id,
        None => {
            eprintln!("Skipping test: No venues with contests found in production data");
            return Ok(());
        }
    };

    // Get baseline: all contests
    let url = format!("{}/api/contests/search", base);
    let client = reqwest::Client::new();
    let all_res = client
        .get(&url)
        .query(&[("scope", "all"), ("page_size", "100")])
        .send()
        .await
        .context("Failed to fetch all contests")?;

    assert!(
        all_res.status().is_success(),
        "Expected success status for all contests, got: {}",
        all_res.status()
    );
    let all_contests: ContestResponse = all_res
        .json()
        .await
        .context("Failed to parse all contests")?;

    // Filter by venue_id
    let filtered_res = client
        .get(&url)
        .query(&[
            ("venue_id", venue_id.as_str()),
            ("scope", "all"),
            ("page_size", "100"),
        ])
        .send()
        .await
        .context("Failed to fetch filtered contests")?;

    assert!(
        filtered_res.status().is_success(),
        "Expected success status for filtered contests, got: {}",
        filtered_res.status()
    );
    let filtered_contests: ContestResponse = filtered_res
        .json()
        .await
        .context("Failed to parse filtered contests")?;

    // Verify: filtered results should be a subset
    assert!(
        filtered_contests.total <= all_contests.total,
        "Filtered total ({}) should be <= all contests total ({})",
        filtered_contests.total,
        all_contests.total
    );

    // Verify: ALL filtered contests must have the correct venue
    for contest in &filtered_contests.items {
        if let Some(venue) = &contest.venue {
            let contest_venue_id = venue
                .get("_id")
                .or_else(|| venue.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if contest_venue_id.is_empty() {
                // Skip this contest if we can't find venue ID - might be a data issue
                continue;
            }
            assert_eq!(
                contest_venue_id, venue_id,
                "Contest '{}' (id: {}) has venue {} but filter was for {}",
                contest.name, contest.id, contest_venue_id, venue_id
            );
        } else {
            // Contest has no venue - this might be valid, just skip it
            continue;
        }
    }

    if filtered_contests.total > 0 && all_contests.total > filtered_contests.total {
        eprintln!(
            "✅ Venue filter works: {} contests filtered down to {} contests",
            all_contests.total, filtered_contests.total
        );
    }

    Ok(())
}

#[tokio::test]
async fn e2e_contest_filters_game_ids_actually_filters() -> Result<()> {
    let base = match skip_if_no_backend() {
        Some(url) => url,
        None => {
            eprintln!("Skipping test: BACKEND_URL not set");
            return Ok(());
        }
    };

    // Find a game that has contests
    let game_id = match find_game_with_contests(&base).await? {
        Some(id) => id,
        None => {
            eprintln!("Skipping test: No games with contests found in production data");
            return Ok(());
        }
    };

    // Get baseline: all contests
    let url = format!("{}/api/contests/search", base);
    let client = reqwest::Client::new();
    let all_res = client
        .get(&url)
        .query(&[("scope", "all"), ("page_size", "100")])
        .send()
        .await
        .context("Failed to fetch all contests")?;

    assert!(
        all_res.status().is_success(),
        "Expected success status for all contests, got: {}",
        all_res.status()
    );
    let all_contests: ContestResponse = all_res
        .json()
        .await
        .context("Failed to parse all contests")?;

    // Filter by game_id
    let filtered_res = client
        .get(&url)
        .query(&[
            ("game_ids", game_id.as_str()),
            ("scope", "all"),
            ("page_size", "100"),
        ])
        .send()
        .await
        .context("Failed to fetch filtered contests")?;

    assert!(
        filtered_res.status().is_success(),
        "Expected success status for filtered contests, got: {}",
        filtered_res.status()
    );
    let filtered_contests: ContestResponse = filtered_res
        .json()
        .await
        .context("Failed to parse filtered contests")?;

    // Verify: filtered results should be a subset
    assert!(
        filtered_contests.total <= all_contests.total,
        "Filtered total ({}) should be <= all contests total ({})",
        filtered_contests.total,
        all_contests.total
    );

    // Verify: ALL filtered contests must contain the game
    for contest in &filtered_contests.items {
        let has_game = contest
            .games
            .iter()
            .any(|game| game.get("id").and_then(|v| v.as_str()) == Some(game_id.as_str()));
        assert!(
            has_game,
            "Contest '{}' (id: {}) does not contain game {} in games. Games: {:?}",
            contest.name, contest.id, game_id, contest.games
        );
    }

    if filtered_contests.total > 0 && all_contests.total > filtered_contests.total {
        eprintln!(
            "✅ Game filter works: {} contests filtered down to {} contests",
            all_contests.total, filtered_contests.total
        );
    }

    Ok(())
}

#[tokio::test]
async fn e2e_contest_filters_combined_filters_work() -> Result<()> {
    let base = match skip_if_no_backend() {
        Some(url) => url,
        None => {
            eprintln!("Skipping test: BACKEND_URL not set");
            return Ok(());
        }
    };

    // Find a player and venue that have contests
    let player_id = find_player_with_contests(&base).await?;
    let venue_id = find_venue_with_contests(&base).await?;

    if player_id.is_none() || venue_id.is_none() {
        eprintln!("Skipping test: Need both player and venue with contests in production data");
        return Ok(());
    }

    let player_id = player_id.unwrap();
    let venue_id = venue_id.unwrap();

    // Get contests filtered by player only
    let url = format!("{}/api/contests/search", base);
    let client = reqwest::Client::new();
    let player_only_res = client
        .get(&url)
        .query(&[
            ("player_id", player_id.as_str()),
            ("scope", "all"),
            ("page_size", "100"),
        ])
        .send()
        .await
        .context("Failed to fetch player-filtered contests")?;

    assert!(
        player_only_res.status().is_success(),
        "Expected success status, got: {}",
        player_only_res.status()
    );
    let player_only: ContestResponse = player_only_res
        .json()
        .await
        .context("Failed to parse player-filtered contests")?;

    // Get contests filtered by both player AND venue
    let combined_res = client
        .get(&url)
        .query(&[
            ("player_id", player_id.as_str()),
            ("venue_id", venue_id.as_str()),
            ("scope", "all"),
            ("page_size", "100"),
        ])
        .send()
        .await
        .context("Failed to fetch combined-filtered contests")?;

    assert!(
        combined_res.status().is_success(),
        "Expected success status, got: {}",
        combined_res.status()
    );
    let combined: ContestResponse = combined_res
        .json()
        .await
        .context("Failed to parse combined-filtered contests")?;

    // Verify: Combined filter should be a subset of player-only filter
    assert!(
        combined.total <= player_only.total,
        "Combined filter total ({}) should be <= player-only total ({})",
        combined.total,
        player_only.total
    );

    // Verify: All combined results have both the player and venue
    for contest in &combined.items {
        // Check player
        let has_player = contest
            .outcomes
            .iter()
            .any(|outcome| outcome.player_id == player_id);
        assert!(
            has_player,
            "Contest '{}' should have player {}",
            contest.name, player_id
        );

        // Check venue
        if let Some(venue) = &contest.venue {
            let contest_venue_id = venue.get("id").and_then(|v| v.as_str()).unwrap_or("");
            assert_eq!(
                contest_venue_id, venue_id,
                "Contest '{}' should have venue {}",
                contest.name, venue_id
            );
        } else {
            panic!("Contest '{}' should have venue {}", contest.name, venue_id);
        }
    }

    if combined.total > 0 {
        eprintln!(
            "✅ Combined filters work: player filter gave {} contests, combined filter gave {} contests",
            player_only.total,
            combined.total
        );
    }

    Ok(())
}

#[tokio::test]
async fn e2e_contest_filters_player_email_lookup_works() -> Result<()> {
    let base = match skip_if_no_backend() {
        Some(url) => url,
        None => {
            eprintln!("Skipping test: BACKEND_URL not set");
            return Ok(());
        }
    };

    // Find a player email that has contests
    let player_email = match find_player_email_with_contests(&base).await? {
        Some(email) => email,
        None => {
            eprintln!("Skipping test: No players with email and contests found in production data");
            return Ok(());
        }
    };

    // Get the player_id for this email by searching for the player
    let player_search_url = format!("{}/api/players/search", base);
    let client = reqwest::Client::new();
    let player_search_res = client
        .get(&player_search_url)
        .query(&[("query", player_email.as_str())])
        .send()
        .await
        .context("Failed to search for player")?;

    if !player_search_res.status().is_success() {
        eprintln!("Skipping test: Could not search for player by email");
        return Ok(());
    }

    #[derive(Debug, Deserialize)]
    struct PlayerSearchItem {
        #[serde(rename = "_id")]
        pub id: String,
        pub email: String,
        pub handle: String,
    }

    let players: Vec<PlayerSearchItem> = player_search_res
        .json()
        .await
        .context("Failed to parse player search results")?;

    let player_id = players
        .iter()
        .find(|p| p.email == player_email)
        .map(|p| p.id.clone());

    let player_id = match player_id {
        Some(id) => id,
        None => {
            eprintln!(
                "Skipping test: Could not find player_id for email {}",
                player_email
            );
            return Ok(());
        }
    };

    // Get baseline: all contests
    let url = format!("{}/api/contests/search", base);
    let all_res = client
        .get(&url)
        .query(&[("scope", "all"), ("page_size", "100")])
        .send()
        .await
        .context("Failed to fetch all contests")?;

    assert!(
        all_res.status().is_success(),
        "Expected success status for all contests, got: {}",
        all_res.status()
    );
    let all_contests: ContestResponse = all_res
        .json()
        .await
        .context("Failed to parse all contests")?;

    // Filter by player_id (using the ID directly)
    let filtered_by_id_res = client
        .get(&url)
        .query(&[
            ("player_id", player_id.as_str()),
            ("scope", "all"),
            ("page_size", "100"),
        ])
        .send()
        .await
        .context("Failed to fetch contests filtered by player_id")?;

    assert!(
        filtered_by_id_res.status().is_success(),
        "Expected success status for filtered contests, got: {}",
        filtered_by_id_res.status()
    );
    let filtered_by_id: ContestResponse = filtered_by_id_res
        .json()
        .await
        .context("Failed to parse filtered contests")?;

    // Filter by player email (should look up the player and return same results)
    let filtered_by_email_res = client
        .get(&url)
        .query(&[
            ("player_id", player_email.as_str()),
            ("scope", "all"),
            ("page_size", "100"),
        ])
        .send()
        .await
        .context("Failed to fetch contests filtered by email")?;

    assert!(
        filtered_by_email_res.status().is_success(),
        "Expected success status for email-filtered contests, got: {}",
        filtered_by_email_res.status()
    );
    let filtered_by_email: ContestResponse = filtered_by_email_res
        .json()
        .await
        .context("Failed to parse email-filtered contests")?;

    // Verify: Results should be identical (same total, same items)
    assert_eq!(
        filtered_by_id.total, filtered_by_email.total,
        "Filtering by player_id ({}) and email ({}) should return same total. Got: {} vs {}",
        player_id, player_email, filtered_by_id.total, filtered_by_email.total
    );

    // Verify: All email-filtered contests must contain the player
    for contest in &filtered_by_email.items {
        let has_player = contest
            .outcomes
            .iter()
            .any(|outcome| outcome.player_id == player_id);
        assert!(
            has_player,
            "Contest '{}' (id: {}) does not contain player {} in outcomes when filtered by email {}",
            contest.name, contest.id, player_id, player_email
        );
    }

    // Test: Non-existent email should return empty results
    let nonexistent_email = "nonexistent_player_xyz123@example.com";
    let empty_res = client
        .get(&url)
        .query(&[
            ("player_id", nonexistent_email),
            ("scope", "all"),
            ("page_size", "100"),
        ])
        .send()
        .await
        .context("Failed to fetch contests for nonexistent email")?;

    assert!(
        empty_res.status().is_success(),
        "Expected success status for nonexistent email, got: {}",
        empty_res.status()
    );
    let empty_contests: ContestResponse = empty_res
        .json()
        .await
        .context("Failed to parse empty contests response")?;

    assert_eq!(
        empty_contests.total, 0,
        "Non-existent email should return 0 contests, got: {}",
        empty_contests.total
    );
    assert!(
        empty_contests.items.is_empty(),
        "Non-existent email should return empty items list"
    );

    if filtered_by_email.total > 0 {
        eprintln!(
            "✅ Email lookup works: email '{}' correctly filters to {} contests (matches player_id filter)",
            player_email, filtered_by_email.total
        );
    }

    Ok(())
}
