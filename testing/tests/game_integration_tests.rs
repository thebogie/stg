//! Comprehensive integration tests for Game API endpoints

use actix_web::dev::ServiceResponse;
use actix_web::{test, web, App};
use anyhow::Result;
use serde_json::json;
use shared::dto::game::GameDto;
use testing::{app_setup, TestEnvironment};

/// Helper to read response body as text for debugging  
async fn read_body_text<B: actix_web::body::MessageBody>(resp: ServiceResponse<B>) -> String {
    let body = test::read_body(resp).await;
    String::from_utf8_lossy(&body).to_string()
}

#[tokio::test]
async fn test_create_game() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.session_store.clone())
            .app_data(app_data.game_repo.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: std::sync::Arc::new(app_data.redis_data.get_ref().clone()),
                    })
                    .service(backend::game::controller::create_game_handler),
            ),
    )
    .await;

    // Register and login
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "game_test_user",
            "email": "game_test@example.com",
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
            "email": "game_test@example.com",
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
    let session_id = login_body["session_id"]
        .as_str()
        .expect("Login response should contain session_id");

    // Create game
    let game_data = json!({
        "name": "Test Game",
        "year_published": 2023,
        "source": "bgg"
    });

    let req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&game_data)
        .to_request();

    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    if !status.is_success() {
        let body_text = read_body_text(resp).await;
        panic!(
            "Create game should succeed, got status: {}, body: {}",
            status, body_text
        );
    }
    let game: GameDto = test::read_body_json(resp).await;
    assert_eq!(game.name, "Test Game");
    assert_eq!(game.year_published, Some(2023));
    assert!(!game.id.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_get_all_games() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.session_store.clone())
            .app_data(app_data.game_repo.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: std::sync::Arc::new(app_data.redis_data.get_ref().clone()),
                    })
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::get_all_games_handler),
            ),
    )
    .await;

    // Register and login
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "all_games_user",
            "email": "all_games@example.com",
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
            "email": "all_games@example.com",
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
    let session_id = login_body["session_id"]
        .as_str()
        .expect("Login response should contain session_id");

    // Create multiple games
    for i in 0..3 {
        let create_req = test::TestRequest::post()
            .uri("/api/games")
            .insert_header(("Authorization", format!("Bearer {}", session_id)))
            .set_json(&json!({
                "name": format!("Game {}", i),
                "year_published": 2020 + i,
                "source": "bgg"
            }))
            .to_request();
        test::call_service(&app, create_req).await;
    }

    // Get all games
    let get_all_req = test::TestRequest::get()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let get_all_resp = test::call_service(&app, get_all_req).await;
    let status = get_all_resp.status();
    if !status.is_success() {
        let body_text = read_body_text(get_all_resp).await;
        panic!(
            "Get all games should succeed, got status: {}, body: {}",
            status, body_text
        );
    }
    let games: Vec<GameDto> = test::read_body_json(get_all_resp).await;
    assert!(games.len() >= 3);

    Ok(())
}

#[tokio::test]
async fn test_update_game() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.session_store.clone())
            .app_data(app_data.game_repo.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: std::sync::Arc::new(app_data.redis_data.get_ref().clone()),
                    })
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::update_game_handler),
            ),
    )
    .await;

    // Register, login, create game
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "update_game_user",
            "email": "update_game@example.com",
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
            "email": "update_game@example.com",
            "password": "password123"
        }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    let status = login_resp.status();
    if !status.is_success() {
        let body = read_body_text(login_resp).await;
        panic!(
            "Login should succeed, got status: {}, body: {}",
            status, body
        );
    }
    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let session_id = login_body["session_id"]
        .as_str()
        .expect("Login response should contain session_id");

    let create_req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "name": "Original Game",
            "year_published": 2020,
            "source": "bgg"
        }))
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    let status = create_resp.status();
    if !status.is_success() {
        let body_text = read_body_text(create_resp).await;
        panic!(
            "Create game should succeed, got status: {}, body: {}",
            status, body_text
        );
    }
    let game: GameDto = test::read_body_json(create_resp).await;
    let game_id = game.id.clone();

    // Update game
    let update_req = test::TestRequest::put()
        .uri(&format!("/api/games/{}", game_id))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "name": "Updated Game",
            "year_published": 2024,
            "source": "bgg"
        }))
        .to_request();

    let update_resp = test::call_service(&app, update_req).await;
    let status = update_resp.status();
    if !status.is_success() {
        let body_text = read_body_text(update_resp).await;
        panic!(
            "Update game should succeed, got status: {}, body: {}",
            status, body_text
        );
    }
    let updated: GameDto = test::read_body_json(update_resp).await;
    assert_eq!(updated.name, "Updated Game");
    assert_eq!(updated.year_published, Some(2024));
    assert_eq!(updated.id, game_id);

    Ok(())
}

#[tokio::test]
async fn test_delete_game() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.session_store.clone())
            .app_data(app_data.game_repo.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: std::sync::Arc::new(app_data.redis_data.get_ref().clone()),
                    })
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::delete_game_handler)
                    .service(backend::game::controller::get_game_handler),
            ),
    )
    .await;

    // Register, login, create game
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "delete_game_user",
            "email": "delete_game@example.com",
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
            "email": "delete_game@example.com",
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
    let session_id = login_body["session_id"]
        .as_str()
        .expect("Login response should contain session_id");

    let create_req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "name": "Delete Test Game",
            "year_published": 2023,
            "source": "bgg"
        }))
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    let game: GameDto = test::read_body_json(create_resp).await;
    let game_id = game.id.clone();

    // Delete game
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/games/{}", game_id))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let delete_resp = test::call_service(&app, delete_req).await;
    let delete_status = delete_resp.status();
    if !delete_status.is_success() {
        let body_bytes = test::read_body(delete_resp).await;
        let body_text = String::from_utf8_lossy(&body_bytes);
        panic!(
            "Delete game should succeed, got status: {}, body: {}",
            delete_status, body_text
        );
    }

    // Verify game is deleted
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/games/{}", game_id))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status().as_u16(), 404);

    Ok(())
}
