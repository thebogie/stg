//! Comprehensive integration tests for Venue API endpoints
//! Tests CRUD operations, search, validation, and error handling

use anyhow::Result;
use actix_web::{test, web, App};
use serde_json::json;
use testing::{TestEnvironment, app_setup};
use testing::create_authenticated_user;
use shared::dto::venue::VenueDto;

#[tokio::test]
async fn test_create_venue() -> Result<()> {
    let env = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        TestEnvironment::new()
    ).await
    .map_err(|_| anyhow::anyhow!("Test environment setup timed out"))??;
    
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
            .app_data(app_data.venue_repo.clone())
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

    // First, register and login to get auth token
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "venue_test_user",
            "email": "venue_test@example.com",
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
            "email": "venue_test@example.com",
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

    // Create venue with authentication
    let venue_data = json!({
        "displayName": "Test Venue",
        "formattedAddress": "123 Test St, Test City, TS 12345, USA",
        "place_id": "test_place_id",
        "lat": 40.7128,
        "lng": -74.0060,
        "timezone": "America/New_York"
    });

    let req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&venue_data)
        .to_request();

    let mut resp = test::call_service(&app, req).await;
    let status = resp.status();
    if !status.is_success() {
        let body_bytes = test::read_body(resp).await;
        let body_text = String::from_utf8_lossy(&body_bytes);
        panic!(
            "Venue creation should succeed, got status: {}, body: {}",
            status,
            body_text
        );
    }
    let venue: VenueDto = test::read_body_json(resp).await;
    assert_eq!(venue.display_name, "Test Venue");
    assert_eq!(venue.formatted_address, "123 Test St, Test City, TS 12345, USA");
    assert!(!venue.id.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_get_venue() -> Result<()> {
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
            .app_data(app_data.venue_repo.clone())
            .service(web::scope("/api/players")
                .service(backend::player::controller::register_handler_prod)
                .service(backend::player::controller::login_handler_prod)
            )
            .service(web::scope("/api/venues")
                .wrap(backend::auth::AuthMiddleware { 
                    redis: std::sync::Arc::new(app_data.redis_data.get_ref().clone()) 
                })
                .service(backend::venue::controller::create_venue_handler)
                .service(backend::venue::controller::get_venue_handler)
            )
    ).await;

    // Register, login, create venue
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "get_venue_user",
            "email": "get_venue@example.com",
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
            "email": "get_venue@example.com",
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

    let create_req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "displayName": "Get Test Venue",
            "formattedAddress": "456 Get St, Get City, GT 54321, USA",
            "place_id": "get_place_id",
            "lat": 34.0522,
            "lng": -118.2437,
            "timezone": "America/Los_Angeles"
        }))
        .to_request();
    
    let mut create_resp = test::call_service(&app, create_req).await;
    let create_status = create_resp.status();
    if !create_status.is_success() {
        let body_bytes = test::read_body(create_resp).await;
        let body_text = String::from_utf8_lossy(&body_bytes);
        panic!(
            "Venue creation should succeed, got status: {}, body: {}",
            create_status,
            body_text
        );
    }
    let venue: VenueDto = test::read_body_json(create_resp).await;
    let venue_id = venue.id.clone();
    // Extract just the key part (after "/") for the URL path
    let venue_key = venue_id.split_once('/').map(|(_, k)| k).unwrap_or(&venue_id);

    // Get venue by ID (can use either full ID or just key)
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/venues/{}", venue_key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let mut get_resp = test::call_service(&app, get_req).await;
    let get_status = get_resp.status();
    if !get_status.is_success() {
        let body_bytes = test::read_body(get_resp).await;
        let body_text = String::from_utf8_lossy(&body_bytes);
        panic!(
            "Get venue should succeed, got status: {}, body: {}",
            get_status,
            body_text
        );
    }
    let retrieved: VenueDto = test::read_body_json(get_resp).await;
    assert_eq!(retrieved.id, venue_id);
    assert_eq!(retrieved.display_name, "Get Test Venue");

    Ok(())
}

