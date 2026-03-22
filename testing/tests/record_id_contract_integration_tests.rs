//! Integration tests: record IDs in **JSON** match the product contract while **HTTP paths** use raw keys.
//!
//! Contract (see `docs/api/RESOURCE_IDS_HTTP.md` and `front/web/src/api/games.rs`):
//! - Responses use canonical `table/key` in `_id`.
//! - `GET/PUT/DELETE` use a **single path segment** = raw record key (UUID), not `…/game/<uuid>` as two segments.

use actix_web::{test, web, App};
use anyhow::Result;
use serde_json::json;
use shared::dto::game::GameDto;
use shared::dto::venue::VenueDto;
use testing::create_authenticated_user;
use testing::{app_setup, TestEnvironment};

fn key_only(id: &str) -> &str {
    id.split_once('/').map(|(_, k)| k).unwrap_or(id)
}

#[tokio::test]
async fn game_json_ids_canonical_http_uses_raw_key() -> Result<()> {
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
                    .service(backend::game::controller::get_game_handler)
                    .service(backend::game::controller::create_game_handler)
                    .service(backend::game::controller::update_game_handler)
                    .service(backend::game::controller::delete_game_handler),
            ),
    )
    .await;

    let email = format!(
        "rid-game-{}@example.com",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_millis()
    );
    let session_id = create_authenticated_user!(app, email.as_str(), "ridgame");

    let create_req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(
            &json!({ "name": "RID Contract Game", "year_published": 2024, "source": "database" }),
        )
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    if !create_resp.status().is_success() {
        let status = create_resp.status();
        let body = test::read_body(create_resp).await;
        panic!(
            "POST /api/games should succeed (status={} body={})",
            status,
            String::from_utf8_lossy(&body)
        );
    }
    let created: GameDto = test::read_body_json(create_resp).await;
    assert!(
        created.id.starts_with("game/"),
        "JSON _id must be canonical game/<key>"
    );
    let key = key_only(&created.id).to_string();

    // GET by raw key (same as web client)
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/games/{}", key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    if !get_resp.status().is_success() {
        let status = get_resp.status();
        let body = test::read_body(get_resp).await;
        panic!(
            "GET /api/games/{{key}} should succeed (created_id={} key={} status={} body={})",
            created.id,
            key,
            status,
            String::from_utf8_lossy(&body)
        );
    }
    let fetched: GameDto = test::read_body_json(get_resp).await;
    assert_eq!(fetched.id, created.id, "Stable canonical id round-trip");

    let update_req = test::TestRequest::put()
        .uri(&format!("/api/games/{}", key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({ "id": created.id, "name": "RID Contract Game Updated", "year_published": 2024, "source": "database" }))
        .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert!(update_resp.status().is_success());

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/games/{}", key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;
    assert!(delete_resp.status().is_success());

    Ok(())
}

#[tokio::test]
async fn venue_json_ids_canonical_http_uses_raw_key() -> Result<()> {
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
            .app_data(app_data.venue_repo.clone())
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
                    .service(backend::venue::controller::get_venue_handler)
                    .service(backend::venue::controller::create_venue_handler)
                    .service(backend::venue::controller::update_venue_handler)
                    .service(backend::venue::controller::delete_venue_handler),
            ),
    )
    .await;

    let email = format!(
        "rid-venue-{}@example.com",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_millis()
    );
    let session_id = create_authenticated_user!(app, email.as_str(), "ridvenue");

    let create_req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "displayName": "RID Contract Venue",
            "formattedAddress": "1 Contract St, Test City",
            "place_id": "rid_contract_place",
            "lat": 40.0,
            "lng": -70.0,
            "timezone": "UTC",
            "source": "database"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert!(create_resp.status().is_success());
    let created: VenueDto = test::read_body_json(create_resp).await;
    assert!(
        created.id.starts_with("venue/"),
        "JSON _id must be canonical venue/<key>"
    );
    let key = key_only(&created.id).to_string();

    let get_req = test::TestRequest::get()
        .uri(&format!("/api/venues/{}", key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert!(get_resp.status().is_success());

    let update_req = test::TestRequest::put()
        .uri(&format!("/api/venues/{}", key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&json!({
            "id": created.id,
            "displayName": "RID Contract Venue Updated",
            "formattedAddress": "1 Contract St, Test City",
            "place_id": "rid_contract_place",
            "lat": 40.0,
            "lng": -70.0,
            "timezone": "UTC",
            "source": "database"
        }))
        .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert!(update_resp.status().is_success());

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/venues/{}", key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;
    assert!(delete_resp.status().is_success());

    Ok(())
}
