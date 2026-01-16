//! Helper functions to set up the backend application for testing
//!
//! This module provides utilities to create a test Actix-web application
//! with real database and Redis connections from testcontainers.

use actix_web::web;
use anyhow::{Context, Result};
use arangors::client::reqwest::ReqwestClient;
use arangors::{Connection, Database};
use backend::player::session::RedisSessionStore;
use reqwest::Client;
use std::sync::Arc;

use super::TestEnvironment;

/// Test application data for setting up the backend
#[derive(Clone)]
pub struct TestAppData {
    pub redis_data: web::Data<redis::Client>,
    pub player_repo: web::Data<backend::player::repository::PlayerRepositoryImpl>,
    pub venue_repo: web::Data<backend::venue::repository::VenueRepositoryImpl>,
    pub game_repo: web::Data<backend::game::repository::GameRepositoryImpl>,
    pub contest_repo: web::Data<backend::contest::repository::ContestRepositoryImpl>,
    pub session_store: web::Data<RedisSessionStore>,
    pub redis_arc: Arc<redis::Client>,
}

/// Set up test application data with testcontainers
///
/// This creates all the necessary data structures (repositories, Redis, etc.)
/// connected to the ephemeral containers from TestEnvironment.
pub async fn setup_test_app_data(env: &TestEnvironment) -> Result<TestAppData> {
    // Wait for containers to be ready
    env.wait_for_ready().await?;

    // Retry connection to ArangoDB with exponential backoff
    let mut conn = None;
    for attempt in 0..5 {
        match Connection::establish_basic_auth(env.arangodb_url(), "root", "test_password").await {
            Ok(c) => {
                conn = Some(c);
                break;
            }
            Err(e) if attempt < 4 => {
                log::warn!(
                    "Failed to connect to ArangoDB (attempt {}): {}, retrying...",
                    attempt + 1,
                    e
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    500 * (attempt + 1) as u64,
                ))
                .await;
            }
            Err(e) => {
                return Err(e).context("Failed to connect to ArangoDB after retries");
            }
        }
    }
    let conn = conn.expect("Connection should be established");

    // Use _system database for tests (it always exists)
    let db: Database<ReqwestClient> = conn
        .db("_system")
        .await
        .context("Failed to access _system database")?;

    // Create collections if they don't exist
    // This is a minimal set for player tests - add more as needed
    let collections = vec![
        "player",
        "venue",
        "game",
        "contest",
        "player_contests",
        "player_performance",
    ];
    for collection_name in collections {
        match db.collection(&collection_name).await {
            Ok(_) => {
                log::debug!("Collection {} already exists", collection_name);
            }
            Err(_) => {
                log::info!("Creating collection: {}", collection_name);
                db.create_collection(&collection_name)
                    .await
                    .with_context(|| format!("Failed to create collection {}", collection_name))?;
            }
        }
    }

    // Create edge collections for graph relationships
    // Edge collections need to be created via HTTP API with type: 3 (edge collection)
    let edge_collections = vec![
        "played_at",   // Contest -> Venue
        "played_with", // Contest -> Game
        "resulted_in", // Contest -> Player (for outcomes)
    ];

    // Get the database connection URL for HTTP requests
    let base_url = env.arangodb_url();
    let db_name = "_system";

    for edge_name in edge_collections {
        match db.collection(&edge_name).await {
            Ok(_) => {
                log::debug!("Edge collection {} already exists", edge_name);
            }
            Err(_) => {
                log::info!("Creating edge collection: {}", edge_name);
                // Create edge collection via HTTP API
                // ArangoDB edge collection type is 3
                let url = format!("{}/_db/{}/_api/collection", base_url, db_name);
                let payload = serde_json::json!({
                    "name": edge_name,
                    "type": 3  // Edge collection type
                });

                // Base64 encoding for auth
                use base64::Engine;
                let auth_val = "root:test_password";
                let auth_b64 = base64::engine::general_purpose::STANDARD.encode(auth_val);
                let auth_header = format!("Basic {}", auth_b64);

                match Client::new()
                    .post(&url)
                    .header("Authorization", auth_header)
                    .json(&payload)
                    .send()
                    .await
                {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            log::info!("✅ Created edge collection: {}", edge_name);
                        } else {
                            let status = resp.status();
                            let body = resp.text().await.unwrap_or_default();
                            log::warn!(
                                "Failed to create edge collection {}: {} - {}",
                                edge_name,
                                status,
                                body
                            );
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to create edge collection {}: {}", edge_name, e);
                    }
                }
            }
        }
    }

    // Connect to Redis
    let redis_client =
        redis::Client::open(env.redis_url()).context("Failed to create Redis client")?;
    let redis_data = web::Data::new(redis_client.clone());
    let session_store = web::Data::new(RedisSessionStore {
        client: redis_client.clone(),
    });

    // Create repositories
    let player_repo = web::Data::new(backend::player::repository::PlayerRepositoryImpl::new(
        db.clone(),
    ));

    // Venue repository (no Google API key for tests)
    let venue_repo = web::Data::new(backend::venue::repository::VenueRepositoryImpl::new(
        db.clone(),
        None,
    ));

    // Game repository setup
    // Default: No BGG service (to avoid external API calls in most tests)
    // For performance tests with small scenarios, real APIs can be enabled via env var
    let game_repo = if std::env::var("USE_REAL_BGG_API").is_ok() {
        use backend::config::BGGConfig;
        use backend::third_party::BGGService;
        let bgg_service = BGGService::new_with_config(&BGGConfig {
            api_url: std::env::var("BGG_API_URL")
                .unwrap_or_else(|_| "https://boardgamegeek.com/xmlapi2".to_string()),
            api_token: std::env::var("BGG_API_TOKEN").ok(),
        });
        log::info!("Using real BGG API for testing (small scenarios only)");
        web::Data::new(backend::game::repository::GameRepositoryImpl::new_with_bgg(
            db.clone(),
            bgg_service,
        ))
    } else {
        log::debug!("Using game repository without BGG service (database only)");
        web::Data::new(backend::game::repository::GameRepositoryImpl::new(
            db.clone(),
        ))
    };

    // Contest repository
    let contest_repo = web::Data::new(
        backend::contest::repository::ContestRepositoryImpl::new_with_google_config(
            db.clone(),
            None,
        ),
    );

    let redis_arc = Arc::new(redis_data.get_ref().clone());

    Ok(TestAppData {
        redis_data,
        player_repo,
        venue_repo,
        game_repo,
        contest_repo,
        session_store,
        redis_arc,
    })
}

// Note: App creation is done inline in tests to avoid complex type signatures
// The TestAppData struct provides all the necessary data for setting up the app
