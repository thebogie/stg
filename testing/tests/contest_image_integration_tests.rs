//! Integration tests: contest thumbnail upload, serve (WebP), auth, and cleanup.
//!
//! Requires SurrealDB + Redis: `./scripts/start-back.sh` then `cargo test -p testing contest_image`

use actix_web::{test, web, App};
use anyhow::Result;
use backend::contest::image::{
    image_file_exists, image_path_for_key, sample_png_bytes, CONTEST_IMAGE_UPLOAD_MAX_BYTES,
};
use chrono::{DateTime, FixedOffset, Utc};
use serde_json::json;
use shared::dto::contest::ContestDto;
use shared::dto::game::GameDto;
use shared::dto::venue::VenueDto;
use testing::app_setup;
use testing::create_authenticated_user;
use testing::TestEnvironment;

fn key_only(id: &str) -> &str {
    id.split_once('/').map(|(_, k)| k).unwrap_or(id)
}

struct ContestImageDirGuard {
    previous: Option<String>,
    _temp: tempfile::TempDir,
}

impl ContestImageDirGuard {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let previous = std::env::var("CONTEST_IMAGE_DIR").ok();
        std::env::set_var(
            "CONTEST_IMAGE_DIR",
            temp.path().to_string_lossy().as_ref(),
        );
        backend::contest::image::ensure_image_dir().expect("ensure dir");
        Self {
            previous,
            _temp: temp,
        }
    }
}

impl Drop for ContestImageDirGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(v) => std::env::set_var("CONTEST_IMAGE_DIR", v),
            None => std::env::remove_var("CONTEST_IMAGE_DIR"),
        }
    }
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

macro_rules! contests_api_scope {
    ($app_data:expr) => {
        web::scope("/api/contests")
            .wrap(backend::auth::AuthMiddleware {
                redis: $app_data.redis_arc.clone(),
            })
            .app_data(actix_web::web::JsonConfig::default().limit(128 * 1024))
            .app_data(
                web::PayloadConfig::default().limit(CONTEST_IMAGE_UPLOAD_MAX_BYTES),
            )
            .app_data($app_data.player_repo.clone())
            .service(backend::contest::controller::create_contest_handler)
            .service(backend::contest::image_handlers::get_contest_image_handler)
            .service(backend::contest::image_handlers::upload_contest_image_handler)
            .service(backend::contest::image_handlers::delete_contest_image_handler)
            .service(backend::contest::controller::get_contest_handler)
            .service(
                web::scope("")
                    .wrap(backend::auth::AdminAuthMiddleware {
                        redis: $app_data.redis_arc.clone(),
                        player_repo: $app_data.player_repo_arc.clone(),
                    })
                    .app_data($app_data.contest_repo.clone())
                    .service(backend::contest::controller::delete_contest_handler),
            )
    };
}

