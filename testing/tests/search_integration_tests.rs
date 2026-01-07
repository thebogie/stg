//! Integration tests for search functionality
//! Tests search endpoints for players, venues, games, and contests

use anyhow::Result;
use actix_web::{test, web, App};
use serde_json::json;
use testing::{TestEnvironment, app_setup};

#[tokio::test]
async fn test_search_players() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;
    
    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(web::scope("/api/players")
                .service(backend::player::controller::register_handler_prod)
                .service(backend::player::controller::search_players_handler)
            )
    ).await;

    // Create multiple players
    for i in 0..5 {
        let register_req = test::TestRequest::post()
            .uri("/api/players/register")
            .set_json(&json!({
                "username": format!("search_user_{}", i),
                "email": format!("search_{}@example.com", i),
                "password": "password123"
            }))
            .to_request();
        test::call_service(&app, register_req).await;
    }

    // Search for players
    let search_req = test::TestRequest::get()
        .uri("/api/players/search?query=search_user")
        .to_request();

    let mut search_resp = test::call_service(&app, search_req).await;
    let status = search_resp.status();
    if !status.is_success() {
        let body_bytes = test::read_body(search_resp).await;
        let body_text = String::from_utf8_lossy(&body_bytes);
        panic!(
            "Search games should succeed, got status: {}, body: {}",
            status,
            body_text
        );
    }
    let results: serde_json::Value = test::read_body_json(search_resp).await;
    // Results should contain search matches
    assert!(results.is_array() || results.get("results").is_some());

    Ok(())
}

#[tokio::test]
async fn test_search_venues() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;
    
    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(web::scope("/api/players")
                .service(backend::player::controller::register_handler_prod)
                .service(backend::player::controller::login_handler_prod)
            )
            .service(web::scope("/api/venues")
                .wrap(backend::auth::AuthMiddleware { 
                    redis: std::sync::Arc::new(app_data.redis_data.get_ref().clone()) 
                })
                .service(backend::venue::controller::create_venue_handler)
                .service(backend::venue::controller::search_venues_handler)
            )
    ).await;

    // Register and login
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "search_venue_user",
            "email": "search_venue@example.com",
            "password": "password123"
        }))
        .to_request();
    let register_resp = test::call_service(&app, register_req).await;
    assert!(
        register_resp.status().is_success(),
        "Registration should succeed, got status: {}",
        register_resp.status()
    );
    
    let login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": "search_venue@example.com",
            "password": "password123"
        }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    assert!(
        login_resp.status().is_success(),
        "Login should succeed, got status: {}",
        login_resp.status()
    );
    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let session_id = login_body["session_id"].as_str().expect("Login response should contain session_id");

    // Create multiple venues
    for i in 0..3 {
        let create_req = test::TestRequest::post()
            .uri("/api/venues")
            .insert_header(("Authorization", format!("Bearer {}", session_id)))
            .set_json(&json!({
                "displayName": format!("Search Venue {}", i),
                "formattedAddress": format!("{} Search St, Search City, SC 12345, USA", i),
                "placeId": format!("search_place_id_{}", i),
                "lat": 40.7128,
                "lng": -74.0060,
                "timezone": "America/New_York"
            }))
            .to_request();
        test::call_service(&app, create_req).await;
    }

    // Search venues
    let search_req = test::TestRequest::get()
        .uri("/api/venues/search?query=Search")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let mut search_resp = test::call_service(&app, search_req).await;
    let status = search_resp.status();
    if !status.is_success() {
        let body_bytes = test::read_body(search_resp).await;
        let body_text = String::from_utf8_lossy(&body_bytes);
        panic!(
            "Search games should succeed, got status: {}, body: {}",
            status,
            body_text
        );
    }
    let results: serde_json::Value = test::read_body_json(search_resp).await;
    assert!(results.is_array() || results.get("results").is_some());

    Ok(())
}

#[tokio::test]
async fn test_search_games() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;
    
    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(web::scope("/api/players")
                .service(backend::player::controller::register_handler_prod)
                .service(backend::player::controller::login_handler_prod)
            )
            .service(web::scope("/api/games")
                .wrap(backend::auth::AuthMiddleware { 
                    redis: std::sync::Arc::new(app_data.redis_data.get_ref().clone()) 
                })
                .service(backend::game::controller::create_game_handler)
                .service(backend::game::controller::search_games_handler)
            )
    ).await;

    // Register and login
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "search_game_user",
            "email": "search_game@example.com",
            "password": "password123"
        }))
        .to_request();
    let register_resp = test::call_service(&app, register_req).await;
    assert!(
        register_resp.status().is_success(),
        "Registration should succeed, got status: {}",
        register_resp.status()
    );
    
    let login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": "search_game@example.com",
            "password": "password123"
        }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    assert!(
        login_resp.status().is_success(),
        "Login should succeed, got status: {}",
        login_resp.status()
    );
    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let session_id = login_body["session_id"].as_str().expect("Login response should contain session_id");

    // Create multiple games
    for i in 0..3 {
        let create_req = test::TestRequest::post()
            .uri("/api/games")
            .insert_header(("Authorization", format!("Bearer {}", session_id)))
            .set_json(&json!({
                "name": format!("Search Game {}", i),
                "year_published": 2020 + i,
                "source": "bgg"
            }))
            .to_request();
        test::call_service(&app, create_req).await;
    }

    // Search games (uses 'query' parameter)
    let search_req = test::TestRequest::get()
        .uri("/api/games/search?query=Search")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let mut search_resp = test::call_service(&app, search_req).await;
    let status = search_resp.status();
    if !status.is_success() {
        let body_bytes = test::read_body(search_resp).await;
        let body_text = String::from_utf8_lossy(&body_bytes);
        panic!(
            "Search games should succeed, got status: {}, body: {}",
            status,
            body_text
        );
    }
    let results: serde_json::Value = test::read_body_json(search_resp).await;
    assert!(results.is_array() || results.get("results").is_some());

    Ok(())
}

#[tokio::test]
async fn test_search_empty_query() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;
    
    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(app_data.player_repo.clone())
            .service(web::scope("/api/players")
                .service(backend::player::controller::search_players_handler)
            )
    ).await;

    // Search with empty query
    let search_req = test::TestRequest::get()
        .uri("/api/players/search?query=")
        .to_request();

    let search_resp = test::call_service(&app, search_req).await;
    // Should handle empty query gracefully
    assert!(search_resp.status().is_success() || search_resp.status().is_client_error());

    Ok(())
}

#[tokio::test]
async fn test_search_special_characters() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;
    
    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(app_data.player_repo.clone())
            .service(web::scope("/api/players")
                .service(backend::player::controller::search_players_handler)
            )
    ).await;

    // Search with special characters
    let search_req = test::TestRequest::get()
        .uri("/api/players/search?query=test%26%26user")
        .to_request();

    let search_resp = test::call_service(&app, search_req).await;
    // Should handle special characters safely
    assert!(search_resp.status().is_success() || search_resp.status().is_client_error());

    Ok(())
}

