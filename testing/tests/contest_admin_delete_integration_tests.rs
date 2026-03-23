//! Integration tests: admin-only contest delete (create → verify present → delete → verify gone).
//!
//! Requires SurrealDB + Redis: `./deploy/stack.sh start`
//!
//! The roundtrip uses a unique email each run; admin delete is authorized by setting `ADMIN_EMAILS`
//! to that email for the duration of the test (same env-based admin path as production bootstrap).
//! A `Drop` guard restores the previous value so serial integration tests do not leak env state.

use actix_web::{test, web, App};
use anyhow::Result;
use backend::contest::repository::ContestRepository;
use chrono::{DateTime, FixedOffset, Utc};
use serde_json::json;
use shared::dto::contest::ContestDto;
use shared::dto::game::GameDto;
use shared::dto::venue::VenueDto;
use testing::create_authenticated_user;
use testing::{app_setup, TestEnvironment};

fn key_only(id: &str) -> &str {
    id.split_once('/').map(|(_, k)| k).unwrap_or(id)
}

struct AdminEmailsGuard {
    previous: Option<String>,
}

impl AdminEmailsGuard {
    fn set_to(email: &str) -> Self {
        let previous = std::env::var("ADMIN_EMAILS").ok();
        std::env::set_var("ADMIN_EMAILS", email);
        Self { previous }
    }
}

impl Drop for AdminEmailsGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(v) => std::env::set_var("ADMIN_EMAILS", v),
            None => std::env::remove_var("ADMIN_EMAILS"),
        }
    }
}

#[tokio::test]
#[serial_test::serial]
async fn contest_admin_create_delete_roundtrip() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let email = format!("contest_admin_del_{}@example.com", ts);
    let place_id = format!("del_place_{}", ts);
    let _admin_emails = AdminEmailsGuard::set_to(&email);

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.db.clone())
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
                    .service(backend::venue::controller::create_venue_handler)
                    .service(backend::venue::controller::get_venue_handler),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::get_game_handler),
            )
            .service(
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(app_data.player_repo.clone())
                    .service(backend::contest::controller::create_contest_handler)
                    .service(backend::contest::controller::get_contest_handler)
                    .service(
                        web::scope("")
                            .wrap(backend::auth::AdminAuthMiddleware {
                                redis: app_data.redis_arc.clone(),
                                player_repo: app_data.player_repo_arc.clone(),
                            })
                            .app_data(app_data.contest_repo.clone())
                            .service(backend::contest::controller::delete_contest_handler),
                    ),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, email.as_str(), "contestadmindel");

    let venue_req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "displayName": "Del Test Venue",
            "formattedAddress": "1 Delete St",
            "place_id": place_id,
            "lat": 41.0,
            "lng": -71.0,
            "timezone": "UTC",
            "source": "database"
        }))
        .to_request();
    let venue_resp = test::call_service(&app, venue_req).await;
    assert!(venue_resp.status().is_success());
    let venue: VenueDto = test::read_body_json(venue_resp).await;

    let game_req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({ "name": "Del Test Game", "year_published": 2024, "source": "database" }))
        .to_request();
    let game_resp = test::call_service(&app, game_req).await;
    assert!(game_resp.status().is_success());
    let game: GameDto = test::read_body_json(game_resp).await;

    let start: DateTime<FixedOffset> = Utc::now().into();
    let stop: DateTime<FixedOffset> = (Utc::now() + chrono::Duration::hours(1)).into();
    let contest_req = test::TestRequest::post()
        .uri("/api/contests")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "name": "Roundtrip Delete Contest",
            "start": start.to_rfc3339(),
            "stop": stop.to_rfc3339(),
            "venue": {
                "id": venue.id,
                "displayName": venue.display_name,
                "formattedAddress": venue.formatted_address,
                "place_id": venue.place_id,
                "lat": venue.lat,
                "lng": venue.lng,
                "timezone": venue.timezone,
                "source": "database"
            },
            "games": [{
                "id": game.id,
                "name": game.name,
                "year_published": game.year_published,
                "bgg_id": game.bgg_id,
                "description": game.description,
                "source": "database"
            }],
            "outcomes": []
        }))
        .to_request();
    let contest_resp = test::call_service(&app, contest_req).await;
    assert!(contest_resp.status().is_success(), "create contest");
    let created: ContestDto = test::read_body_json(contest_resp).await;
    assert!(created.id.starts_with("contest/"));

    assert!(
        app_data
            .contest_repo
            .find_by_id(&created.id)
            .await
            .is_some(),
        "repository should find contest after create"
    );

    let contest_key = key_only(&created.id);
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/contests/{}", contest_key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert!(get_resp.status().is_success(), "GET contest before delete");

    let del_req = test::TestRequest::delete()
        .uri(&format!("/api/contests/{}", contest_key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(
        del_resp.status(),
        actix_web::http::StatusCode::NO_CONTENT,
        "DELETE contest"
    );

    assert!(
        app_data
            .contest_repo
            .find_by_id(&created.id)
            .await
            .is_none(),
        "repository should not find contest after delete"
    );

    let get_after = test::TestRequest::get()
        .uri(&format!("/api/contests/{}", contest_key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    let get_after_resp = test::call_service(&app, get_after).await;
    assert_eq!(
        get_after_resp.status(),
        actix_web::http::StatusCode::NOT_FOUND,
        "GET contest after delete"
    );

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn contest_delete_requires_admin() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let email = format!("contest_nonadmin_del_{}@example.com", ts);

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(actix_web::web::JsonConfig::default().limit(256 * 1024))
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.db.clone())
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
                    .service(backend::venue::controller::create_venue_handler)
                    .service(backend::venue::controller::get_venue_handler),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::get_game_handler),
            )
            .service(
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(app_data.player_repo.clone())
                    .service(backend::contest::controller::create_contest_handler)
                    .service(backend::contest::controller::get_contest_handler)
                    .service(
                        web::scope("")
                            .wrap(backend::auth::AdminAuthMiddleware {
                                redis: app_data.redis_arc.clone(),
                                player_repo: app_data.player_repo_arc.clone(),
                            })
                            .app_data(app_data.contest_repo.clone())
                            .service(backend::contest::controller::delete_contest_handler),
                    ),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, email.as_str(), "contestnoadmdel");

    let del_req = test::TestRequest::delete()
        .uri("/api/contests/0000000000000000000000000")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(
        del_resp.status(),
        actix_web::http::StatusCode::UNAUTHORIZED,
        "non-admin should not reach delete handler"
    );

    Ok(())
}
