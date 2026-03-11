#[cfg(test)]
mod contest_controller_tests {
    use super::*;
    use actix_web::test;
    use actix_web::web;
    use actix_web::App;
    use shared::dto::contest::ContestDto;
    use shared::models::contest::{Contest, ContestStatus};
    use shared::models::venue::Venue;
    use shared::models::game::Game;
    use shared::models::game::GameSource;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock repository for testing
    #[derive(Clone)]
    struct MockContestRepository {
        contests: Arc<Mutex<Vec<ContestDto>>>,
    }

    impl MockContestRepository {
        fn new() -> Self {
            Self {
                contests: Arc::new(Mutex::new(vec![])),
            }
        }

        fn add_contest(&self, contest: ContestDto) {
            let mut contests = self.contests.blocking_lock();
            contests.push(contest);
        }
    }

    #[async_trait::async_trait]
    impl ContestRepository for MockContestRepository {
        async fn create_contest(&self, mut contest: ContestDto, creator_id: String) -> Result<ContestDto, String> {
            // Set creator information
            contest.creator_id = creator_id;
            contest.created_at = Some(chrono::Utc::now().fixed_offset());
            
            let mut contests = self.contests.lock().await;
            contests.push(contest.clone());
            Ok(contest)
        }

        async fn find_details_by_id(&self, id: &str) -> Option<ContestDto> {
            let contests = self.contests.lock().await;
            contests.iter().find(|c| c.id == id).cloned()
        }

        async fn find_by_player(&self, _player_id: &str) -> Result<Vec<ContestDto>, String> {
            let contests = self.contests.lock().await;
            Ok(contests.clone())
        }

        async fn find_by_id(&self, id: &str) -> Option<Contest> {
            let contests = self.contests.lock().await;
            contests.iter().find(|c| c.id == id).map(|dto| Contest {
                id: dto.id.clone(),
                rev: "1".to_string(),
                name: dto.name.clone(),
                start: dto.start,
                stop: dto.stop,
                creator_id: dto.creator_id.clone(),
                created_at: dto.created_at.unwrap_or_else(|| chrono::Utc::now().fixed_offset()),
            })
        }

        async fn find_all(&self) -> Vec<Contest> {
            let contests = self.contests.lock().await;
            contests.iter().map(|dto| Contest {
                id: dto.id.clone(),
                rev: "1".to_string(),
                name: dto.name.clone(),
                start: dto.start,
                stop: dto.stop,
                creator_id: dto.creator_id.clone(),
                created_at: dto.created_at.unwrap_or_else(|| chrono::Utc::now().fixed_offset()),
            }).collect()
        }

        async fn search(&self, _query: &str) -> Vec<Contest> {
            self.find_all().await
        }

        async fn update(&self, _contest: Contest) -> Result<Contest, String> {
            Err("Not implemented in mock".to_string())
        }

        async fn delete(&self, _id: &str) -> Result<(), String> {
            Err("Not implemented in mock".to_string())
        }

        async fn find_contests_by_player_and_game(&self, _player_id: &str, _game_id: &str) -> Result<Vec<serde_json::Value>, String> {
            Ok(vec![])
        }

        async fn find_by_game(&self, _game_id: &str) -> Result<Vec<ContestDto>, String> {
            let contests = self.contests.lock().await;
            Ok(contests.clone())
        }

        async fn find_by_venue(&self, _venue_id: &str) -> Result<Vec<ContestDto>, String> {
            let contests = self.contests.lock().await;
            Ok(contests.clone())
        }

        async fn update_contest(&self, contest: ContestDto) -> Result<ContestDto, String> {
            let mut contests = self.contests.lock().await;
            if let Some(existing_contest) = contests.iter_mut().find(|c| c.id == contest.id) {
                *existing_contest = contest.clone();
                Ok(contest)
            } else {
                Err("Contest not found".to_string())
            }
        }