#[tokio::test]
async fn test_get_all_venues() -> Result<()> {
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
            .app_data(app_data.venue_repo.clone())
            .service(web::scope("/api/players")
                .service(backend::player::controller::register_handler_prod)
                .service(backend::player::controller::login_handler_prod)
            )
            .service(web::scope("/api/venues")
                .wrap(backend::auth::AuthMiddleware { 
                    redis: std::sync::Arc::new(app_data.redis_data.get_ref().clone()) 
                })
                .service(backend::venue::controller::create_venue_handler)
                .service(backend::venue::controller::get_all_venues_handler)
            )
    ).await;

    // Register and login
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "all_venues_user",
            "email": "all_venues@example.com",
            "password": "password123"
        }))
        .to_request();
    test::call_service(&app, register_req).await;
    
    let login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": "all_venues@example.com",
            "password": "password123"
        }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let session_id = login_body["session_id"].as_str().unwrap();

    // Create multiple venues
    for i in 0..3 {
        let create_req = test::TestRequest::post()
            .uri("/api/venues")
            .insert_header(("Authorization", format!("Bearer {}", session_id)))
            .set_json(&json!({
            "displayName": format!("Venue {}", i),
            "formattedAddress": format!("{} Test St, Test City, TS 12345, USA", i),
            "place_id": format!("test_place_id_{}", i),
            "lat": 40.7128,
            "lng": -74.0060,
                "timezone": "America/New_York"
            }))
            .to_request();
        let create_resp = test::call_service(&app, create_req).await;
        assert!(
            create_resp.status().is_success(),
            "Venue creation should succeed, got status: {}",
            create_resp.status()
        );
    }

    // Get all venues
    let get_all_req = test::TestRequest::get()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let mut get_all_resp = test::call_service(&app, get_all_req).await;
    let get_all_status = get_all_resp.status();
    if !get_all_status.is_success() {
        let body_bytes = test::read_body(get_all_resp).await;
        let body_text = String::from_utf8_lossy(&body_bytes);
        panic!(
            "Get all venues should succeed, got status: {}, body: {}",
            get_all_status,
            body_text
        );
    }
    let venues: Vec<VenueDto> = test::read_body_json(get_all_resp).await;
    assert!(venues.len() >= 3);

    Ok(())
}

#[tokio::test]
async fn test_update_venue() -> Result<()> {
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
            .app_data(app_data.venue_repo.clone())
            .service(web::scope("/api/players")
                .service(backend::player::controller::register_handler_prod)
                .service(backend::player::controller::login_handler_prod)
            )
            .service(web::scope("/api/venues")
                .wrap(backend::auth::AuthMiddleware { 
                    redis: std::sync::Arc::new(app_data.redis_data.get_ref().clone()) 
                })
                .service(backend::venue::controller::create_venue_handler)
                .service(backend::venue::controller::update_venue_handler)
            )
    ).await;

    // Register, login, create venue
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "update_venue_user",
            "email": "update_venue@example.com",
            "password": "password123"
        }))
        .to_request();
    test::call_service(&app, register_req).await;
    
    let login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": "update_venue@example.com",
            "password": "password123"
        }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let session_id = login_body["session_id"].as_str().unwrap();

    let create_req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "displayName": "Original Venue",
            "formattedAddress": "123 Original St, Original City, OR 12345, USA",
            "place_id": "original_place_id",
            "lat": 40.7128,
            "lng": -74.0060,
            "timezone": "America/New_York"
        }))
        .to_request();
    
    let mut create_resp = test::call_service(&app, create_req).await;
    let create_status = create_resp.status();
    if !create_status.is_success() {
        let body_bytes = test::read_body(create_resp).await;
        let body_text = String::from_utf8_lossy(&body_bytes);
        panic!(
            "Venue creation should succeed, got status: {}, body: {}",
            create_status,
            body_text
        );
    }
    let venue: VenueDto = test::read_body_json(create_resp).await;
    let venue_id = venue.id.clone();

    // Update venue
    let venue_key = venue_id.split_once('/').map(|(_, k)| k).unwrap_or(&venue_id);
    let update_req = test::TestRequest::put()
        .uri(&format!("/api/venues/{}", venue_key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "displayName": "Updated Venue",
            "formattedAddress": "456 Updated St, Updated City, UP 54321, USA",
            "place_id": "updated_place_id",
            "lat": 34.0522,
            "lng": -118.2437,
            "timezone": "America/Los_Angeles"
        }))
        .to_request();

    let mut update_resp = test::call_service(&app, update_req).await;
    let update_status = update_resp.status();
    if !update_status.is_success() {
        let body_bytes = test::read_body(update_resp).await;
        let body_text = String::from_utf8_lossy(&body_bytes);
        panic!(
            "Update venue should succeed, got status: {}, body: {}",
            update_status,
            body_text
        );
    }
    let updated: VenueDto = test::read_body_json(update_resp).await;
    assert_eq!(updated.display_name, "Updated Venue");
    assert_eq!(updated.formatted_address, "456 Updated St, Updated City, UP 54321, USA");
    assert_eq!(updated.id, venue_id);

    Ok(())
}

