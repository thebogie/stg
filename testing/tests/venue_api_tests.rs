//! Integration tests for Venue API endpoints
//!
//! Tests complete CRUD operations for venues with real database and Redis

mod test_helpers;

use actix_web::{test, web, App};
use anyhow::Result;
use serde_json::json;
use shared::dto::venue::VenueDto;
use shared::models::venue::VenueSource;
use testing::create_authenticated_user;
use testing::{app_setup, TestEnvironment};

#[tokio::test]
async fn test_create_venue_success() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_all_venues_handler)
                    .service(backend::venue::controller::get_venue_handler)
                    .service(backend::venue::controller::create_venue_handler)
                    .service(backend::venue::controller::update_venue_handler)
                    .service(backend::venue::controller::delete_venue_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "venue_test@example.com", "venueuser");

    let venue_data = json!({
        "displayName": "Test Venue",
        "formattedAddress": "123 Test St, Test City, TS 12345",
        "place_id": "test_place_id_123",
        "lat": 40.7128,
        "lng": -74.0060,
        "timezone": "America/New_York",
        "source": "database"
    });

    let req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&venue_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(
        resp.status().is_success(),
        "Creating venue should succeed, got status: {}",
        resp.status()
    );

    let venue: VenueDto = test::read_body_json(resp).await;
    assert_eq!(venue.display_name, "Test Venue");
    assert_eq!(venue.formatted_address, "123 Test St, Test City, TS 12345");
    assert_eq!(venue.place_id, "test_place_id_123");
    assert!(!venue.id.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_create_venue_validation_errors() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::create_venue_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "venue_val@example.com", "venueval");

    // Test empty display name
    let invalid_venue = json!({
        "displayName": "",
        "formattedAddress": "123 Test St",
        "place_id": "test_place_id",
        "lat": 40.7128,
        "lng": -74.0060
    });

    let req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&invalid_venue)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_client_error(),
        "Invalid venue should return 4xx, got: {}",
        resp.status()
    );

    // Test invalid coordinates
    let invalid_coords = json!({
        "displayName": "Test Venue",
        "formattedAddress": "123 Test St",
        "place_id": "test_place_id",
        "lat": 91.0,  // Invalid: > 90
        "lng": -74.0060
    });

    let req2 = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&invalid_coords)
        .to_request();

    let resp2 = test::call_service(&app, req2).await;
    assert!(
        resp2.status().is_client_error(),
        "Invalid coordinates should return 4xx, got: {}",
        resp2.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_get_venue_success() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_all_venues_handler)
                    .service(backend::venue::controller::get_venue_handler)
                    .service(backend::venue::controller::create_venue_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "venue_get@example.com", "venueget");

    // First create a venue
    let venue_data = json!({
        "displayName": "Get Test Venue",
        "formattedAddress": "456 Get St, Test City",
        "place_id": "get_place_id_456",
        "lat": 40.7580,
        "lng": -73.9855
    });

    let create_req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&venue_data)
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    assert!(
        create_resp.status().is_success(),
        "Create should succeed, got: {}",
        create_resp.status()
    );
    let created_venue: VenueDto = test::read_body_json(create_resp).await;
    let venue_id = created_venue.id.clone();

    // Extract just the ID part if it's in format "venue/123"
    let venue_id_for_url = if venue_id.contains('/') {
        venue_id.split('/').last().unwrap_or(&venue_id).to_string()
    } else {
        venue_id.clone()
    };

    // Now get it - use the ID part only, handler will add "venue/" prefix if needed
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/venues/{}", venue_id_for_url))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let get_resp = test::call_service(&app, get_req).await;
    assert!(
        get_resp.status().is_success(),
        "Getting venue should succeed, got: {}",
        get_resp.status()
    );

    let retrieved_venue: VenueDto = test::read_body_json(get_resp).await;
    assert_eq!(retrieved_venue.id, venue_id);
    assert_eq!(retrieved_venue.display_name, "Get Test Venue");

    Ok(())
}

