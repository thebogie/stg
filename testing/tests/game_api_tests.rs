//! Integration tests for Game API endpoints
//!
//! Tests complete CRUD operations for games with real database and Redis

//! Integration tests for Game API endpoints
//!
//! Tests complete CRUD operations for games with real database and Redis

mod test_helpers;

use actix_web::{test, web, App};
use anyhow::Result;
use serde_json::json;
use shared::dto::game::GameDto;
use shared::models::game::GameSource;
use testing::create_authenticated_user;
use testing::{app_setup, TestEnvironment};

#[tokio::test]
async fn test_create_game_success() -> Result<()> {
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
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler)
                    .service(backend::game::controller::get_game_handler)
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::update_game_handler)
                    .service(backend::game::controller::delete_game_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "game_test@example.com", "gameuser");

    let game_data = json!({
        "name": "Test Game",
        "year_published": 2020,
        "bgg_id": 12345,
        "description": "A test game",
        "source": "database"
    });

    let req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&game_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(
        resp.status().is_success(),
        "Creating game should succeed, got status: {}",
        resp.status()
    );

    let game: GameDto = test::read_body_json(resp).await;
    assert_eq!(game.name, "Test Game");
    assert_eq!(game.year_published, Some(2020));
    assert_eq!(game.bgg_id, Some(12345));
    assert!(!game.id.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_create_game_validation_errors() -> Result<()> {
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
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler)
                    .service(backend::game::controller::get_game_handler)
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::update_game_handler)
                    .service(backend::game::controller::delete_game_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "game_val@example.com", "gameval");

    // Test empty name
    let invalid_game = json!({
        "name": "",
        "source": "database"
    });

    let req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&invalid_game)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_client_error(),
        "Invalid game should return 4xx, got: {}",
        resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_get_game_success() -> Result<()> {
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
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler)
                    .service(backend::game::controller::get_game_handler)
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::update_game_handler)
                    .service(backend::game::controller::delete_game_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "game_get@example.com", "gameget");

    // First create a game
    let game_data = json!({
        "name": "Get Test Game",
        "year_published": 2021,
        "source": "database"
    });

    let create_req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&game_data)
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    assert!(create_resp.status().is_success());
    let created_game: GameDto = test::read_body_json(create_resp).await;
    let game_id = created_game.id.clone();

    // Now get it
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/games/{}", game_id))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let get_resp = test::call_service(&app, get_req).await;
    assert!(
        get_resp.status().is_success(),
        "Getting game should succeed, got: {}",
        get_resp.status()
    );

    let retrieved_game: GameDto = test::read_body_json(get_resp).await;
    assert_eq!(retrieved_game.id, game_id);
    assert_eq!(retrieved_game.name, "Get Test Game");

    Ok(())
}

#[tokio::test]
async fn test_get_game_not_found() -> Result<()> {
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
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler)
                    .service(backend::game::controller::get_game_handler)
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::update_game_handler)
                    .service(backend::game::controller::delete_game_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "game_nf@example.com", "gamenf");

    let req = test::TestRequest::get()
        .uri("/api/games/game/nonexistent_12345")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "Non-existent game should return 404, got: {}",
        resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_update_game_success() -> Result<()> {
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
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler)
                    .service(backend::game::controller::get_game_handler)
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::update_game_handler)
                    .service(backend::game::controller::delete_game_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "game_upd@example.com", "gameupd");

    // Create a game first
    let game_data = json!({
        "name": "Original Name",
        "year_published": 2020,
        "source": "database"
    });

    let create_req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&game_data)
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    assert!(create_resp.status().is_success());
    let created_game: GameDto = test::read_body_json(create_resp).await;
    let game_id = created_game.id.clone();

    // Update it
    let updated_data = json!({
        "name": "Updated Name",
        "year_published": 2022,
        "description": "Updated description",
        "source": "database"
    });

    let update_req = test::TestRequest::put()
        .uri(&format!("/api/games/{}", game_id))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&updated_data)
        .to_request();

    let update_resp = test::call_service(&app, update_req).await;
    assert!(
        update_resp.status().is_success(),
        "Updating game should succeed, got: {}",
        update_resp.status()
    );

    let updated_game: GameDto = test::read_body_json(update_resp).await;
    assert_eq!(updated_game.name, "Updated Name");
    assert_eq!(updated_game.year_published, Some(2022));

    Ok(())
}

#[tokio::test]
async fn test_update_game_not_found() -> Result<()> {
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
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler)
                    .service(backend::game::controller::get_game_handler)
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::update_game_handler)
                    .service(backend::game::controller::delete_game_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "game_upd_nf@example.com", "gameupdnf");

    let update_data = json!({
        "name": "Updated Name",
        "source": "database"
    });

    let req = test::TestRequest::put()
        .uri("/api/games/game/nonexistent_99999")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&update_data)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "Updating non-existent game should return 404, got: {}",
        resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_delete_game_success() -> Result<()> {
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
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler)
                    .service(backend::game::controller::get_game_handler)
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::update_game_handler)
                    .service(backend::game::controller::delete_game_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "game_del@example.com", "gamedel");

    // Create a game first
    let game_data = json!({
        "name": "To Delete",
        "source": "database"
    });

    let create_req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&game_data)
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    assert!(create_resp.status().is_success());
    let created_game: GameDto = test::read_body_json(create_resp).await;
    let game_id = created_game.id.clone();

    // Delete it
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/games/{}", game_id))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let delete_resp = test::call_service(&app, delete_req).await;
    assert!(
        delete_resp.status().is_success(),
        "Deleting game should succeed, got: {}",
        delete_resp.status()
    );

    // Verify it's deleted
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/games/{}", game_id))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(
        get_resp.status(),
        404,
        "Deleted game should return 404, got: {}",
        get_resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_delete_game_not_found() -> Result<()> {
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
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler)
                    .service(backend::game::controller::get_game_handler)
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::update_game_handler)
                    .service(backend::game::controller::delete_game_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "game_del_nf@example.com", "gamedelnf");

    let req = test::TestRequest::delete()
        .uri("/api/games/game/nonexistent_99999")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "Deleting non-existent game should return 404, got: {}",
        resp.status()
    );

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
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler)
                    .service(backend::game::controller::get_game_handler)
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::update_game_handler)
                    .service(backend::game::controller::delete_game_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "game_list@example.com", "gamelist");

    // Create a couple of games
    for i in 1..=3 {
        let game_data = json!({
            "name": format!("Game {}", i),
            "year_published": 2020 + i,
            "source": "database"
        });

        let create_req = test::TestRequest::post()
            .uri("/api/games")
            .insert_header(("Authorization", format!("Bearer {}", session_id)))
            .set_json(&game_data)
            .to_request();

        let create_resp = test::call_service(&app, create_req).await;
        assert!(create_resp.status().is_success());
    }

    // Get all games
    let req = test::TestRequest::get()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Getting all games should succeed, got: {}",
        resp.status()
    );

    let games: Vec<GameDto> = test::read_body_json(resp).await;
    assert!(games.len() >= 3, "Should have at least 3 games");

    Ok(())
}

#[tokio::test]
async fn test_game_unauthorized_access() -> Result<()> {
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
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler)
                    .service(backend::game::controller::get_game_handler)
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::update_game_handler)
                    .service(backend::game::controller::delete_game_handler),
            ),
    )
    .await;

    // Try to access without authentication
    let req = test::TestRequest::get().uri("/api/games").to_request();

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
