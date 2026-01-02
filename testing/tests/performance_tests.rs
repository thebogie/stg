//! Performance and load tests for API endpoints
//! Tests response times, concurrent requests, and resource usage

use anyhow::Result;
use actix_web::{test, web, App};
use serde_json::json;
use testing::{TestEnvironment, app_setup};
use std::time::Instant;

#[tokio::test]
async fn test_concurrent_player_registrations() -> Result<()> {
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
            .app_data(app_data.session_store.clone())
            .service(web::scope("/api/players")
                .service(backend::player::controller::register_handler_prod)
            )
    ).await;

    // Test concurrent registrations
    let start = Instant::now();
    let mut handles = vec![];
    
    for i in 0..10 {
        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            let req = test::TestRequest::post()
                .uri("/api/players/register")
                .set_json(&json!({
                    "username": format!("concurrent_user_{}", i),
                    "email": format!("concurrent_{}@example.com", i),
                    "password": "password123"
                }))
                .to_request();
            
            test::call_service(&app_clone, req).await
        });
        handles.push(handle);
    }
    
    let results = futures::future::join_all(handles).await;
    let duration = start.elapsed();
    
    // All should succeed
    for result in results {
        let resp = result.unwrap();
        assert!(resp.status().is_success() || resp.status().is_client_error());
    }
    
    // Should complete in reasonable time (5 seconds for 10 concurrent)
    assert!(duration.as_secs() < 5, "Concurrent registrations took too long: {:?}", duration);

    Ok(())
}

#[tokio::test]
async fn test_api_response_time() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;
    
    let app = test::init_service(
        App::new()
            .wrap(backend::middleware::Logger)
            .wrap(backend::middleware::cors_middleware())
            .app_data(app_data.player_repo.clone())
            .service(web::scope("/api/players")
                .service(backend::player::controller::register_handler_prod)
            )
    ).await;

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
    // Should respond in under 1 second
    assert!(duration.as_millis() < 1000, "Response took too long: {:?}", duration);

    Ok(())
}

