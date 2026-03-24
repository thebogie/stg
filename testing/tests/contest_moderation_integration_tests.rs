//! Integration tests: contest moderation (pending → not in public search → admin approves → visible).
//!
//! Requires SurrealDB + Redis: `./deploy/stack.sh start`
//!
//! Uses `ADMIN_EMAILS` for the approving user (same pattern as contest admin delete tests).

use actix_web::{test, web, App};
use anyhow::Result;
use chrono::{DateTime, FixedOffset, Utc};
use serde_json::json;
use shared::dto::contest::ContestDto;
use shared::dto::game::GameDto;
use shared::dto::venue::VenueDto;
use shared::models::contest_moderation::moderation_status;
use testing::app_setup;
use testing::create_authenticated_user;
use testing::TestEnvironment;

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
async fn contest_pending_not_in_search_until_approved() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let creator_email = format!("contest_mod_creator_{}@example.com", ts);
    let admin_email = format!("contest_mod_admin_{}@example.com", ts);
    let _admin_guard = AdminEmailsGuard::set_to(&admin_email);
    let place_id = format!("mod_place_{}", ts);
    let unique_name = format!("ModerationUniqueContest{}", ts);

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
                    .service(backend::contest::controller::search_contests_handler)
                    .service(backend::contest::controller::get_contest_handler)
                    .service(
                        web::scope("")
                            .wrap(backend::auth::AdminAuthMiddleware {
                                redis: app_data.redis_arc.clone(),
                                player_repo: app_data.player_repo_arc.clone(),
                            })
                            .app_data(app_data.contest_repo.clone())
                            .service(backend::contest::controller::list_pending_contests_handler)
                            .service(backend::contest::controller::approve_contest_handler)
                            .service(backend::contest::controller::reject_contest_handler)
                            .service(backend::contest::controller::delete_contest_handler),
                    ),
            ),
    )
    .await;

    let creator_session = create_authenticated_user!(app, creator_email.as_str(), "modcreator");
    let admin_session = create_authenticated_user!(app, admin_email.as_str(), "modadmin");

    let venue_req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header((
            "Authorization",
            format!("Bearer {}", creator_session),
        ))
        .set_json(&json!({
            "displayName": "Mod Test Venue",
            "formattedAddress": "1 Mod St",
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
        .insert_header((
            "Authorization",
            format!("Bearer {}", creator_session),
        ))
        .set_json(&json!({ "name": "Mod Test Game", "year_published": 2024, "source": "database" }))
        .to_request();
    let game_resp = test::call_service(&app, game_req).await;
    assert!(game_resp.status().is_success());
    let game: GameDto = test::read_body_json(game_resp).await;

    let start: DateTime<FixedOffset> = Utc::now().into();
    let stop: DateTime<FixedOffset> = (Utc::now() + chrono::Duration::hours(2)).into();
    let contest_req = test::TestRequest::post()
        .uri("/api/contests")
        .insert_header((
            "Authorization",
            format!("Bearer {}", creator_session),
        ))
        .set_json(&json!({
            "name": unique_name,
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
    assert_eq!(created.moderation_status, moderation_status::PENDING);

    let search_req = test::TestRequest::get()
        .uri(&format!(
            "/api/contests/search?q={}&scope=all&page=1&page_size=20",
            unique_name
        ))
        .insert_header((
            "Authorization",
            format!("Bearer {}", creator_session),
        ))
        .to_request();
    let search_resp = test::call_service(&app, search_req).await;
    assert!(search_resp.status().is_success());
    let search_json: serde_json::Value = test::read_body_json(search_resp).await;
    let total = search_json["total"].as_u64().unwrap_or(0);
    assert_eq!(total, 0, "pending contest must not appear in public search");

    let contest_key = key_only(&created.id);
    let approve_req = test::TestRequest::post()
        .uri(&format!("/api/contests/{}/approve", contest_key))
        .insert_header(("Authorization", format!("Bearer {}", admin_session)))
        .to_request();
    let approve_resp = test::call_service(&app, approve_req).await;
    assert_eq!(
        approve_resp.status(),
        actix_web::http::StatusCode::NO_CONTENT,
        "admin approve"
    );

    let search_after = test::TestRequest::get()
        .uri(&format!(
            "/api/contests/search?q={}&scope=all&page=1&page_size=20",
            unique_name
        ))
        .insert_header((
            "Authorization",
            format!("Bearer {}", creator_session),
        ))
        .to_request();
    let search_after_resp = test::call_service(&app, search_after).await;
    assert!(search_after_resp.status().is_success());
    let search_after_json: serde_json::Value = test::read_body_json(search_after_resp).await;
    let total_after = search_after_json["total"].as_u64().unwrap_or(0);
    assert!(
        total_after >= 1,
        "approved contest should appear in search"
    );

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn contest_approve_requires_admin() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

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
                    .service(backend::contest::controller::search_contests_handler)
                    .service(backend::contest::controller::get_contest_handler)
                    .service(
                        web::scope("")
                            .wrap(backend::auth::AdminAuthMiddleware {
                                redis: app_data.redis_arc.clone(),
                                player_repo: app_data.player_repo_arc.clone(),
                            })
                            .app_data(app_data.contest_repo.clone())
                            .service(backend::contest::controller::list_pending_contests_handler)
                            .service(backend::contest::controller::approve_contest_handler)
                            .service(backend::contest::controller::reject_contest_handler)
                            .service(backend::contest::controller::delete_contest_handler),
                    ),
            ),
    )
    .await;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let email = format!("contest_mod_nonadmin_{}@example.com", ts);
    let session_id = create_authenticated_user!(app, email.as_str(), "modnonadmin");

    let approve_req = test::TestRequest::post()
        .uri("/api/contests/00000000-0000-0000-0000-000000000001/approve")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    let approve_resp = test::call_service(&app, approve_req).await;
    assert_eq!(
        approve_resp.status(),
        actix_web::http::StatusCode::UNAUTHORIZED,
        "non-admin cannot approve"
    );

    Ok(())
}
