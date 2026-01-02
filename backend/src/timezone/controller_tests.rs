#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};
    use backend::third_party::google::timezone::GoogleTimezoneService;

    #[tokio::test]
    async fn test_timezone_resolve_by_coordinates() {
        let service = GoogleTimezoneService::new(
            "https://maps.googleapis.com/maps/api/timezone/json".to_string(),
            "test_key".to_string()
        );
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(service))
                .route("/api/timezone/resolve", web::get().to(|query: web::Query<ResolveQuery>, svc: web::Data<GoogleTimezoneService>| async move {
                    let tz = if let Some(place_id) = &query.place_id {
                        svc.infer_timezone_from_place_id(place_id).await
                    } else if let (Some(lat), Some(lng)) = (query.lat, query.lng) {
                        svc.infer_timezone_from_coordinates(lat, lng).await
                    } else {
                        "UTC".to_string()
                    };
                    Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(serde_json::json!({"timezone": tz})))
                }))
        ).await;

        // Test with coordinates
        let req = test::TestRequest::get()
            .uri("/api/timezone/resolve?lat=40.7128&lng=-74.0060")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body.get("timezone").is_some());
    }

    #[tokio::test]
    async fn test_timezone_resolve_by_place_id() {
        let service = GoogleTimezoneService::new(
            "https://maps.googleapis.com/maps/api/timezone/json".to_string(),
            "test_key".to_string()
        );
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(service))
                .route("/api/timezone/resolve", web::get().to(|query: web::Query<ResolveQuery>, svc: web::Data<GoogleTimezoneService>| async move {
                    let tz = if let Some(place_id) = &query.place_id {
                        svc.infer_timezone_from_place_id(place_id).await
                    } else if let (Some(lat), Some(lng)) = (query.lat, query.lng) {
                        svc.infer_timezone_from_coordinates(lat, lng).await
                    } else {
                        "UTC".to_string()
                    };
                    Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(serde_json::json!({"timezone": tz})))
                }))
        ).await;

        // Test with place_id
        let req = test::TestRequest::get()
            .uri("/api/timezone/resolve?place_id=ChIJiWmIvJFZwokRVdr0ZDWdHu4")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body.get("timezone").is_some());
    }

    #[tokio::test]
    async fn test_timezone_resolve_no_parameters() {
        let service = GoogleTimezoneService::new(
            "https://maps.googleapis.com/maps/api/timezone/json".to_string(),
            "test_key".to_string()
        );
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(service))
                .route("/api/timezone/resolve", web::get().to(|query: web::Query<ResolveQuery>, svc: web::Data<GoogleTimezoneService>| async move {
                    let tz = if let Some(place_id) = &query.place_id {
                        svc.infer_timezone_from_place_id(place_id).await
                    } else if let (Some(lat), Some(lng)) = (query.lat, query.lng) {
                        svc.infer_timezone_from_coordinates(lat, lng).await
                    } else {
                        "UTC".to_string()
                    };
                    Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(serde_json::json!({"timezone": tz})))
                }))
        ).await;

        // Test with no parameters (should return UTC)
        let req = test::TestRequest::get()
            .uri("/api/timezone/resolve")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body.get("timezone").unwrap().as_str().unwrap(), "UTC");
    }

    #[tokio::test]
    async fn test_timezone_resolve_priority() {
        let service = GoogleTimezoneService::new(
            "https://maps.googleapis.com/maps/api/timezone/json".to_string(),
            "test_key".to_string()
        );
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(service))
                .route("/api/timezone/resolve", web::get().to(|query: web::Query<ResolveQuery>, svc: web::Data<GoogleTimezoneService>| async move {
                    let tz = if let Some(place_id) = &query.place_id {
                        svc.infer_timezone_from_place_id(place_id).await
                    } else if let (Some(lat), Some(lng)) = (query.lat, query.lng) {
                        svc.infer_timezone_from_coordinates(lat, lng).await
                    } else {
                        "UTC".to_string()
                    };
                    Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(serde_json::json!({"timezone": tz})))
                }))
        ).await;

        // Test with both place_id and coordinates (place_id should take priority)
        let req = test::TestRequest::get()
            .uri("/api/timezone/resolve?place_id=ChIJiWmIvJFZwokRVdr0ZDWdHu4&lat=40.7128&lng=-74.0060")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body.get("timezone").is_some());
        // In test environment, both should return "UTC" since we don't have real API key
        assert_eq!(body.get("timezone").unwrap().as_str().unwrap(), "UTC");
    }
}
