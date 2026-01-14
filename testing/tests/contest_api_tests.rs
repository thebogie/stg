//! Integration tests for Contest API endpoints
//!
//! Tests complete CRUD operations for contests with real database and Redis

//! Integration tests for Contest API endpoints
//!
//! Tests complete CRUD operations for contests with real database and Redis

mod test_helpers;

use actix_web::{test, web, App};
use anyhow::Result;
use chrono::{DateTime, FixedOffset, Utc};
use serde_json::json;
use shared::dto::contest::ContestDto;
use shared::dto::game::GameDto;
use shared::dto::venue::VenueDto;
use shared::models::game::GameSource;
use shared::models::venue::VenueSource;
use testing::create_authenticated_user;
use testing::{app_setup, TestEnvironment};

fn create_test_venue_dto() -> VenueDto {
    VenueDto {
        id: String::new(),
        display_name: "Test Venue".to_string(),
        formatted_address: "123 Test St, Test City".to_string(),
        place_id: "test_place_id".to_string(),
        lat: 40.7128,
        lng: -74.0060,
        timezone: "America/New_York".to_string(),
        source: VenueSource::Database,
    }
}

fn create_test_game_dto() -> GameDto {
    GameDto {
        id: String::new(),
        name: "Test Game".to_string(),
        year_published: Some(2020),
        bgg_id: None,
        description: Some("A test game".to_string()),
        source: GameSource::Database,
    }
}

#[tokio::test]
async fn test_create_contest_success() -> Result<()> {
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
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(app_data.player_repo.clone())
                    .service(backend::contest::controller::create_contest_handler)
                    .service(backend::contest::controller::search_contests_handler)
                    .service(backend::contest::controller::get_contest_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "contest_test@example.com", "contestuser");

    let start: DateTime<FixedOffset> = Utc::now().into();
    let stop: DateTime<FixedOffset> = start + chrono::Duration::hours(2);

    let contest_data = json!({
        "name": "Test Contest",
        "start": start.to_rfc3339(),
        "stop": stop.to_rfc3339(),
        "venue": {
            "displayName": "Test Venue",
            "formattedAddress": "123 Test St",
            "place_id": "test_place_id",
            "lat": 40.7128,
            "lng": -74.0060,
            "timezone": "America/New_York",
            "source": "database"
        },
        "games": [{
            "name": "Test Game",
            "year_published": 2020,
            "source": "database"
        }],
        "outcomes": []
    });

    let req = test::TestRequest::post()
        .uri("/api/contests")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&contest_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(
        resp.status().is_success(),
        "Creating contest should succeed, got status: {}",
        resp.status()
    );

    let contest: ContestDto = test::read_body_json(resp).await;
    assert_eq!(contest.name, "Test Contest");
    assert!(!contest.id.is_empty());
    assert!(!contest.creator_id.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_create_contest_validation_errors() -> Result<()> {
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
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(app_data.player_repo.clone())
                    .service(backend::contest::controller::create_contest_handler)
                    .service(backend::contest::controller::search_contests_handler)
                    .service(backend::contest::controller::get_contest_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "contest_val@example.com", "contestval");

    let start: DateTime<FixedOffset> = Utc::now().into();
    let stop: DateTime<FixedOffset> = start - chrono::Duration::hours(1); // Invalid: stop before start

    // Test invalid date range (stop before start)
    let invalid_contest = json!({
        "name": "Invalid Contest",
        "start": start.to_rfc3339(),
        "stop": stop.to_rfc3339(),
        "venue": {
            "displayName": "Test Venue",
            "formattedAddress": "123 Test St",
            "place_id": "test_place_id",
            "lat": 40.7128,
            "lng": -74.0060,
            "timezone": "America/New_York",
            "source": "database"
        },
        "games": [],
        "outcomes": []
    });

    let req = test::TestRequest::post()
        .uri("/api/contests")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&invalid_contest)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_client_error(),
        "Invalid contest (stop before start) should return 4xx, got: {}",
        resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_get_contest_success() -> Result<()> {
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
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(app_data.player_repo.clone())
                    .service(backend::contest::controller::create_contest_handler)
                    .service(backend::contest::controller::search_contests_handler)
                    .service(backend::contest::controller::get_contest_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "contest_get@example.com", "contestget");

    // First create a contest
    let start: DateTime<FixedOffset> = Utc::now().into();
    let stop: DateTime<FixedOffset> = start + chrono::Duration::hours(2);

    let contest_data = json!({
        "name": "Get Test Contest",
        "start": start.to_rfc3339(),
        "stop": stop.to_rfc3339(),
        "venue": {
            "displayName": "Test Venue",
            "formattedAddress": "123 Test St",
            "place_id": "test_place_id",
            "lat": 40.7128,
            "lng": -74.0060,
            "timezone": "America/New_York",
            "source": "database"
        },
        "games": [],
        "outcomes": []
    });

    let create_req = test::TestRequest::post()
        .uri("/api/contests")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&contest_data)
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    assert!(create_resp.status().is_success());
    let created_contest: ContestDto = test::read_body_json(create_resp).await;
    let contest_id = created_contest.id.clone();

    // Extract just the ID part if it's in format "contest/123"
    let contest_id_for_url = if contest_id.contains('/') {
        contest_id
            .split('/')
            .last()
            .unwrap_or(&contest_id)
            .to_string()
    } else {
        contest_id.clone()
    };

    // Now get it
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/contests/{}", contest_id_for_url))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let get_resp = test::call_service(&app, get_req).await;
    assert!(
        get_resp.status().is_success(),
        "Getting contest should succeed, got: {}",
        get_resp.status()
    );

    let retrieved_contest: ContestDto = test::read_body_json(get_resp).await;
    assert_eq!(retrieved_contest.id, contest_id);
    assert_eq!(retrieved_contest.name, "Get Test Contest");

    Ok(())
}

#[tokio::test]
async fn test_get_contest_not_found() -> Result<()> {
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
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(app_data.player_repo.clone())
                    .service(backend::contest::controller::create_contest_handler)
                    .service(backend::contest::controller::search_contests_handler)
                    .service(backend::contest::controller::get_contest_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "contest_nf@example.com", "contestnf");

    let req = test::TestRequest::get()
        .uri("/api/contests/contest/nonexistent_12345")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "Non-existent contest should return 404, got: {}",
        resp.status()
    );

    Ok(())
}

