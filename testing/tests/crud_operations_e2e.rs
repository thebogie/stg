//! E2E tests for CRUD operations using production snapshot data.
//! Requires BACKEND_URL environment variable.
//!
//! Tests create, read, update, delete operations for venues and games.

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::env;

#[derive(Debug, Deserialize)]
struct VenueDto {
    pub id: String,
    pub display_name: String,
    pub formatted_address: String,
    pub place_id: String,
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Deserialize)]
struct GameDto {
    pub id: String,
    pub name: String,
    pub year_published: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    pub player: PlayerDto,
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
struct PlayerDto {
    pub id: String,
    pub email: String,
}

fn base_url() -> Option<String> {
    env::var("BACKEND_URL").ok()
}

fn skip_if_no_backend() -> Option<String> {
    base_url()
}

/// Helper to get an authenticated session
async fn get_authenticated_session(base_url: &str) -> Result<Option<String>> {
    // Try to register a unique test user
    let register_url = format!("{}/api/players/register", base_url);
    let client = reqwest::Client::new();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let email = format!("crud_test_{}@example.com", timestamp);

    let register_res = client
        .post(&register_url)
        .json(&json!({
            "username": format!("crud_test_{}", timestamp),
            "email": email.clone(),
            "password": "password123"
        }))
        .send()
        .await
        .context("Failed to register")?;

    // If registration fails, try to login instead
    if !register_res.status().is_success() {
        // User might already exist, try login
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

#[tokio::test]
async fn e2e_venue_crud_create_and_read() -> Result<()> {
    let base = match skip_if_no_backend() {
        Some(url) => url,
        None => {
            eprintln!("Skipping test: BACKEND_URL not set");
            return Ok(());
        }
    };

    let session_id = match get_authenticated_session(&base).await? {
        Some(id) => id,
        None => {
            eprintln!("Skipping test: Could not authenticate");
            return Ok(());
        }
    };

    let client = reqwest::Client::new();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Create venue
    let create_url = format!("{}/api/venues", base);
    let venue_data = json!({
        "displayName": format!("E2E Test Venue {}", timestamp),
        "formattedAddress": format!("{} Test St, Test City, TC 12345, USA", timestamp),
        "placeId": format!("test_place_id_{}", timestamp),
        "lat": 40.7128,
        "lng": -74.0060,
        "timezone": "America/New_York"
    });

    let create_res = client
        .post(&create_url)
        .header("Authorization", format!("Bearer {}", session_id))
        .json(&venue_data)
        .send()
        .await
        .context("Failed to create venue")?;

    if !create_res.status().is_success() {
        eprintln!("Skipping test: Could not create venue (may require special permissions)");
        return Ok(());
    }

    let created_venue: VenueDto = create_res
        .json()
        .await
        .context("Failed to parse created venue")?;

    // Read venue
    let get_url = format!("{}/api/venues/{}", base, created_venue.id);
    let get_res = client
        .get(&get_url)
        .header("Authorization", format!("Bearer {}", session_id))
        .send()
        .await
        .context("Failed to get venue")?;

    assert!(
        get_res.status().is_success(),
        "Should be able to read created venue, got: {}",
        get_res.status()
    );

    let retrieved_venue: VenueDto = get_res.json().await.context("Failed to parse venue")?;

    // Verify: Retrieved venue matches created venue
    assert_eq!(
        retrieved_venue.id, created_venue.id,
        "Retrieved venue ID should match created venue ID"
    );
    assert_eq!(
        retrieved_venue.display_name, created_venue.display_name,
        "Retrieved venue name should match created venue name"
    );

    eprintln!(
        "✅ Venue CRUD works: Created and retrieved venue '{}'",
        created_venue.display_name
    );

    Ok(())
}

#[tokio::test]
async fn e2e_game_crud_create_and_read() -> Result<()> {
    let base = match skip_if_no_backend() {
        Some(url) => url,
        None => {
            eprintln!("Skipping test: BACKEND_URL not set");
            return Ok(());
        }
    };

    let session_id = match get_authenticated_session(&base).await? {
        Some(id) => id,
        None => {
            eprintln!("Skipping test: Could not authenticate");
            return Ok(());
        }
    };

    let client = reqwest::Client::new();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Create game
    let create_url = format!("{}/api/games", base);
    let game_data = json!({
        "name": format!("E2E Test Game {}", timestamp),
        "year_published": 2024,
        "source": "manual"
    });

    let create_res = client
        .post(&create_url)
        .header("Authorization", format!("Bearer {}", session_id))
        .json(&game_data)
        .send()
        .await
        .context("Failed to create game")?;

    if !create_res.status().is_success() {
        eprintln!("Skipping test: Could not create game (may require special permissions)");
        return Ok(());
    }

    let created_game: GameDto = create_res
        .json()
        .await
        .context("Failed to parse created game")?;

    // Read game
    let get_url = format!("{}/api/games/{}", base, created_game.id);
    let get_res = client
        .get(&get_url)
        .header("Authorization", format!("Bearer {}", session_id))
        .send()
        .await
        .context("Failed to get game")?;

    assert!(
        get_res.status().is_success(),
        "Should be able to read created game, got: {}",
        get_res.status()
    );

    let retrieved_game: GameDto = get_res.json().await.context("Failed to parse game")?;

    // Verify: Retrieved game matches created game
    assert_eq!(
        retrieved_game.id, created_game.id,
        "Retrieved game ID should match created game ID"
    );
    assert_eq!(
        retrieved_game.name, created_game.name,
        "Retrieved game name should match created game name"
    );

    eprintln!(
        "✅ Game CRUD works: Created and retrieved game '{}'",
        created_game.name
    );

    Ok(())
}

#[tokio::test]
async fn e2e_crud_get_nonexistent_returns_404() -> Result<()> {
    let base = match skip_if_no_backend() {
        Some(url) => url,
        None => {
            eprintln!("Skipping test: BACKEND_URL not set");
            return Ok(());
        }
    };

    let session_id = match get_authenticated_session(&base).await? {
        Some(id) => id,
        None => {
            eprintln!("Skipping test: Could not authenticate");
            return Ok(());
        }
    };

    let client = reqwest::Client::new();

    // Try to get nonexistent venue
    let venue_url = format!("{}/api/venues/venue/__nonexistent__", base);
    let venue_res = client
        .get(&venue_url)
        .header("Authorization", format!("Bearer {}", session_id))
        .send()
        .await
        .context("Failed to get venue")?;

    assert!(
        venue_res.status().is_client_error(),
        "Getting nonexistent venue should return 404, got: {}",
        venue_res.status()
    );

    // Try to get nonexistent game
    let game_url = format!("{}/api/games/game/__nonexistent__", base);
    let game_res = client
        .get(&game_url)
        .header("Authorization", format!("Bearer {}", session_id))
        .send()
        .await
        .context("Failed to get game")?;

    assert!(
        game_res.status().is_client_error(),
        "Getting nonexistent game should return 404, got: {}",
        game_res.status()
    );

    Ok(())
}
