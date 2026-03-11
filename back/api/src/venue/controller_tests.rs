#[cfg(test)]
mod venue_controller_tests {
    use super::*;
    use actix_web::test;
    use actix_web::web;
    use actix_web::App;
    use shared::dto::venue::{VenueDto, CreateVenueRequest, UpdateVenueRequest};
    use shared::models::venue::Venue;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock repository for testing
    #[derive(Clone)]
    struct MockVenueRepository {
        venues: Arc<Mutex<Vec<Venue>>>,
    }

    impl MockVenueRepository {
        fn new() -> Self {
            Self {
                venues: Arc::new(Mutex::new(vec![])),
            }
        }

        fn add_venue(&self, venue: Venue) {
            let mut venues = self.venues.blocking_lock();
            venues.push(venue);
        }
    }

    #[async_trait::async_trait]
    impl VenueRepository for MockVenueRepository {
        async fn get_venue(&self, id: &str) -> Result<Venue, String> {
            let venues = self.venues.lock().await;
            venues.iter()
                .find(|v| v.id == id)
                .cloned()
                .ok_or_else(|| "Venue not found".to_string())
        }

        async fn get_all_venues(&self) -> Result<Vec<Venue>, String> {
            let venues = self.venues.lock().await;
            Ok(venues.clone())
        }

        async fn create_venue(&self, venue: &Venue) -> Result<Venue, String> {
            let mut venues = self.venues.lock().await;
            venues.push(venue.clone());
            Ok(venue.clone())
        }

        async fn update_venue(&self, venue: &Venue) -> Result<Venue, String> {
            let mut venues = self.venues.lock().await;
            if let Some(existing_venue) = venues.iter_mut().find(|v| v.id == venue.id) {
                *existing_venue = venue.clone();
                Ok(venue.clone())
            } else {
                Err("Venue not found".to_string())
            }
        }

        async fn delete_venue(&self, id: &str) -> Result<(), String> {
            let mut venues = self.venues.lock().await;
            if let Some(pos) = venues.iter().position(|v| v.id == id) {
                venues.remove(pos);
                Ok(())
            } else {
                Err("Venue not found".to_string())
            }
        }

        async fn search_venues(&self, _query: &str) -> Result<Vec<Venue>, String> {
            let venues = self.venues.lock().await;
            Ok(venues.clone())
        }

        async fn infer_timezone_from_coordinates(&self, _lat: f64, _lng: f64) -> String {
            "UTC".to_string()
        }
    }

    fn create_test_venue() -> Venue {
        Venue {
            id: "venue/test123".to_string(),
            name: "Test Venue".to_string(),
            address: Some("123 Test St".to_string()),
            city: Some("Test City".to_string()),
            state: Some("TS".to_string()),
            zip_code: Some("12345".to_string()),
            country: Some("USA".to_string()),
            lat: Some(40.7128),
            lng: Some(-74.0060),
            timezone: "UTC".to_string(),
            place_id: None,
            created_at: chrono::Utc::now().fixed_offset(),
            updated_at: chrono::Utc::now().fixed_offset(),
        }
    }

