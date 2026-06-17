//! Integration tests for Sell a Game listing API (no-AI flow).

use actix_web::{test, web, App};
use anyhow::Result;
use backend::contest::image::sample_png_bytes;
use backend::sell::image::max_upload_bytes;
use shared::dto::sell_listing::SellListingDto;
use shared::dto::sell_preferences::SellPreferencesDto;
use testing::create_authenticated_user;
use testing::{app_setup, TestEnvironment};

fn test_png() -> Vec<u8> {
    sample_png_bytes(64, 64)
}

#[tokio::test]
#[serial_test::serial]
async fn test_sell_preferences_and_photo_upload() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let sell_repo = web::Data::new(
        backend::sell::repository::SellListingRepositoryImpl::new_with_scope(
            app_data.db.get_ref().clone(),
            env.surrealdb_ns.clone(),
            env.surrealdb_db.clone(),
        ),
    );
    let prefs_repo = web::Data::new(
        backend::sell::preferences_repository::SellPreferencesRepositoryImpl::new_with_scope(
            app_data.db.get_ref().clone(),
            env.surrealdb_ns.clone(),
            env.surrealdb_db.clone(),
        ),
    );

    let _ = backend::sell::image::ensure_image_dir();
    std::env::set_var(
        "SELL_IMAGE_DIR",
        std::env::temp_dir()
            .join("stg_sell_test_images")
            .to_string_lossy()
            .as_ref(),
    );

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::cors_middleware())
            .app_data(app_data.redis_data.clone())
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.session_store.clone())
            .app_data(sell_repo)
            .app_data(prefs_repo)
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod)
                    .service(backend::player::controller::login_handler_prod),
            )
            .service(
                web::scope("/api/sell/preferences")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(app_data.player_repo.clone())
                    .service(backend::sell::controller::get_preferences_handler)
                    .service(backend::sell::controller::put_preferences_handler),
            )
            .service(
                web::scope("/api/sell/listings")
                    .wrap(backend::auth::AuthMiddleware {
                        redis: app_data.redis_arc.clone(),
                    })
                    .app_data(web::PayloadConfig::default().limit(max_upload_bytes()))
                    .app_data(app_data.player_repo.clone())
                    .service(backend::sell::controller::create_listing_handler)
                    .service(backend::sell::controller::upload_photo_handler)
                    .service(backend::sell::controller::approve_checkpoint_handler)
                    .service(backend::sell::controller::get_listing_handler),
            ),
    )
    .await;

    let session_id = create_authenticated_user!(app, "sell_test@example.com", "selltester");

    let prefs_req = test::TestRequest::put()
        .uri("/api/sell/preferences")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .set_json(SellPreferencesDto {
            currency: "USD".to_string(),
            condition: "very_good".to_string(),
            payment_paypal: true,
            payment_other: false,
            item_location: "United States".to_string(),
            ship_to: "United States only".to_string(),
            seller_notes: "Ships promptly".to_string(),
            bgg_username: None,
            updated_at: None,
        })
        .to_request();
    assert!(test::call_service(&app, prefs_req).await.status().is_success());

    let create_req = test::TestRequest::post()
        .uri("/api/sell/listings")
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    let listing: SellListingDto = test::read_body_json(test::call_service(&app, create_req).await).await;
    let listing_key = listing.id.split('/').last().unwrap_or(&listing.id);

    let upload_req = test::TestRequest::put()
        .uri(&format!("/api/sell/listings/{listing_key}/photos"))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .insert_header(("Content-Type", "image/png"))
        .set_payload(test_png())
        .to_request();
    let upload_resp = test::call_service(&app, upload_req).await;
    assert!(
        upload_resp.status().is_success(),
        "upload photo: status={} body={}",
        upload_resp.status(),
        String::from_utf8_lossy(&test::read_body(upload_resp).await)
    );

    let cp_req = test::TestRequest::post()
        .uri(&format!("/api/sell/listings/{listing_key}/checkpoint/photos"))
        .insert_header(("Authorization", format!("Bearer {}", session_id)))
        .to_request();
    assert!(test::call_service(&app, cp_req).await.status().is_success());

    Ok(())
}
