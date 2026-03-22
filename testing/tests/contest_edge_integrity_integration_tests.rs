//! Integration test: contest edge integrity (contest ↔ venue/game edges).
//!
//! Value: verifies "plumbing" between API handlers + SurrealDB (tables + edges)
//! without re-testing internal unit logic.

use actix_web::{test, web, App};
use anyhow::Result;
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

#[tokio::test]
async fn contest_create_then_get_has_edge_ids() -> Result<()> {
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
                    .service(backend::contest::controller::get_contest_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "edge_contest@example.com", "edgecontest");

    // Create venue
    let venue_req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "displayName": "Edge Venue",
            "formattedAddress": "10 Edge St",
            "place_id": "edge_place_id",
            "lat": 40.0,
            "lng": -70.0,
            "timezone": "UTC",
            "source": "database"
        }))
        .to_request();
    let venue_resp = test::call_service(&app, venue_req).await;
    assert!(venue_resp.status().is_success());
    let venue: VenueDto = test::read_body_json(venue_resp).await;
    assert!(venue.id.starts_with("venue/"));

    // Create game
    let game_req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({ "name": "Edge Game", "year_published": 2024, "source": "database" }))
        .to_request();
    let game_resp = test::call_service(&app, game_req).await;
    assert!(game_resp.status().is_success());
    let game: GameDto = test::read_body_json(game_resp).await;
    assert!(game.id.starts_with("game/"));

    // Create contest referencing those ids
    let start: DateTime<FixedOffset> = Utc::now().into();
    let stop: DateTime<FixedOffset> = (Utc::now() + chrono::Duration::hours(2)).into();
    let contest_req = test::TestRequest::post()
        .uri("/api/contests")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "name": "Edge Contest",
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
    assert!(contest_resp.status().is_success());
    let created: ContestDto = test::read_body_json(contest_resp).await;
    assert!(created.id.starts_with("contest/"));
    assert!(created.venue.id.starts_with("venue/"));
    assert!(!created.games.is_empty());
    assert!(created.games[0].id.starts_with("game/"));

    // GET contest by key-only and assert ids are present and stable
    let contest_key = key_only(&created.id);
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/contests/{}", contest_key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert!(get_resp.status().is_success());
    let fetched: ContestDto = test::read_body_json(get_resp).await;

    assert_eq!(fetched.id, created.id);
    assert!(fetched.venue.id.starts_with("venue/"));
    assert!(!fetched.games.is_empty());
    assert!(fetched.games[0].id.starts_with("game/"));

    Ok(())
}