        async fn delete_contest(&self, id: &str) -> Result<(), String> {
            let mut contests = self.contests.lock().await;
            if let Some(pos) = contests.iter().position(|c| c.id == id) {
                contests.remove(pos);
                Ok(())
            } else {
                Err("Contest not found".to_string())
            }
        }
    }

    fn create_test_contest_dto() -> ContestDto {
        ContestDto {
            id: "contest/test123".to_string(),
            name: "Test Contest".to_string(),
            description: Some("A test contest".to_string()),
            game: shared::dto::game::GameDto {
                id: "game/test".to_string(),
                name: "Test Game".to_string(),
                description: None,
                min_players: 2,
                max_players: 4,
                playtime_minutes: Some(60),
                bgg_id: None,
                source: "database".to_string(),
                created_at: chrono::Utc::now().fixed_offset(),
                updated_at: chrono::Utc::now().fixed_offset(),
            },
            venue: shared::dto::venue::VenueDto {
                id: "venue/test".to_string(),
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
            },
            start_time: chrono::Utc::now().fixed_offset(),
            end_time: Some(chrono::Utc::now().fixed_offset()),
            status: "scheduled".to_string(),
            max_players: Some(8),
            current_players: 4,
            entry_fee: Some(10.0),
            prize_pool: Some(80.0),
            created_at: chrono::Utc::now().fixed_offset(),
            updated_at: chrono::Utc::now().fixed_offset(),
        }
    }

    #[tokio::test]
    async fn test_create_contest_handler_success() {
        let repo = MockContestRepository::new();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/contests").service(create_contest_handler))
        ).await;

        let contest_dto = create_test_contest_dto();

