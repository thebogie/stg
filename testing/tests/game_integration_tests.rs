//! Comprehensive integration tests for Game API endpoints

use anyhow::Result;
use actix_web::{test, web, App};
use serde_json::json;
use testing::{TestEnvironment, app_setup};
use shared::dto::game::GameDto;

#[tokio::test]
async fn test_create_game() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;
    
    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
            .app_data(app_data.game_repo.clone())
            .service(web::scope("/api/games")
                .wrap(backend::auth::AuthMiddleware { 
                    redis: std::sync::Arc::new(app_data.redis_data.get_ref().clone()) 
                })
                .service(backend::game::controller::create_game_handler)
            )
    ).await;

    // Register and login
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "game_test_user",
            "email": "game_test@example.com",
            "password": "password123"
        }))
        .to_request();
    test::call_service(&app, register_req).await;
    
    let login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": "game_test@example.com",
            "password": "password123"
        }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let session_id = login_body["session_id"].as_str().unwrap();

    // Create game
    let game_data = json!({
        "name": "Test Game",
        "year_published": 2023
    });

    let req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&game_data)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
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
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(app_data.game_repo.clone())
            .service(web::scope("/api/games")
                .wrap(backend::auth::AuthMiddleware { 
                    redis: std::sync::Arc::new(app_data.redis_data.get_ref().clone()) 
                })
                .service(backend::game::controller::create_game_handler)
                .service(backend::game::controller::get_all_games_handler)
            )
    ).await;

    // Register and login
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "all_games_user",
            "email": "all_games@example.com",
            "password": "password123"
        }))
        .to_request();
    test::call_service(&app, register_req).await;
    
    let login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": "all_games@example.com",
            "password": "password123"
        }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let session_id = login_body["session_id"].as_str().unwrap();

    // Create multiple games
    for i in 0..3 {
        let create_req = test::TestRequest::post()
            .uri("/api/games")
            .insert_header(("Authorization", format!("Bearer {}", session_id)))
            .set_json(&json!({
                "name": format!("Game {}", i),
                "year_published": 2020 + i
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
    assert!(get_all_resp.status().is_success());
    
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
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
            .app_data(app_data.game_repo.clone())
            .service(web::scope("/api/games")
                .wrap(backend::auth::AuthMiddleware { 
                    redis: std::sync::Arc::new(app_data.redis_data.get_ref().clone()) 
                })
                .service(backend::game::controller::create_game_handler)
                .service(backend::game::controller::update_game_handler)
            )
    ).await;

    // Register, login, create game
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "update_game_user",
            "email": "update_game@example.com",
            "password": "password123"
        }))
        .to_request();
    test::call_service(&app, register_req).await;
    
    let login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": "update_game@example.com",
            "password": "password123"
        }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let session_id = login_body["session_id"].as_str().unwrap();

    let create_req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "name": "Original Game",
            "year_published": 2020
        }))
        .to_request();
    
    let create_resp = test::call_service(&app, create_req).await;
    let game: GameDto = test::read_body_json(create_resp).await;
    let game_id = game.id.clone();

    // Update game
    let update_req = test::TestRequest::put()
        .uri(&format!("/api/games/{}", game_id))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "name": "Updated Game",
            "year_published": 2024
        }))
        .to_request();

    let update_resp = test::call_service(&app, update_req).await;
    assert!(update_resp.status().is_success());
    
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
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(app_data.game_repo.clone())
            .service(web::scope("/api/games")
                .wrap(backend::auth::AuthMiddleware { 
                    redis: std::sync::Arc::new(app_data.redis_data.get_ref().clone()) 
                })
                .service(backend::game::controller::create_game_handler)
                .service(backend::game::controller::delete_game_handler)
                .service(backend::game::controller::get_game_handler)
            )
    ).await;

    // Register, login, create game
    let register_req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "delete_game_user",
            "email": "delete_game@example.com",
            "password": "password123"
        }))
        .to_request();
    test::call_service(&app, register_req).await;
    
    let login_req = test::TestRequest::post()
        .uri("/api/players/login")
        .set_json(&json!({
            "email": "delete_game@example.com",
            "password": "password123"
        }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let session_id = login_body["session_id"].as_str().unwrap();

    let create_req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "name": "Delete Test Game",
            "year_published": 2023
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
    assert!(delete_resp.status().is_success());

    // Verify game is deleted
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/games/{}", game_id))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status().as_u16(), 404);

    Ok(())
}