#[tokio::test]
async fn test_search_contests() -> Result<()> {
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
                web::scope("/api/venues")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::venue::controller::create_venue_handler),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(64 * 1024))
                    .service(backend::game::controller::create_game_handler),
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
                    .service(backend::contest::controller::get_contest_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "contest_search@example.com", "contestsearch");

    // First create a venue and game that we'll reuse
    let venue_data = json!({
        "displayName": "Search Test Venue",
        "formattedAddress": "123 Search St",
        "place_id": "search_place_id",
        "lat": 40.7128,
        "lng": -74.0060,
        "timezone": "America/New_York",
        "source": "database"
    });

    let create_venue_req = test::TestRequest::post()
        .uri("/api/venues")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&venue_data)
        .to_request();

    let create_venue_resp = test::call_service(&app, create_venue_req).await;
    assert!(create_venue_resp.status().is_success());
    let created_venue: shared::dto::venue::VenueDto = test::read_body_json(create_venue_resp).await;

    let game_data = json!({
        "name": "Search Test Game",
        "year_published": 2020,
        "source": "database"
    });

    let create_game_req = test::TestRequest::post()
        .uri("/api/games")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(&game_data)
        .to_request();

    let create_game_resp = test::call_service(&app, create_game_req).await;
    assert!(create_game_resp.status().is_success());
    let created_game: shared::dto::game::GameDto = test::read_body_json(create_game_resp).await;

    // Create a couple of contests with venue and games
    for i in 1..=2 {
        let start: DateTime<FixedOffset> = Utc::now().into();
        let stop: DateTime<FixedOffset> = start + chrono::Duration::hours(2);

        let contest_data = json!({
            "name": format!("Contest {}", i),
            "start": start.to_rfc3339(),
            "stop": stop.to_rfc3339(),
            "venue": {
                "id": created_venue.id,
                "displayName": created_venue.display_name,
                "formattedAddress": created_venue.formatted_address,
                "place_id": created_venue.place_id,
                "lat": created_venue.lat,
                "lng": created_venue.lng,
                "timezone": created_venue.timezone,
                "source": "database"
            },
            "games": [{
                "id": created_game.id,
                "name": created_game.name,
                "year_published": created_game.year_published,
                "source": "database"
            }],
            "outcomes": []
        });

        let create_req = test::TestRequest::post()
            .uri("/api/contests")
            .insert_header(("Authorization", format!("Bearer {}", session_id)))
            .set_json(&contest_data)
            .to_request();

        let create_resp = test::call_service(&app, create_req).await;
        assert!(create_resp.status().is_success());
    }

    // Search contests - use scope=all to see all contests (not just user's)
    let req = test::TestRequest::get()
        .uri("/api/contests/search?scope=all")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Searching contests should succeed, got: {}",
        resp.status()
    );

    // Search returns a paginated response with items, total, page, page_size
    let search_result: serde_json::Value = test::read_body_json(resp).await;
    let items = search_result
        .get("items")
        .and_then(|v| v.as_array())
        .expect("Response should have 'items' array");
    let total = search_result
        .get("total")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // We created 2 contests, so we should see at least 2 (or more if there's existing data)
    assert!(
        total >= 2 || items.len() >= 2,
        "Should have at least 2 contests (total: {}, items: {})",
        total,
        items.len()
    );

    Ok(())
}

#[tokio::test]
async fn test_contest_unauthorized_access() -> Result<()> {
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
                web::scope("/api/contests")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
                    .app_data(app_data.player_repo.clone())
                    .service(backend::contest::controller::create_contest_handler)
                    .service(backend::contest::controller::search_contests_handler)
                    .service(backend::contest::controller::get_contest_handler),
            ),
    )
    .await;

    // Try to access without authentication
    let req = test::TestRequest::get()
        .uri("/api/contests/search")
        .to_request();

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
