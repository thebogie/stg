//! Integration tests for Error Handling
//!
//! Tests various error scenarios (400, 401, 403, 404, 500)

//! Integration tests for Error Handling
//!
//! Tests various error scenarios (400, 401, 403, 404, 500)

mod test_helpers;

use actix_web::{test, web, App};
use anyhow::Result;
use serde_json::json;
use testing::create_authenticated_user;
use testing::{app_setup, TestEnvironment};

#[tokio::test]
async fn test_400_bad_request_validation_errors() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::create_game_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "error_test@example.com", "errortest");

    // Test invalid venue data (empty display name)
    let invalid_venue = json!({
        "displayName": "",
        "formattedAddress": "Test",
        "place_id": "test",
        "lat": 40.7128,
        "lng": -74.0060
    });

    let req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&invalid_venue)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        400,
        "Invalid venue data should return 400, got: {}",
        resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_401_unauthorized_missing_auth() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::create_game_handler),
            ),
    )
    .await;

    // Try to access protected endpoint without auth
    let req = test::TestRequest::get().uri("/api/venues").to_request();

    let resp = test::try_call_service(&app, req).await;

    match resp {
        Ok(resp) => {
            assert_eq!(
                resp.status(),
                401,
                "Missing auth should return 401, got: {}",
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

#[tokio::test]
async fn test_401_unauthorized_invalid_session() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::create_game_handler),
            ),
    )
    .await;

    // Try to access with invalid session
    let req = test::TestRequest::get()
        .uri("/api/venues")
        .insert_header(("Authorization", "Bearer invalid_session_token"))
        .to_request();

    let resp = test::try_call_service(&app, req).await;

    match resp {
        Ok(resp) => {
            assert_eq!(
                resp.status(),
                401,
                "Invalid session should return 401, got: {}",
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

#[tokio::test]
async fn test_404_not_found_resources() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::create_game_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "error_404@example.com", "error404");

    // Test non-existent venue
    let req1 = test::TestRequest::get()
        .uri("/api/venues/venue/nonexistent_99999")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let resp1 = test::call_service(&app, req1).await;
    assert_eq!(
        resp1.status(),
        404,
        "Non-existent venue should return 404, got: {}",
        resp1.status()
    );

    // Test non-existent game
    let req2 = test::TestRequest::get()
        .uri("/api/games/game/nonexistent_99999")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let resp2 = test::call_service(&app, req2).await;
    assert_eq!(
        resp2.status(),
        404,
        "Non-existent game should return 404, got: {}",
        resp2.status()
    );

    // Test non-existent contest
    let req3 = test::TestRequest::get()
        .uri("/api/contests/contest/nonexistent_99999")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let resp3 = test::call_service(&app, req3).await;
    assert_eq!(
        resp3.status(),
        404,
        "Non-existent contest should return 404, got: {}",
        resp3.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_malformed_json_request() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::create_game_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "error_json@example.com", "errorjson");

    // Send malformed JSON
    let req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .insert_header(("Content-Type", "application/json"))
        .set_payload("{invalid json}")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_client_error(),
        "Malformed JSON should return 4xx, got: {}",
        resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_missing_required_fields() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::create_game_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "error_fields@example.com", "errorfields");

    // Create venue with missing required fields
    let incomplete_venue = json!({
        "displayName": "Test"
        // Missing formattedAddress, place_id, lat, lng
    });

    let req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&incomplete_venue)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_client_error(),
        "Missing required fields should return 4xx, got: {}",
        resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_invalid_field_types() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::create_game_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "error_types@example.com", "errortypes");

    // Create venue with wrong type for lat (string instead of number)
    let invalid_types = json!({
        "displayName": "Test",
        "formattedAddress": "Test Address",
        "place_id": "test_place",
        "lat": "not_a_number",  // Should be a number
        "lng": -74.0060
    });

    let req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&invalid_types)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_client_error(),
        "Invalid field types should return 4xx, got: {}",
        resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_oversized_request_body() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::create_game_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "error_size@example.com", "errorsize");

    // Create a very large description (over 4000 chars for game)
    let large_description = "x".repeat(5000);
    let oversized_game = json!({
        "name": "Test Game",
        "description": large_description,
        "source": "Database"
    });

    let req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&oversized_game)
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Should either reject with 400 (validation) or 413 (payload too large)
    assert!(
        resp.status().is_client_error(),
        "Oversized request should return 4xx, got: {}",
        resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_invalid_content_type() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::create_game_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "error_content@example.com", "errorcontent");

    // Send JSON with wrong content type
    let req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .insert_header(("Content-Type", "text/plain"))
        .set_payload(r#"{"displayName":"Test"}"#)
        .to_request();

    // Actix-web is lenient with content types, but this tests the scenario
    let resp = test::call_service(&app, req).await;
    // May succeed or fail depending on actix-web's content type handling
    // Just verify it doesn't crash
    assert!(
        resp.status().is_success() || resp.status().is_client_error(),
        "Request should be handled (success or error), got: {}",
        resp.status()
    );

    Ok(())
}
