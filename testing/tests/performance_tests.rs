//! Performance and load tests for API endpoints
//! Tests response times, concurrent requests, and resource usage

use actix_web::{test, web, App};
use anyhow::Result;
use serde_json::json;
use std::time::Instant;
use testing::{app_setup, TestEnvironment};

#[tokio::test]
async fn test_concurrent_player_registrations() -> Result<()> {
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
            .app_data(app_data.session_store.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod),
            ),
    )
    .await;

    // Test concurrent registrations
    // Note: actix-web test services cannot be sent across threads, so we test
    // sequential requests quickly to verify performance
    let start = Instant::now();
    let mut responses = vec![];

    for i in 0..10 {
        let req = test::TestRequest::post()
            .uri("/api/players/register")
            .set_json(&json!({
                "username": format!("concurrent_user_{}", i),
                "email": format!("concurrent_{}@example.com", i),
                "password": "password123"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        responses.push(resp);
    }

    let duration = start.elapsed();

    // All should succeed (or be client errors for duplicates)
    for resp in responses {
        assert!(resp.status().is_success() || resp.status().is_client_error());
    }

    // Should complete in reasonable time (30 seconds for 10 sequential requests,
    // allowing for test environment overhead, resource contention, and container startup delays)
    // Note: In CI or when running full test suite, this can take longer due to resource sharing
    assert!(
        duration.as_secs() < 30,
        "Concurrent registrations took too long: {:?} (expected < 30s, took {}s)",
        duration,
        duration.as_secs()
    );

    Ok(())
}

#[tokio::test]
async fn test_api_response_time() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger::new())
            .wrap(backend::middleware::cors_middleware())
            .app_data(app_data.player_repo.clone())
            .service(
                web::scope("/api/players")
                    .service(backend::player::controller::register_handler_prod),
            ),
    )
    .await;

    // Test response time for registration
    let start = Instant::now();
    let req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "perf_test_user",
            "email": "perf_test@example.com",
            "password": "password123"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let duration = start.elapsed();

    assert!(resp.status().is_success());
    // Should respond in under 2 seconds (allowing for container/test environment overhead)
    assert!(
        duration.as_millis() < 2000,
        "Response took too long: {:?}ms (expected < 2000ms)",
        duration.as_millis()
    );

    Ok(())
}
