//! Full-stack API integration tests using testcontainers
//!
//! These tests exercise the complete backend API with real database and Redis,
//! using ephemeral containers for true isolation.

use anyhow::Result;
use actix_web::{test, web, App, HttpResponse};
use serde_json::json;
use testing::{TestEnvironment, app_setup};

// Use the actual DTOs from shared
use shared::dto::player::{PlayerDto, LoginResponse};

// Note: The backend returns session_id in the JSON response body.
// We extract it and use it in the Cookie header for authenticated requests.

#[tokio::test]
async fn test_player_registration() -> Result<()> {
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
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
            )
    ).await;

    // Test successful registration
    let req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "testuser",
            "email": "test@example.com",
            "password": "password123"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    assert!(
        resp.status().is_success(),
        "Registration should succeed, got status: {}",
        resp.status()
    );

    let body: PlayerDto = test::read_body_json(resp).await;
    assert_eq!(body.handle, "testuser");
    assert_eq!(body.email, "test@example.com");
    assert!(!body.id.is_empty());
    assert!(!body.firstname.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_player_registration_duplicate_email() -> Result<()> {
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
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
            )
    ).await;

    // Register first user
    let req1 = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "user1",
            "email": "duplicate@example.com",
            "password": "password123"
        }))
        .to_request();
    let resp1 = test::call_service(&app, req1).await;
    assert!(resp1.status().is_success());

    // Try to register with same email
    let req2 = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "user2",
            "email": "duplicate@example.com",
            "password": "password123"
        }))
        .to_request();
    let resp2 = test::call_service(&app, req2).await;
    
    assert!(
        resp2.status().is_client_error(),
        "Duplicate email should fail, got status: {}",
        resp2.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_player_login() -> Result<()> {
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
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod)
            )
    ).await;

    // First, register a user
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "loginuser",
            "email": "login@example.com",
            "password": "password123"
        }))
        .to_request();
    let register_resp = test::call_service(&app, register_req).await;
    assert!(register_resp.status().is_success());

    // Now try to login
    let login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": "login@example.com",
            "password": "password123"
        }))
        .to_request();

    let login_resp = test::call_service(&app, login_req).await;
    
    assert!(
        login_resp.status().is_success(),
        "Login should succeed, got status: {}",
        login_resp.status()
    );

    let body: LoginResponse = test::read_body_json(login_resp).await;
    assert_eq!(body.player.email, "login@example.com");
    assert_eq!(body.player.handle, "loginuser");
    assert!(!body.session_id.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_player_login_invalid_credentials() -> Result<()> {
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
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::login_handler_prod)
            )
    ).await;

    // Try to login with non-existent user
    let req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": "nonexistent@example.com",
            "password": "wrongpassword"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    assert!(
        resp.status().is_client_error(),
        "Invalid credentials should fail, got status: {}",
        resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_get_current_player() -> Result<()> {
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
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod)
                    .service(
                        web::scope("/me")
                            .wrap(backend::auth::AuthMiddleware {
                                redis: app_data.redis_arc.clone(),
                            })
                            .service(backend::player::controller::me_handler_prod)
                    )
            )
    ).await;

    // Register and login to get session
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "meuser",
            "email": "me@example.com",
            "password": "password123"
        }))
        .to_request();
    test::call_service(&app, register_req).await;

    let login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": "me@example.com",
            "password": "password123"
        }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    assert!(login_resp.status().is_success());

    // Extract session ID from response
    let login_body: LoginResponse = test::read_body_json(login_resp).await;
    let session_id = login_body.session_id;

    // Get current player using session ID in Authorization header
    // The backend expects: "Authorization: Bearer <session_id>"
    let me_req = test::TestRequest::get()
        .uri("/api/players/me")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let me_resp = test::call_service(&app, me_req).await;
    
    assert!(
        me_resp.status().is_success(),
        "Get current player should succeed, got status: {}",
        me_resp.status()
    );

    let body: PlayerDto = test::read_body_json(me_resp).await;
    assert_eq!(body.email, "me@example.com");
    assert_eq!(body.handle, "meuser");

    Ok(())
}

#[tokio::test]
async fn test_get_current_player_unauthorized() -> Result<()> {
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
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(
                        web::scope("/me")
                            .wrap(backend::auth::AuthMiddleware {
                                redis: app_data.redis_arc.clone(),
                            })
                            .service(backend::player::controller::me_handler_prod)
                    )
            )
    ).await;

    // Try to get current player without authentication
    let req = test::TestRequest::get()
        .uri("/api/players/me")
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // The middleware returns 401 Unauthorized for missing auth
    assert!(
        resp.status().is_client_error(),
        "Unauthenticated request should return 4xx, got: {}",
        resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_player_logout() -> Result<()> {
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
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod)
                    .service(backend::player::controller::logout_handler_prod)
                    .service(
                        web::scope("/me")
                            .wrap(backend::auth::AuthMiddleware {
                                redis: app_data.redis_arc.clone(),
                            })
                            .service(backend::player::controller::me_handler_prod)
                    )
            )
    ).await;

    // Register and login
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "logoutuser",
            "email": "logout@example.com",
            "password": "password123"
        }))
        .to_request();
    test::call_service(&app, register_req).await;

    let login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": "logout@example.com",
            "password": "password123"
        }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    let login_body: LoginResponse = test::read_body_json(login_resp).await;
    let session_id = login_body.session_id;

    // Logout - backend expects Authorization header
    let logout_req = test::TestRequest::post()
        .uri("/api/players/logout")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let logout_resp = test::call_service(&app, logout_req).await;
    assert!(
        logout_resp.status().is_success(),
        "Logout should succeed, got status: {}",
        logout_resp.status()
    );

    // Verify session is invalidated - try to access /me
    let me_req = test::TestRequest::get()
        .uri("/api/players/me")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let me_resp = test::call_service(&app, me_req).await;
    assert_eq!(
        me_resp.status(),
        401,
        "Session should be invalidated after logout"
    );

    Ok(())
}