macro_rules! create_test_contest {
    ($app:expr, $session_id:expr, $place_id:expr, $contest_name:expr) => {{
        let venue_req = test::TestRequest::post()
            .uri("/api/venues")
            .insert_header(("Authorization", format!("Bearer {}", $session_id)))
            .set_json(&json!({
                "displayName": "Image Test Venue",
                "formattedAddress": "1 Image St",
                "place_id": $place_id,
                "lat": 41.0,
                "lng": -71.0,
                "timezone": "UTC",
                "source": "database"
            }))
            .to_request();
        let venue: VenueDto =
            test::read_body_json(test::call_service($app, venue_req).await).await;

        let game_req = test::TestRequest::post()
            .uri("/api/games")
            .insert_header(("Authorization", format!("Bearer {}", $session_id)))
            .set_json(&json!({ "name": "Image Test Game", "year_published": 2024, "source": "database" }))
            .to_request();
        let game: GameDto =
            test::read_body_json(test::call_service($app, game_req).await).await;

        let start: DateTime<FixedOffset> = Utc::now().into();
        let stop: DateTime<FixedOffset> = (Utc::now() + chrono::Duration::hours(1)).into();
        let contest_req = test::TestRequest::post()
            .uri("/api/contests")
            .insert_header(("Authorization", format!("Bearer {}", $session_id)))
            .set_json(&json!({
                "name": $contest_name,
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
        let resp = test::call_service($app, contest_req).await;
        assert!(resp.status().is_success(), "create contest");
        let created: ContestDto = test::read_body_json(resp).await;
        created
    }};
}

fn is_webp(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP"
}

#[tokio::test]
#[serial_test::serial]
async fn contest_image_upload_get_delete_roundtrip() -> Result<()> {
    let _img_dir = ContestImageDirGuard::new();
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let email = format!("contest_img_{}@example.com", ts);
    let place_id = format!("img_place_{}", ts);

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
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
                    .service(backend::venue::controller::create_venue_handler),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .service(backend::game::controller::create_game_handler),
            )
            .service(contests_api_scope!(&app_data)),
    )
    .await;

    let session_id = create_authenticated_user!(app, email.as_str(), "contestimg");
    let created = create_test_contest!(
        &app,
        &session_id,
        &place_id,
        &format!("Image Roundtrip {}", ts)
    );
    let contest_key = key_only(&created.id);

    let png = sample_png_bytes(640, 480);
    let put_req = test::TestRequest::put()
        .uri(&format!("/api/contests/{}/image", contest_key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .insert_header(("Content-Type", "image/png"))
        .set_payload(png)
        .to_request();
    let put_resp = test::call_service(&app, put_req).await;
    assert!(put_resp.status().is_success(), "upload image");
    let updated: ContestDto = test::read_body_json(put_resp).await;
    assert!(updated.has_image);
    assert!(updated.image_url.as_ref().is_some_and(|u| u.contains("/image")));

    assert!(image_file_exists(contest_key));
    assert!(image_path_for_key(contest_key).is_file());

    let get_img = test::TestRequest::get()
        .uri(&format!("/api/contests/{}/image", contest_key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    let get_img_resp = test::call_service(&app, get_img).await;
    assert!(get_img_resp.status().is_success());
    assert_eq!(
        get_img_resp
            .headers()
            .get(actix_web::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("image/webp")
    );
    let body = test::read_body(get_img_resp).await;
    assert!(is_webp(&body));

    let get_req = test::TestRequest::get()
        .uri(&format!("/api/contests/{}", contest_key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert!(get_resp.status().is_success());
    let fetched: ContestDto = test::read_body_json(get_resp).await;
    assert!(fetched.has_image);
    assert!(fetched.image_url.is_some());

    let del_img = test::TestRequest::delete()
        .uri(&format!("/api/contests/{}/image", contest_key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    let del_resp = test::call_service(&app, del_img).await;
    assert_eq!(del_resp.status(), actix_web::http::StatusCode::NO_CONTENT);
    assert!(!image_file_exists(contest_key));

    let get_after = test::TestRequest::get()
        .uri(&format!("/api/contests/{}/image", contest_key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    assert_eq!(
        test::call_service(&app, get_after).await.status(),
        actix_web::http::StatusCode::NOT_FOUND
    );

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn contest_image_upload_forbidden_for_non_creator() -> Result<()> {
    let _img_dir = ContestImageDirGuard::new();
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let creator_email = format!("contest_img_cr_{}@example.com", ts);
    let other_email = format!("contest_img_other_{}@example.com", ts);
    let place_id = format!("img_place2_{}", ts);

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
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
                    .service(backend::venue::controller::create_venue_handler),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .service(backend::game::controller::create_game_handler),
            )
            .service(contests_api_scope!(&app_data)),
    )
    .await;

    let creator_session = create_authenticated_user!(app, creator_email.as_str(), "imgcreator");
    let other_session = create_authenticated_user!(app, other_email.as_str(), "imgother");

    let created = create_test_contest!(
        &app,
        &creator_session,
        &place_id,
        &format!("Image Forbidden {}", ts)
    );
    let contest_key = key_only(&created.id);

    let put_req = test::TestRequest::put()
        .uri(&format!("/api/contests/{}/image", contest_key))
        .insert_header(("Authorization", format!("Bearer {}", other_session)))
        .insert_header(("Content-Type", "image/png"))
        .set_payload(sample_png_bytes(64, 64))
        .to_request();
    let put_resp = test::call_service(&app, put_req).await;
    assert_eq!(put_resp.status(), actix_web::http::StatusCode::FORBIDDEN);
    assert!(!image_file_exists(contest_key));

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn contest_image_upload_rejects_invalid_bytes() -> Result<()> {
    let _img_dir = ContestImageDirGuard::new();
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let email = format!("contest_img_bad_{}@example.com", ts);
    let place_id = format!("img_place3_{}", ts);

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
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
                    .service(backend::venue::controller::create_venue_handler),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .service(backend::game::controller::create_game_handler),
            )
            .service(contests_api_scope!(&app_data)),
    )
    .await;

    let session_id = create_authenticated_user!(app, email.as_str(), "imgbad");
    let created = create_test_contest!(
        &app,
        &session_id,
        &place_id,
        &format!("Image Bad Upload {}", ts)
    );
    let contest_key = key_only(&created.id);

    let put_req = test::TestRequest::put()
        .uri(&format!("/api/contests/{}/image", contest_key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .insert_header(("Content-Type", "image/png"))
        .set_payload(b"not-an-image".to_vec())
        .to_request();
    let put_resp = test::call_service(&app, put_req).await;
    assert_eq!(put_resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    assert!(!image_file_exists(contest_key));

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn contest_image_file_removed_when_contest_deleted() -> Result<()> {
    let _img_dir = ContestImageDirGuard::new();
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let email = format!("contest_img_del_{}@example.com", ts);
    let place_id = format!("img_place4_{}", ts);
    let _admin = AdminEmailsGuard::set_to(&email);

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
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
                    .service(backend::venue::controller::create_venue_handler),
            )
            .service(
                web::scope("/api/games")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .service(backend::game::controller::create_game_handler),
            )
            .service(contests_api_scope!(&app_data)),
    )
    .await;

    let session_id = create_authenticated_user!(app, email.as_str(), "imgdel");
    let created = create_test_contest!(
        &app,
        &session_id,
        &place_id,
        &format!("Image Delete Contest {}", ts)
    );
    let contest_key = key_only(&created.id);

    let put_req = test::TestRequest::put()
        .uri(&format!("/api/contests/{}/image", contest_key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .insert_header(("Content-Type", "image/png"))
        .set_payload(sample_png_bytes(100, 100))
        .to_request();
    assert!(test::call_service(&app, put_req).await.status().is_success());
    assert!(image_file_exists(contest_key));

    let del_req = test::TestRequest::delete()
        .uri(&format!("/api/contests/{}", contest_key))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    assert_eq!(
        test::call_service(&app, del_req).await.status(),
        actix_web::http::StatusCode::NO_CONTENT
    );
    assert!(!image_file_exists(contest_key));

    Ok(())
}