#[tokio::test]
async fn test_get_venue_not_found() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_venue_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "venue_nf@example.com", "venuenf");

    let req = test::TestRequest::get()
        .uri("/api/venues/venue/nonexistent_12345")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "Non-existent venue should return 404, got: {}",
        resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_update_venue_success() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_venue_handler)
                    .service(backend::venue::controller::create_venue_handler)
                    .service(backend::venue::controller::update_venue_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "venue_upd@example.com", "venueupd");

    // Create a venue first
    let venue_data = json!({
        "displayName": "Original Name",
        "formattedAddress": "Original Address",
        "place_id": "update_place_id",
        "lat": 40.7128,
        "lng": -74.0060
    });

    let create_req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&venue_data)
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    assert!(create_resp.status().is_success());
    let created_venue: VenueDto = test::read_body_json(create_resp).await;
    let venue_id = created_venue.id.clone();

    // Extract just the ID part if it's in format "venue/123"
    let venue_id_for_url = if venue_id.contains('/') {
        venue_id.split('/').last().unwrap_or(&venue_id).to_string()
    } else {
        venue_id.clone()
    };

    // Update it
    let updated_data = json!({
        "displayName": "Updated Name",
        "formattedAddress": "Updated Address",
        "place_id": "update_place_id",
        "lat": 40.7580,
        "lng": -73.9855
    });

    let update_req = test::TestRequest::put()
        .uri(&format!("/api/venues/{}", venue_id_for_url))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&updated_data)
        .to_request();

    let update_resp = test::call_service(&app, update_req).await;
    assert!(
        update_resp.status().is_success(),
        "Updating venue should succeed, got: {}",
        update_resp.status()
    );

    let updated_venue: VenueDto = test::read_body_json(update_resp).await;
    assert_eq!(updated_venue.display_name, "Updated Name");
    assert_eq!(updated_venue.formatted_address, "Updated Address");

    Ok(())
}

#[tokio::test]
async fn test_update_venue_not_found() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::update_venue_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "venue_upd_nf@example.com", "venueupdnf");

    let update_data = json!({
        "displayName": "Updated Name",
        "formattedAddress": "Updated Address",
        "place_id": "nonexistent",
        "lat": 40.7128,
        "lng": -74.0060
    });

    let req = test::TestRequest::put()
        .uri("/api/venues/venue/nonexistent_99999")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&update_data)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "Updating non-existent venue should return 404, got: {}",
        resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_delete_venue_success() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_venue_handler)
                    .service(backend::venue::controller::create_venue_handler)
                    .service(backend::venue::controller::delete_venue_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "venue_del@example.com", "venuedel");

    // Create a venue first
    let venue_data = json!({
        "displayName": "To Delete",
        "formattedAddress": "Delete Address",
        "place_id": "delete_place_id",
        "lat": 40.7128,
        "lng": -74.0060
    });

    let create_req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&venue_data)
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    assert!(create_resp.status().is_success());
    let created_venue: VenueDto = test::read_body_json(create_resp).await;
    let venue_id = created_venue.id.clone();

    // Extract just the ID part if it's in format "venue/123"
    let venue_id_for_url = if venue_id.contains('/') {
        venue_id.split('/').last().unwrap_or(&venue_id).to_string()
    } else {
        venue_id.clone()
    };

    // Delete it
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/venues/{}", venue_id_for_url))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let delete_resp = test::call_service(&app, delete_req).await;
    assert!(
        delete_resp.status().is_success(),
        "Deleting venue should succeed, got: {}",
        delete_resp.status()
    );

    // Verify it's deleted
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/venues/{}", venue_id_for_url))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(
        get_resp.status(),
        404,
        "Deleted venue should return 404, got: {}",
        get_resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_delete_venue_not_found() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::delete_venue_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "venue_del_nf@example.com", "venuedelnf");

    let req = test::TestRequest::delete()
        .uri("/api/venues/venue/nonexistent_99999")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "Deleting non-existent venue should return 404, got: {}",
        resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_get_all_venues() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_all_venues_handler)
                    .service(backend::venue::controller::create_venue_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "venue_list@example.com", "venuelist");

    // Create a couple of venues
    for i in 1..=3 {
        let venue_data = json!({
            "displayName": format!("Venue {}", i),
            "formattedAddress": format!("Address {}", i),
            "place_id": format!("place_id_{}", i),
            "lat": 40.7128 + (i as f64 * 0.01),
            "lng": -74.0060 + (i as f64 * 0.01)
        });

        let create_req = test::TestRequest::post()
            .uri("/api/venues")
            .insert_header(("Authorization", format!("Bearer {}", session_id)))
            .set_json(&venue_data)
            .to_request();

        let create_resp = test::call_service(&app, create_req).await;
        assert!(create_resp.status().is_success());
    }

    // Get all venues
    let req = test::TestRequest::get()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Getting all venues should succeed, got: {}",
        resp.status()
    );

    let venues: Vec<VenueDto> = test::read_body_json(resp).await;
    assert!(venues.len() >= 3, "Should have at least 3 venues");

    Ok(())
}

#[tokio::test]
async fn test_venue_unauthorized_access() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_all_venues_handler),
            ),
    )
    .await;

    // Try to access without authentication
    let req = test::TestRequest::get().uri("/api/venues").to_request();

    let resp = test::try_call_service(&app, req).await;

    match resp {
        Ok(resp) => {
            assert_eq!(
                resp.status(),
                401,
                "Unauthenticated request should return 401, got: {}",
                resp.status()
            );
        }
        Err(e) => {
            use actix_web::error::ResponseError;
            let status = e.as_response_error().status_code();
            assert_eq!(
                status, 401,
                "Should return 401 Unauthorized error, got: {}",
                status
            );
        }
    }

    Ok(())
}
