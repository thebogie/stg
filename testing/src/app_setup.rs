//! Helper functions to set up the backend application for testing
//!
//! This module provides utilities to create a test Actix-web application
//! with real database and Redis connections from testcontainers.

use anyhow::{Context, Result};
use actix_web::{web, App};
use arangors::{Connection, Database};
use arangors::client::reqwest::ReqwestClient;
use backend::player::session::RedisSessionStore;
use backend::third_party::BGGService;
use backend::config::BGGConfig;
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
        match Connection::establish_basic_auth(
            env.arangodb_url(),
            "root",
            "test_password"
        ).await {
            Ok(c) => {
                conn = Some(c);
                break;
            }
            Err(e) if attempt < 4 => {
                log::warn!("Failed to connect to ArangoDB (attempt {}): {}, retrying...", attempt + 1, e);
                tokio::time::sleep(tokio::time::Duration::from_millis(500 * (attempt + 1) as u64)).await;
            }
            Err(e) => {
                return Err(e).context("Failed to connect to ArangoDB after retries");
            }
        }
    }
    let conn = conn.expect("Connection should be established");

    // Use _system database for tests (it always exists)
    let db: Database<ReqwestClient> = conn.db("_system")
        .await
        .context("Failed to access _system database")?;

    // Create collections if they don't exist
    // This is a minimal set for player tests - add more as needed
    let collections = vec!["player", "venue", "game", "contest", "player_contests", "player_performance"];
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

    // Connect to Redis
    let redis_client = redis::Client::open(env.redis_url())
        .context("Failed to create Redis client")?;
    let redis_data = web::Data::new(redis_client.clone());
    let session_store = web::Data::new(RedisSessionStore {
        client: redis_client.clone(),
    });

    // Create repositories
    let player_repo = web::Data::new(backend::player::repository::PlayerRepositoryImpl {
        db: db.clone(),
    });

    // Venue repository (no Google API key for tests)
    let venue_repo = web::Data::new(
        backend::venue::repository::VenueRepositoryImpl::new(db.clone(), None)
    );

    // Game repository with BGG service
    let bgg_service = BGGService::new_with_config(&BGGConfig {
        api_token: None,
        api_url: "http://localhost:8080".to_string(), // Mock URL for tests
    });
    let game_repo = web::Data::new(
        backend::game::repository::GameRepositoryImpl::new_with_bgg(db.clone(), bgg_service)
    );

    // Contest repository
    let contest_repo = web::Data::new(
        backend::contest::repository::ContestRepositoryImpl::new_with_google_config(db.clone(), None)
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

