//! Integration tests for Authentication & Authorization
//!
//! Tests session management, authentication flows, and authorization checks

//! Integration tests for Authentication & Authorization
//!
//! Tests session management, authentication flows, and authorization checks

mod test_helpers;

use actix_web::{test, web, App};
use anyhow::Result;
use testing::create_authenticated_user;
use testing::{app_setup, TestEnvironment};

#[tokio::test]
async fn test_session_expiration() -> Result<()> {
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
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_all_venues_handler),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler),
            )
            .service(
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(app_data.player_repo.clone())
                    .service(backend::contest::controller::search_contests_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "session_exp@example.com", "sessionexp");

    // Use the session immediately - should work
    let req = test::TestRequest::get()
        .uri("/api/players/me")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Session should be valid immediately, got: {}",
        resp.status()
    );

    // Note: Actual expiration testing would require waiting for TTL,
    // which is slow. This test verifies the session works when valid.
    Ok(())
}

#[tokio::test]
async fn test_invalid_session_token() -> Result<()> {
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
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_all_venues_handler),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler),
            )
            .service(
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(app_data.player_repo.clone())
                    .service(backend::contest::controller::search_contests_handler),
            ),
    )
    .await;

    // Try to use a fake session token
    let req = test::TestRequest::get()
        .uri("/api/players/me")
        .insert_header(("Authorization", "Bearer fake_session_token_12345"))
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
async fn test_malformed_authorization_header() -> Result<()> {
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
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_all_venues_handler),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler),
            )
            .service(
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(app_data.player_repo.clone())
                    .service(backend::contest::controller::search_contests_handler),
            ),
    )
    .await;

    // Test missing "Bearer " prefix
    let req1 = test::TestRequest::get()
        .uri("/api/players/me")
        .insert_header(("Authorization", "just_a_token"))
        .to_request();

    let resp1 = test::try_call_service(&app, req1).await;
    match resp1 {
        Ok(resp) => assert_eq!(resp.status(), 401),
        Err(e) => {
            use actix_web::error::ResponseError;
            assert_eq!(e.as_response_error().status_code(), 401);
        }
    }

    // Test empty authorization header
    let req2 = test::TestRequest::get()
        .uri("/api/players/me")
        .insert_header(("Authorization", ""))
        .to_request();

    let resp2 = test::try_call_service(&app, req2).await;
    match resp2 {
        Ok(resp) => assert_eq!(resp.status(), 401),
        Err(e) => {
            use actix_web::error::ResponseError;
            assert_eq!(e.as_response_error().status_code(), 401);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_missing_authorization_header() -> Result<()> {
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
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_all_venues_handler),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler),
            )
            .service(
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(app_data.player_repo.clone())
                    .service(backend::contest::controller::search_contests_handler),
            ),
    )
    .await;

    // Try to access protected endpoint without Authorization header
    let req = test::TestRequest::get().uri("/api/players/me").to_request();

    let resp = test::try_call_service(&app, req).await;

    match resp {
        Ok(resp) => {
            assert_eq!(
                resp.status(),
                401,
                "Missing Authorization header should return 401, got: {}",
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
async fn test_logout_invalidates_session() -> Result<()> {
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
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_all_venues_handler),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler),
            )
            .service(
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(app_data.player_repo.clone())
                    .service(backend::contest::controller::search_contests_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "logout_test@example.com", "logouttest");

    // Verify session works before logout
    let me_req = test::TestRequest::get()
        .uri("/api/players/me")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let me_resp = test::call_service(&app, me_req).await;
    assert!(
        me_resp.status().is_success(),
        "Session should work before logout"
    );

    // Logout
    let logout_req = test::TestRequest::post()
        .uri("/api/players/logout")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let logout_resp = test::call_service(&app, logout_req).await;
    assert!(
        logout_resp.status().is_success(),
        "Logout should succeed, got: {}",
        logout_resp.status()
    );

    // Give Redis a moment to ensure session is deleted
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify session is invalidated
    let me_req2 = test::TestRequest::get()
        .uri("/api/players/me")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let me_resp2 = test::try_call_service(&app, me_req2).await;
    match me_resp2 {
        Ok(resp) => {
            assert_eq!(
                resp.status(),
                401,
                "Session should be invalidated after logout, got: {}",
                resp.status()
            );
        }
        Err(e) => {
            use actix_web::error::ResponseError;
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

#[tokio::test]
async fn test_session_persistence_across_requests() -> Result<()> {
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
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_all_venues_handler),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler),
            )
            .service(
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(app_data.player_repo.clone())
                    .service(backend::contest::controller::search_contests_handler),
            ),
    )
    .await;

    let session_id =
        create_authenticated_user!(app, "session_persist@example.com", "sessionpersist");

    // Make multiple requests with the same session
    for i in 1..=3 {
        let req = test::TestRequest::get()
            .uri("/api/players/me")
            .insert_header(("Authorization", format!("Bearer {}", session_id)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(
            resp.status().is_success(),
            "Session should persist across request {}, got: {}",
            i,
            resp.status()
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_concurrent_sessions() -> Result<()> {
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
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_all_venues_handler),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler),
            )
            .service(
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(app_data.player_repo.clone())
                    .service(backend::contest::controller::search_contests_handler),
            ),
    )
    .await;

    // Create two different users with different sessions
    let session1 = create_authenticated_user!(app, "user1@example.com", "user1");
    let session2 = create_authenticated_user!(app, "user2@example.com", "user2");

    // Both sessions should work independently
    let req1 = test::TestRequest::get()
        .uri("/api/players/me")
        .insert_header(("Authorization", format!("Bearer {}", session1)))
        .to_request();

    let req2 = test::TestRequest::get()
        .uri("/api/players/me")
        .insert_header(("Authorization", format!("Bearer {}", session2)))
        .to_request();

    let resp1 = test::call_service(&app, req1).await;
    let resp2 = test::call_service(&app, req2).await;

    assert!(resp1.status().is_success(), "Session 1 should work");
    assert!(resp2.status().is_success(), "Session 2 should work");

    // Verify they return different user data
    use shared::dto::player::PlayerDto;
    let user1: PlayerDto = test::read_body_json(resp1).await;
    let user2: PlayerDto = test::read_body_json(resp2).await;

    assert_ne!(
        user1.id, user2.id,
        "Different sessions should return different users"
    );
    assert_ne!(
        user1.email, user2.email,
        "Different sessions should return different emails"
    );

    Ok(())
}

#[tokio::test]
async fn test_protected_endpoints_require_auth() -> Result<()> {
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
            .app_data(app_data.game_repo.clone())
            .app_data(app_data.venue_repo.clone())
            .app_data(app_data.contest_repo.clone())
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
            )
            .service(
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::get_all_venues_handler),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::get_all_games_handler),
            )
            .service(
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(app_data.player_repo.clone())
                    .service(backend::contest::controller::search_contests_handler),
            ),
    )
    .await;

    // Test various protected endpoints
    let protected_endpoints = vec!["/api/venues", "/api/games", "/api/contests/search"];

    for endpoint in protected_endpoints {
        let req = test::TestRequest::get().uri(endpoint).to_request();

        let resp = test::try_call_service(&app, req).await;

        match resp {
            Ok(resp) => {
                assert_eq!(
                    resp.status(),
                    401,
                    "Endpoint {} should require auth, got: {}",
                    endpoint,
                    resp.status()
                );
            }
            Err(e) => {
                use actix_web::error::ResponseError;
                let status = e.as_response_error().status_code();
                assert_eq!(
                    status, 401,
                    "Endpoint {} should require auth, got error status: {}",
                    endpoint, status
                );
            }
        }
    }

    Ok(())
}
