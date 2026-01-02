use actix_web::{get, HttpResponse, Responder};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    timestamp: u64,
    version: &'static str,
}

#[get("/health")]
pub async fn health_check() -> impl Responder {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let response = HealthResponse {
        status: "ok".to_string(),
        timestamp,
        version: env!("CARGO_PKG_VERSION"),
    };

    HttpResponse::Ok().json(response)
}

#[get("/health/detailed")]
pub async fn detailed_health_check() -> impl Responder {
    // TODO: Add database connectivity check
    // TODO: Add Redis connectivity check
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    #[derive(Serialize)]
    struct DetailedHealthResponse {
        status: String,
        timestamp: u64,
        version: &'static str,
        services: ServicesHealth,
    }

    #[derive(Serialize)]
    struct ServicesHealth {
        database: String,
        redis: String,
        scheduler: String,
    }

    let response = DetailedHealthResponse {
        status: "ok".to_string(),
        timestamp,
        version: env!("CARGO_PKG_VERSION"),
        services: ServicesHealth {
            database: "unknown".to_string(), // TODO: implement actual check
            redis: "unknown".to_string(),    // TODO: implement actual check
            scheduler: "unknown".to_string(), // TODO: implement actual check
        },
    };

    HttpResponse::Ok().json(response)
}

#[get("/health/scheduler")]
pub async fn scheduler_health_check() -> impl Responder {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    #[derive(Serialize)]
    struct SchedulerHealthResponse {
        status: String,
        timestamp: u64,
        message: String,
        note: String,
    }

    let response = SchedulerHealthResponse {
        status: "ok".to_string(),
        timestamp,
        message: "Glicko2 ratings scheduler is running in the backend".to_string(),
        note: "Check /api/ratings/scheduler/status for detailed scheduler information".to_string(),
    };

    HttpResponse::Ok().json(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App, http::StatusCode};
    use serde_json::Value;

    #[actix_web::test]
    async fn test_health_response_structure() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let response = HealthResponse {
            status: "ok".to_string(),
            timestamp,
            version: "1.0.0",
        };
        assert_eq!(response.status, "ok");
        assert_eq!(response.timestamp, timestamp);
        assert_eq!(response.version, "1.0.0");
    }

    #[actix_web::test]
    async fn test_timestamp_is_recent() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(timestamp <= current_time);
        assert!(timestamp >= current_time - 60); // Within last minute
    }

    #[actix_web::test]
    async fn test_health_check() {
        let app = test::init_service(App::new().service(health_check)).await;
        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = test::read_body(resp).await;
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert!(json["timestamp"].as_u64().is_some());
        assert!(json["version"].as_str().is_some());
    }

    #[actix_web::test]
    async fn test_detailed_health_check() {
        let app = test::init_service(App::new().service(detailed_health_check)).await;
        let req = test::TestRequest::get().uri("/health/detailed").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = test::read_body(resp).await;
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert!(json["timestamp"].as_u64().is_some());
        assert!(json["version"].as_str().is_some());
        assert_eq!(json["services"]["database"], "unknown");
        assert_eq!(json["services"]["redis"], "unknown");
    }

    #[actix_web::test]
    async fn test_health_check_json_structure() {
        let app = test::init_service(App::new().service(health_check)).await;
        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;
        let body = test::read_body(resp).await;
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("status").is_some());
        assert!(json.get("timestamp").is_some());
        assert!(json.get("version").is_some());
        assert!(json["status"].is_string());
        assert!(json["timestamp"].is_number());
        assert!(json["version"].is_string());
    }

    #[actix_web::test]
    async fn test_detailed_health_check_json_structure() {
        let app = test::init_service(App::new().service(detailed_health_check)).await;
        let req = test::TestRequest::get().uri("/health/detailed").to_request();
        let resp = test::call_service(&app, req).await;
        let body = test::read_body(resp).await;
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("status").is_some());
        assert!(json.get("timestamp").is_some());
        assert!(json.get("version").is_some());
        assert!(json.get("services").is_some());
        let services = &json["services"];
        assert!(services.get("database").is_some());
        assert!(services.get("redis").is_some());
        assert!(json["status"].is_string());
        assert!(json["timestamp"].is_number());
        assert!(json["version"].is_string());
        assert!(services["database"].is_string());
        assert!(services["redis"].is_string());
    }
} 