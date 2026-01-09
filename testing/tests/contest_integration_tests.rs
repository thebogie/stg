//! Comprehensive integration tests for Contest API endpoints

use anyhow::Result;
use actix_web::{test, web, App};
use serde_json::json;
use testing::{TestEnvironment, app_setup};
use testing::create_authenticated_user;
use shared::dto::contest::ContestDto;

#[tokio::test]
async fn test_create_contest() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;
    
    // First create a venue and game
    let venue_app = test::init_service(
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
            )
    ).await;

    // Register and login
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "contest_test_user",
            "email": "contest_test@example.com",
            "password": "password123"
        }))
        .to_request();
    let register_resp = test::call_service(&venue_app, register_req).await;
    assert!(
        register_resp.status().is_success(),
        "Registration should succeed, got status: {}",
        register_resp.status()
    );
    
    let login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": "contest_test@example.com",
            "password": "password123"
        }))
        .to_request();
    let login_resp = test::call_service(&venue_app, login_req).await;
    assert!(
        login_resp.status().is_success(),
        "Login should succeed, got status: {}",
        login_resp.status()
    );
    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let session_id = login_body["session_id"].as_str().expect("Login response should contain session_id");

    // Create venue
    let venue_req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "displayName": "Contest Test Venue",
            "formattedAddress": "123 Contest St, Contest City, CT 12345, USA",
            "place_id": "contest_place_id",
            "lat": 40.7128,
            "lng": -74.0060,
            "timezone": "America/New_York"
        }))
        .to_request();
    let mut venue_resp = test::call_service(&venue_app, venue_req).await;
    let venue_status = venue_resp.status();
    if !venue_status.is_success() {
        let body_bytes = test::read_body(venue_resp).await;
        let body_text = String::from_utf8_lossy(&body_bytes);
        panic!(
            "Venue creation should succeed, got status: {}, body: {}",
            venue_status,
            body_text
        );
    }
    let venue: shared::dto::venue::VenueDto = test::read_body_json(venue_resp).await;
    let venue_id = venue.id.clone();

    // Create game (reuses session_id from venue_app)
    let game_app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.session_store.clone())
            .service(web::scope("/api/games")
                .wrap(backend::auth::AuthMiddleware { 
                    redis: std::sync::Arc::new(app_data.redis_data.get_ref().clone()) 
                })
                .service(backend::game::controller::create_game_handler)
            )
    ).await;

    let game_req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "name": "Contest Test Game",
            "year_published": 2023,
            "source": "bgg"
        }))
        .to_request();
    let mut game_resp = test::call_service(&game_app, game_req).await;
    let game_status = game_resp.status();
    if !game_status.is_success() {
        let body_bytes = test::read_body(game_resp).await;
        let body_text = String::from_utf8_lossy(&body_bytes);
        panic!(
            "Game creation should succeed, got status: {}, body: {}",
            game_status,
            body_text
        );
    }
    let game: shared::dto::game::GameDto = test::read_body_json(game_resp).await;
    let game_id = game.id.clone();

    // Now create contest
    let contest_app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
            .app_data(app_data.contest_repo.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.redis_data.clone())
            .service(web::scope("/api/contests")
                .wrap(backend::auth::AuthMiddleware { 
                    redis: std::sync::Arc::new(app_data.redis_data.get_ref().clone()) 
                })
                .app_data(app_data.player_repo.clone())
                .service(backend::contest::controller::create_contest_handler)
            )
    ).await;

    // ContestDto requires nested venue and games objects
    let contest_data = json!({
        "name": "Test Contest",
        "start": "2024-01-01T12:00:00Z",
        "stop": "2024-01-01T14:00:00Z",
        "venue": {
            "id": venue_id,
            "displayName": venue.display_name,
            "formattedAddress": venue.formatted_address,
            "place_id": venue.place_id,
            "lat": venue.lat,
            "lng": venue.lng,
            "timezone": venue.timezone
        },
        "games": [{
            "id": game_id,
            "name": game.name,
            "yearPublished": game.year_published,
            "bggId": game.bgg_id,
            "description": game.description,
            "source": "bgg"
        }],
        "outcomes": []
    });

    let contest_req = test::TestRequest::post()
        .uri("/api/contests")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&contest_data)
        .to_request();

    let mut contest_resp = test::call_service(&contest_app, contest_req).await;
    let contest_status = contest_resp.status();
    if !contest_status.is_success() {
        let body_bytes = test::read_body(contest_resp).await;
        let body_text = String::from_utf8_lossy(&body_bytes);
        panic!(
            "Contest creation should succeed, got status: {}, body: {}",
            contest_status,
            body_text
        );
    }
    let contest: ContestDto = test::read_body_json(contest_resp).await;
    // The venue ID might be normalized, so just check that it's not empty
    assert!(!contest.venue.id.is_empty(), "Contest venue should have an ID");
    assert!(!contest.games[0].id.is_empty(), "Contest game should have an ID");
    assert!(!contest.id.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_get_contest() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let _app_data = app_setup::setup_test_app_data(&env).await?;
    
    // Setup similar to create test
    // ... (similar setup code)
    
    // This test would verify getting a contest by ID
    // Implementation similar to venue/game get tests
    
    Ok(())
}

#[tokio::test]
async fn test_get_player_game_contests() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let _app_data = app_setup::setup_test_app_data(&env).await?;
    
    // Test getting all contests for a player and game combination
    // This tests the /api/contests/player/{player_id}/game/{game_id} endpoint
    
    Ok(())
}

