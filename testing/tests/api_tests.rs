//! API integration tests against the official stack (SurrealDB + Redis). Start with: ./deploy/stack.sh start
//!
//! These tests exercise the complete backend API with real database and Redis,
//! using ephemeral containers for true isolation.

use actix_web::{test, web, App};
use anyhow::Result;
use serde_json::json;
use testing::{app_setup, TestEnvironment};

// Use the actual DTOs from shared
use shared::dto::player::{LoginResponse, PlayerDto};

// Note: The backend returns session_id in the JSON response body.
// We extract it and use it in the Cookie header for authenticated requests.

#[tokio::test]
#[serial_test::serial]
async fn test_player_registration() -> Result<()> {
    // Set explicit timeout for test environment setup
    let env = tokio::time::timeout(std::time::Duration::from_secs(120), TestEnvironment::new())
        .await
        .map_err(|_| anyhow::anyhow!("Test environment setup timed out after 120s"))??;

    env.wait_for_ready().await?;

    let app_data = app_setup::setup_test_app_data(&env).await?;
    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod),
            ),
    )
    .await;

    // Use unique email so repeated runs don't hit duplicate-email
    let email = format!(
        "test-{}@example.com",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_millis()
    );
    let req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "testuser",
            "email": &email,
            "password": "password123"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body_bytes = test::read_body(resp).await;
    assert!(
        status.as_u16() == 201,
        "Registration should return 201 Created, got status: {} body: {}",
        status,
        String::from_utf8_lossy(&body_bytes)
    );

    let body: PlayerDto = serde_json::from_slice(body_bytes.as_ref()).unwrap_or_else(|e| {
        panic!(
            "Register response should be PlayerDto: {} body: {}",
            e,
            String::from_utf8_lossy(body_bytes.as_ref())
        )
    });
    assert_eq!(body.handle, "testuser");
    assert_eq!(body.email, email);
    assert!(
        !body.id.is_empty(),
        "Register must return non-empty player id"
    );
    assert!(
        body.id.starts_with("player/"),
        "Register must return id in form player/<key>, got {}",
        body.id
    );
    assert!(!body.firstname.is_empty());

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_player_registration_duplicate_email() -> Result<()> {
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
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod),
            ),
    )
    .await;

    // Use unique email for first user so test is robust when DB already has data (e.g. from earlier test run)
    let email = format!(
        "dup-{}@example.com",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_millis()
    );
    // Register first user
    let req1 = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "user1",
            "email": &email,
            "password": "password123"
        }))
        .to_request();
    let resp1 = test::call_service(&app, req1).await;
    let status1 = resp1.status();
    let body1 = test::read_body(resp1).await;
    assert!(
        status1.is_success(),
        "First register should succeed, got status: {} body: {}",
        status1,
        String::from_utf8_lossy(&body1)
    );

    // Try to register with same email (must be rejected)
    let req2 = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "user2",
            "email": &email,
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
#[serial_test::serial]
async fn test_player_login() -> Result<()> {
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
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            ),
    )
    .await;

    // Use unique email per run so we don't hit conflicts or stale data
    let email = format!(
        "login-{}@example.com",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_millis()
    );
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "loginuser",
            "email": &email,
            "password": "password123"
        }))
        .to_request();
    let register_resp = test::call_service(&app, register_req).await;
    let register_status = register_resp.status();
    let register_body = test::read_body(register_resp).await;
    assert!(
        register_status.is_success(),
        "Register should succeed, got status: {} body: {:?}",
        register_status,
        register_body
    );

    // Retry login in case of read-after-write delay
    let mut last_status = actix_web::http::StatusCode::OK;
    let mut last_body = Vec::new();
    for _ in 0..3 {
        let login_req = test::TestRequest::post()
            .uri("/api/players/login")
            .set_json(&json!({
                "email": &email,
                "password": "password123"
            }))
            .to_request();
        let resp = test::call_service(&app, login_req).await;
        last_status = resp.status();
        last_body = test::read_body(resp).await.to_vec();
        if last_status.is_success() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    }
    assert!(
        last_status.is_success(),
        "Login should succeed after register, got status: {} body: {}",
        last_status,
        String::from_utf8_lossy(&last_body)
    );
    let body: LoginResponse = serde_json::from_slice(&last_body).unwrap_or_else(|e| {
        panic!(
            "Login body should be LoginResponse: {} body: {}",
            e,
            String::from_utf8_lossy(&last_body)
        )
    });
    assert_eq!(body.player.email, email);
    assert_eq!(body.player.handle, "loginuser");
    assert!(!body.session_id.is_empty());

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_player_login_invalid_credentials() -> Result<()> {
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
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players").service(backend::player::controller::login_handler_prod),
            ),
    )
    .await;

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
#[serial_test::serial]
async fn test_get_current_player() -> Result<()> {
    let timeout = std::time::Duration::from_secs(90);
    let body = async {
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
                                .service(backend::player::controller::me_handler_prod),
                        ),
                ),
        )
        .await;

        // Use unique email so we don't conflict with leftover data; register then login.
        let email = format!(
            "meuser-{}@example.com",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_millis()
        );
        let register_req = test::TestRequest::post()
            .uri("/api/players/register")
            .set_json(&json!({
                "username": "meuser",
                "email": &email,
                "password": "password123"
            }))
            .to_request();
        let register_resp = test::call_service(&app, register_req).await;
        let register_status = register_resp.status();
        let register_body = test::read_body(register_resp).await;
        assert!(
            register_status.is_success(),
            "Register should succeed, got status: {} body: {:?}",
            register_status,
            register_body
        );

        // Retry login a few times in case of read-after-write delay in SurrealDB
        let mut login_body_bytes = Vec::new();
        let mut login_status = actix_web::http::StatusCode::OK;
        for _ in 0..3 {
            let login_req = test::TestRequest::post()
                .uri("/api/players/login")
                .set_json(&json!({
                    "email": &email,
                    "password": "password123"
                }))
                .to_request();
            let login_resp = test::call_service(&app, login_req).await;
            login_status = login_resp.status();
            login_body_bytes = test::read_body(login_resp).await.to_vec();
            if login_status.is_success() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }
        assert!(
            login_status.is_success(),
            "Login should succeed after register, got status: {} body: {}",
            login_status,
            String::from_utf8_lossy(&login_body_bytes)
        );

        // Extract session ID from response
        let login_body: LoginResponse =
            serde_json::from_slice(&login_body_bytes).unwrap_or_else(|e| {
                panic!(
                    "Login body should be LoginResponse: {} body: {}",
                    e,
                    String::from_utf8_lossy(&login_body_bytes)
                )
            });
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

        let me_body: PlayerDto = test::read_body_json(me_resp).await;
        assert_eq!(
            me_body.email, email,
            "GET /me should return the registered player's email"
        );
        assert_eq!(me_body.handle, "meuser");

        Ok::<(), anyhow::Error>(())
    };
    tokio::time::timeout(timeout, body)
        .await
        .map_err(|_| anyhow::anyhow!("test_get_current_player timed out after {:?} (is SurrealDB/Redis running on 127.0.0.1?)", timeout))??;
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn test_get_current_player_unauthorized() -> Result<()> {
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
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players").service(
                    web::scope("/me")
                        .wrap(backend::auth::AuthMiddleware {
                            redis: app_data.redis_arc.clone(),
                        })
                        .service(backend::player::controller::me_handler_prod),
                ),
            ),
    )
    .await;

    // Try to get current player without authentication
    let req = test::TestRequest::get().uri("/api/players/me").to_request();

    // Use try_call_service to handle error responses properly
    let resp = test::try_call_service(&app, req).await;

    // The middleware returns 401 Unauthorized for missing auth
    match resp {
        Ok(resp) => {
            assert!(
                resp.status().is_client_error(),
                "Unauthenticated request should return 4xx, got: {}",
                resp.status()
            );
            assert_eq!(
                resp.status(),
                401,
                "Should return 401 Unauthorized, got: {}",
                resp.status()
            );
        }
        Err(e) => {
            // If it's an ErrorUnauthorized, get the status code directly
            // In actix-web, Error has status_code() method
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
#[serial_test::serial]
async fn test_player_logout() -> Result<()> {
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
                            .service(backend::player::controller::me_handler_prod),
                    ),
            ),
    )
    .await;

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
    assert!(
        login_resp.status().is_success(),
        "Login should succeed, got status: {}",
        login_resp.status()
    );

    let login_body: LoginResponse = test::read_body_json(login_resp).await;
    let session_id = login_body.session_id;
    assert!(
        !session_id.is_empty(),
        "Session ID should not be empty after login"
    );

    // Give Redis a moment to ensure session is stored
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Logout - backend expects Authorization header
    let logout_req = test::TestRequest::post()
        .uri("/api/players/logout")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let logout_resp = test::call_service(&app, logout_req).await;
    assert!(
        logout_resp.status().is_success(),
        "Logout should succeed, got status: {}. Response: {:?}",
        logout_resp.status(),
        test::read_body_json::<serde_json::Value, _>(logout_resp).await
    );

    // Give Redis a moment to ensure session is deleted
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify session is invalidated - try to access /me
    let me_req = test::TestRequest::get()
        .uri("/api/players/me")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    // Use try_call_service to handle the expected error
    let me_resp_result = test::try_call_service(&app, me_req).await;
    match me_resp_result {
        Ok(resp) => {
            assert_eq!(
                resp.status(),
                401,
                "Session should be invalidated after logout, got status: {}",
                resp.status()
            );
        }
        Err(e) => {
            // ErrorUnauthorized is expected after logout
            let status = e.as_response_error().status_code();
            assert_eq!(
                status, 401,
                "Session should be invalidated after logout, got error status: {}",
                status
            );
        }
    }

    Ok(())
}