        let req = test::TestRequest::post()
            .uri("/contests")
            .set_json(&contest_dto)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: ContestDto = test::read_body_json(resp).await;
        assert_eq!(body.id, "contest/test123");
        assert_eq!(body.name, "Test Contest");
    }

    #[tokio::test]
    async fn test_create_contest_handler_validation_error() {
        let repo = MockContestRepository::new();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/contests").service(create_contest_handler))
        ).await;

        // Create invalid contest (missing required fields)
        let invalid_contest = ContestDto {
            id: "".to_string(), // Invalid: empty ID
            name: "".to_string(), // Invalid: empty name
            description: None,
            game: shared::dto::game::GameDto {
                id: "game/test".to_string(),
                name: "Test Game".to_string(),
                description: None,
                min_players: 2,
                max_players: 4,
                playtime_minutes: Some(60),
                bgg_id: None,
                source: "database".to_string(),
                created_at: chrono::Utc::now().fixed_offset(),
                updated_at: chrono::Utc::now().fixed_offset(),
            },
            venue: shared::dto::venue::VenueDto {
                id: "venue/test".to_string(),
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
            },
            start_time: chrono::Utc::now().fixed_offset(),
            end_time: Some(chrono::Utc::now().fixed_offset()),
            status: "scheduled".to_string(),
            max_players: Some(8),
            current_players: 4,
            entry_fee: Some(10.0),
            prize_pool: Some(80.0),
            created_at: chrono::Utc::now().fixed_offset(),
            updated_at: chrono::Utc::now().fixed_offset(),
        };

        let req = test::TestRequest::post()
            .uri("/contests")
            .set_json(&invalid_contest)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn test_get_contest_handler_success() {
        let repo = MockContestRepository::new();
        let test_contest = create_test_contest_dto();
        repo.add_contest(test_contest.clone());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/contests").service(get_contest_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/contests/test123")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: ContestDto = test::read_body_json(resp).await;
        assert_eq!(body.id, "contest/test123");
        assert_eq!(body.name, "Test Contest");
    }

    #[tokio::test]
    async fn test_get_contest_handler_with_full_id() {
        let repo = MockContestRepository::new();
        let test_contest = create_test_contest_dto();
        repo.add_contest(test_contest.clone());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/contests").service(get_contest_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/contests/contest/test123")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: ContestDto = test::read_body_json(resp).await;
        assert_eq!(body.id, "contest/test123");
    }

    #[tokio::test]
    async fn test_get_contest_handler_not_found() {
        let repo = MockContestRepository::new();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/contests").service(get_contest_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/contests/nonexistent")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[tokio::test]
    async fn test_get_contests_by_player_handler() {
        let repo = MockContestRepository::new();
        let test_contest = create_test_contest_dto();
        repo.add_contest(test_contest);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/contests").service(get_contests_by_player_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/contests/player/testplayer")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Vec<ContestDto> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].name, "Test Contest");
    }

    #[tokio::test]
    async fn test_get_contests_by_game_handler() {
        let repo = MockContestRepository::new();
        let test_contest = create_test_contest_dto();
        repo.add_contest(test_contest);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/contests").service(get_contests_by_game_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/contests/game/testgame")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Vec<ContestDto> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].name, "Test Contest");
    }

    #[tokio::test]
    async fn test_get_contests_by_venue_handler() {
        let repo = MockContestRepository::new();
        let test_contest = create_test_contest_dto();
        repo.add_contest(test_contest);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/contests").service(get_contests_by_venue_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/contests/venue/testvenue")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Vec<ContestDto> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].name, "Test Contest");
    }

    #[test]
    fn test_contest_id_normalization() {
        // Test the ID normalization logic used in handlers
        let param_with_slash = "contest/test123";
        let param_without_slash = "test123";
        
        let id_with_slash = if param_with_slash.contains('/') { 
            param_with_slash.to_string() 
        } else { 
            format!("contest/{}", param_with_slash) 
        };
        
        let id_without_slash = if param_without_slash.contains('/') { 
            param_without_slash.to_string() 
        } else { 
            format!("contest/{}", param_without_slash) 
        };
        
        assert_eq!(id_with_slash, "contest/test123");
        assert_eq!(id_without_slash, "contest/test123");
    }

    #[test]
    fn test_contest_dto_validation() {
        let valid_contest = create_test_contest_dto();
        assert!(valid_contest.validate().is_ok());

        let mut invalid_contest = create_test_contest_dto();
        invalid_contest.id = "".to_string();
        assert!(invalid_contest.validate().is_err());
    }

    #[tokio::test]
    async fn test_create_contest_handler_creator_tracking() {
        let mock_repo = MockContestRepository::new();
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(mock_repo.clone()))
                .service(web::scope("/contests").service(create_contest_handler))
        ).await;

        let contest_dto = create_test_contest_dto();
        
        // Test with authentication header
        let req = test::TestRequest::post()
            .uri("/contests")
            .insert_header(("Authorization", "Bearer test_session"))
            .set_json(&contest_dto)
            .to_request();

        let resp = test::call_service(&app, req).await;
        
        // Should succeed and include creator information
        assert_eq!(resp.status(), 200);
        
        let body: ContestDto = test::read_body_json(resp).await;
        assert!(!body.creator_id.is_empty());
        assert!(body.created_at.is_some());
        
        // Verify the creator_id was set
        assert_eq!(body.creator_id, "test_session");
        
        // Verify created_at is recent
        let created_at = body.created_at.unwrap();
        let now = chrono::Utc::now().fixed_offset();
        let diff = now.signed_duration_since(created_at);
        assert!(diff.num_seconds() < 5); // Created within last 5 seconds
    }

    #[tokio::test]
    async fn test_create_contest_handler_no_authentication() {
        let mock_repo = MockContestRepository::new();
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(mock_repo))
                .service(web::scope("/contests").service(create_contest_handler))
        ).await;

        let contest_dto = create_test_contest_dto();
        
        // Test without authentication header
        let req = test::TestRequest::post()
            .uri("/contests")
            .set_json(&contest_dto)
            .to_request();

        let resp = test::call_service(&app, req).await;
        
        // Should fail with 401 Unauthorized
        assert_eq!(resp.status(), 401);
    }
}