#[tokio::test]
async fn test_delete_venue() -> Result<()> {
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
            .app_data(app_data.venue_repo.clone())
            .service(web::scope("/api/players")
                .service(backend::player::controller::register_handler_prod)
                .service(backend::player::controller::login_handler_prod)
            )
            .service(web::scope("/api/venues")
                .wrap(backend::auth::AuthMiddleware { 
                    redis: std::sync::Arc::new(app_data.redis_data.get_ref().clone()) 
                })
                .service(backend::venue::controller::create_venue_handler)
                .service(backend::venue::controller::delete_venue_handler)
                .service(backend::venue::controller::get_venue_handler)
            )
    ).await;

    // Register, login, create venue
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "delete_venue_user",
            "email": "delete_venue@example.com",
            "password": "password123"
        }))
        .to_request();
    test::call_service(&app, register_req).await;
    
    let login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": "delete_venue@example.com",
            "password": "password123"
        }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let session_id = login_body["session_id"].as_str().unwrap();

    let create_req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "displayName": "Delete Test Venue",
            "formattedAddress": "789 Delete St, Delete City, DL 12345, USA",
            "place_id": "delete_place_id",
            "lat": 40.7128,
            "lng": -74.0060,
            "timezone": "America/New_York"
        }))
        .to_request();
    
    let mut create_resp = test::call_service(&app, create_req).await;
    let create_status = create_resp.status();
    if !create_status.is_success() {
        let body_bytes = test::read_body(create_resp).await;
        let body_text = String::from_utf8_lossy(&body_bytes);
        panic!(
            "Venue creation should succeed, got status: {}, body: {}",
            create_status,
            body_text
        );
    }
    let venue: VenueDto = test::read_body_json(create_resp).await;
    let venue_id = venue.id.clone();

    // Delete venue
    let venue_key = venue_id.split_once('/').map(|(_, k)| k).unwrap_or(&venue_id);
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/venues/{}", venue_key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let delete_resp = test::call_service(&app, delete_req).await;
    let delete_status = delete_resp.status();
    if !delete_status.is_success() {
        let body_bytes = test::read_body(delete_resp).await;
        let body_text = String::from_utf8_lossy(&body_bytes);
        panic!(
            "Delete venue should succeed, got status: {}, body: {}",
            delete_status,
            body_text
        );
    }

    // Verify venue is deleted
    let venue_key = venue_id.split_once('/').map(|(_, k)| k).unwrap_or(&venue_id);
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/venues/{}", venue_key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status().as_u16(), 404);

    Ok(())
}

#[tokio::test]
async fn test_venue_validation_errors() -> Result<()> {
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
            .app_data(app_data.venue_repo.clone())
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
            "username": "validation_user",
            "email": "validation@example.com",
            "password": "password123"
        }))
        .to_request();
    test::call_service(&app, register_req).await;
    
    let login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": "validation@example.com",
            "password": "password123"
        }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let session_id = login_body["session_id"].as_str().unwrap();

    // Test missing required fields
    let invalid_req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "displayName": "Incomplete Venue"
            // Missing required fields
        }))
        .to_request();

    let invalid_resp = test::call_service(&app, invalid_req).await;
    assert!(invalid_resp.status().is_client_error());

    Ok(())
}

#[tokio::test]
async fn test_venue_unauthorized_access() -> Result<()> {
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
            .app_data(app_data.venue_repo.clone())
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

    // Try to create venue without authentication
    let req = test::TestRequest::post()
        .uri("/api/venues")
        .set_json(&json!({
            "displayName": "Unauthorized Venue",
            "formattedAddress": "123 Unauthorized St, Unauthorized City, UN 12345, USA",
            "place_id": "unauthorized_place_id",
            "lat": 40.7128,
            "lng": -74.0060,
            "timezone": "America/New_York"
        }))
        .to_request();

    // Use try_call_service for error responses
    match test::try_call_service(&app, req).await {
        Ok(resp) => {
            assert!(
                resp.status().is_client_error(),
                "Unauthorized request should return client error, got: {}",
                resp.status()
            );
        }
        Err(e) => {
            // If it's an ErrorUnauthorized, that's also acceptable
            use actix_web::error::ResponseError;
            let status = e.as_response_error().status_code();
            assert_eq!(
                status,
                401,
                "Should return 401 Unauthorized, got: {}",
                status
            );
        }
    }

    Ok(())
}