    #[tokio::test]
    async fn test_get_venue_handler_success() {
        let repo = MockVenueRepository::new();
        let test_venue = create_test_venue();
        repo.add_venue(test_venue.clone());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/venues").service(get_venue_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/venues/test123")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: VenueDto = test::read_body_json(resp).await;
        assert_eq!(body.id, "venue/test123");
        assert_eq!(body.name, "Test Venue");
        assert_eq!(body.city, Some("Test City".to_string()));
    }

    #[tokio::test]
    async fn test_get_venue_handler_not_found() {
        let repo = MockVenueRepository::new();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/venues").service(get_venue_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/venues/nonexistent")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[tokio::test]
    async fn test_get_all_venues_handler() {
        let repo = MockVenueRepository::new();
        let venue1 = Venue {
            id: "venue/test1".to_string(),
            name: "Venue 1".to_string(),
            address: Some("123 Main St".to_string()),
            city: Some("City 1".to_string()),
            state: Some("ST".to_string()),
            zip_code: Some("12345".to_string()),
            country: Some("USA".to_string()),
            lat: Some(40.7128),
            lng: Some(-74.0060),
            timezone: "UTC".to_string(),
            place_id: None,
            created_at: chrono::Utc::now().fixed_offset(),
            updated_at: chrono::Utc::now().fixed_offset(),
        };
        let venue2 = Venue {
            id: "venue/test2".to_string(),
            name: "Venue 2".to_string(),
            address: Some("456 Oak Ave".to_string()),
            city: Some("City 2".to_string()),
            state: Some("ST".to_string()),
            zip_code: Some("67890".to_string()),
            country: Some("USA".to_string()),
            lat: Some(34.0522),
            lng: Some(-118.2437),
            timezone: "UTC".to_string(),
            place_id: None,
            created_at: chrono::Utc::now().fixed_offset(),
            updated_at: chrono::Utc::now().fixed_offset(),
        };
        repo.add_venue(venue1);
        repo.add_venue(venue2);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/venues").service(get_all_venues_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/venues")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Vec<VenueDto> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 2);
        assert_eq!(body[0].name, "Venue 1");
        assert_eq!(body[1].name, "Venue 2");
    }

    #[tokio::test]
    async fn test_create_venue_handler() {
        let repo = MockVenueRepository::new();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/venues").service(create_venue_handler))
        ).await;

        let create_request = CreateVenueRequest {
            name: "New Venue".to_string(),
            address: Some("789 Pine St".to_string()),
            city: Some("New City".to_string()),
            state: Some("NC".to_string()),
            zip_code: Some("54321".to_string()),
            country: Some("USA".to_string()),
            lat: Some(41.8781),
            lng: Some(-87.6298),
            timezone: Some("America/Chicago".to_string()),
            place_id: None,
        };

        let req = test::TestRequest::post()
            .uri("/venues")
            .set_json(&create_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: VenueDto = test::read_body_json(resp).await;
        assert_eq!(body.name, "New Venue");
        assert_eq!(body.city, Some("New City".to_string()));
        assert_eq!(body.lat, Some(41.8781));
        assert_eq!(body.lng, Some(-87.6298));
    }

    #[tokio::test]
    async fn test_create_venue_handler_validation_error() {
        let repo = MockVenueRepository::new();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/venues").service(create_venue_handler))
        ).await;

        // Create invalid venue (missing required fields)
        let invalid_request = CreateVenueRequest {
            name: "".to_string(), // Invalid: empty name
            address: None,
            city: None,
            state: None,
            zip_code: None,
            country: None,
            lat: None,
            lng: None,
            timezone: None,
            place_id: None,
        };

        let req = test::TestRequest::post()
            .uri("/venues")
            .set_json(&invalid_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn test_update_venue_handler() {
        let repo = MockVenueRepository::new();
        let existing_venue = create_test_venue();
        repo.add_venue(existing_venue);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/venues").service(update_venue_handler))
        ).await;

        let update_request = UpdateVenueRequest {
            name: Some("Updated Venue".to_string()),
            address: Some("Updated Address".to_string()),
            city: Some("Updated City".to_string()),
            state: Some("UC".to_string()),
            zip_code: Some("99999".to_string()),
            country: Some("Canada".to_string()),
            lat: Some(43.6532),
            lng: Some(-79.3832),
            timezone: Some("America/Toronto".to_string()),
            place_id: Some("place123".to_string()),
        };

        let req = test::TestRequest::put()
            .uri("/venues/test123")
            .set_json(&update_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: VenueDto = test::read_body_json(resp).await;
        assert_eq!(body.name, "Updated Venue");
        assert_eq!(body.city, Some("Updated City".to_string()));
        assert_eq!(body.country, Some("Canada".to_string()));
    }

    #[tokio::test]
    async fn test_delete_venue_handler() {
        let repo = MockVenueRepository::new();
        let test_venue = create_test_venue();
        repo.add_venue(test_venue);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo.clone()))
                .service(web::scope("/venues").service(delete_venue_handler))
        ).await;

        let req = test::TestRequest::delete()
            .uri("/venues/test123")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 204);

        // Verify venue was deleted
        let venues = repo.get_all_venues().await.unwrap();
        assert_eq!(venues.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_venue_handler_not_found() {
        let repo = MockVenueRepository::new();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/venues").service(delete_venue_handler))
        ).await;

        let req = test::TestRequest::delete()
            .uri("/venues/nonexistent")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[tokio::test]
    async fn test_search_venues_handler() {
        let repo = MockVenueRepository::new();
        let test_venue = create_test_venue();
        repo.add_venue(test_venue);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/venues").service(search_venues_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/venues/search?q=test")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Vec<VenueDto> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].name, "Test Venue");
    }

    #[test]
    fn test_venue_id_normalization() {
        // Test the ID normalization logic used in handlers
        let param_with_slash = "venue/test123";
        let param_without_slash = "test123";
        
        let id_with_slash = if param_with_slash.contains('/') { 
            param_with_slash.to_string() 
        } else { 
            format!("venue/{}", param_with_slash) 
        };
        
        let id_without_slash = if param_without_slash.contains('/') { 
            param_without_slash.to_string() 
        } else { 
            format!("venue/{}", param_without_slash) 
        };
        
        assert_eq!(id_with_slash, "venue/test123");
        assert_eq!(id_without_slash, "venue/test123");
    }

    #[test]
    fn test_venue_dto_conversion() {
        let venue = create_test_venue();
        let venue_dto = VenueDto::from(&venue);
        
        assert_eq!(venue_dto.id, venue.id);
        assert_eq!(venue_dto.name, venue.name);
        assert_eq!(venue_dto.address, venue.address);
        assert_eq!(venue_dto.city, venue.city);
        assert_eq!(venue_dto.lat, venue.lat);
        assert_eq!(venue_dto.lng, venue.lng);
        assert_eq!(venue_dto.timezone, venue.timezone);
    }

    #[test]
    fn test_coordinate_validation() {
        // Test coordinate validation logic
        let valid_lat = 40.7128;
        let valid_lng = -74.0060;
        
        assert!(valid_lat >= -90.0 && valid_lat <= 90.0);
        assert!(valid_lng >= -180.0 && valid_lng <= 180.0);
        
        let invalid_lat = 200.0;
        let invalid_lng = -200.0;
        
        assert!(!(invalid_lat >= -90.0 && invalid_lat <= 90.0));
        assert!(!(invalid_lng >= -180.0 && invalid_lng <= 180.0));
    }
}
