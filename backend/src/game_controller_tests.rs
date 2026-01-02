#[cfg(test)]
mod game_controller_tests {
    use crate::game::repository::GameRepository;
    // GameUseCase and GameUseCaseImpl are used implicitly by the handler implementations
    use crate::game::controller::{get_game_handler_impl, get_all_games_handler_impl, create_game_handler_impl, update_game_handler_impl, delete_game_handler_impl};
    use actix_web::test;
    use actix_web::web;
    use actix_web::App;
    use shared::dto::game::GameDto;
    use shared::models::game::{Game, GameSource};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock repository for testing
    #[derive(Clone)]
    struct MockGameRepository {
        games: Arc<Mutex<Vec<Game>>>,
    }

    impl MockGameRepository {
        fn new() -> Self {
            Self {
                games: Arc::new(Mutex::new(vec![])),
            }
        }

        async fn add_game(&self, game: Game) {
            let mut games = self.games.lock().await;
            games.push(game);
        }
    }

    #[async_trait::async_trait]
    impl GameRepository for MockGameRepository {
        async fn find_by_id(&self, id: &str) -> Option<Game> {
            let games = self.games.lock().await;
            games.iter().find(|g| g.id == id).cloned()
        }

        async fn find_all(&self) -> Vec<Game> {
            let games = self.games.lock().await;
            games.clone()
        }

        async fn search(&self, _query: &str) -> Vec<Game> {
            let games = self.games.lock().await;
            games.clone()
        }

        async fn search_dto(&self, _query: &str) -> Vec<GameDto> {
            let games = self.games.lock().await;
            games.iter().map(|g| GameDto::from(g)).collect()
        }

        async fn search_db_only(&self, _query: &str) -> Vec<Game> {
            let games = self.games.lock().await;
            games.clone()
        }

        async fn search_db_only_dto(&self, _query: &str) -> Vec<GameDto> {
            let games = self.games.lock().await;
            games.iter().map(|g| GameDto::from(g)).collect()
        }

        async fn get_game_recommendations(&self, _player_id: &str, _limit: i32) -> Result<Vec<serde_json::Value>, String> {
            Ok(vec![])
        }

        async fn get_similar_games(&self, _game_id: &str, _limit: i32) -> Result<Vec<serde_json::Value>, String> {
            Ok(vec![])
        }

        async fn get_popular_games(&self, _limit: i32) -> Result<Vec<serde_json::Value>, String> {
            Ok(vec![])
        }

        async fn create(&self, game: Game) -> Result<Game, String> {
            let mut games = self.games.lock().await;
            games.push(game.clone());
            Ok(game)
        }

        async fn update(&self, game: Game) -> Result<Game, String> {
            let mut games = self.games.lock().await;
            if let Some(existing_game) = games.iter_mut().find(|g| g.id == game.id) {
                *existing_game = game.clone();
                Ok(game)
            } else {
                Err("Game not found".to_string())
            }
        }

        async fn delete(&self, id: &str) -> Result<(), String> {
            let mut games = self.games.lock().await;
            if let Some(pos) = games.iter().position(|g| g.id == id) {
                games.remove(pos);
                Ok(())
            } else {
                Err("Game not found".to_string())
            }
        }
    }

    #[tokio::test]
    async fn test_get_game_handler_success() {
        let repo = MockGameRepository::new();
        let test_game = Game {
            id: "game/test123".to_string(),
            rev: "1".to_string(),
            name: "Test Game".to_string(),
            description: Some("A test game".to_string()),
            year_published: Some(2020),
            bgg_id: Some(12345),
            source: GameSource::Database,
        };
        repo.add_game(test_game.clone()).await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/games").route("/{id}", web::get().to(get_game_handler_impl::<MockGameRepository>)))
        ).await;

        let req = test::TestRequest::get()
            .uri("/games/test123")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: GameDto = test::read_body_json(resp).await;
        assert_eq!(body.id, "game/test123");
        assert_eq!(body.name, "Test Game");
    }

    #[tokio::test]
    async fn test_get_game_handler_not_found() {
        let repo = MockGameRepository::new();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/games").route("/{id}", web::get().to(get_game_handler_impl::<MockGameRepository>)))
        ).await;

        let req = test::TestRequest::get()
            .uri("/games/nonexistent")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[tokio::test]
    async fn test_get_all_games_handler() {
        let repo = MockGameRepository::new();
        let game1 = Game {
            id: "game/test1".to_string(),
            rev: "1".to_string(),
            name: "Game 1".to_string(),
            description: None,
            year_published: None,
            bgg_id: None,
            source: GameSource::Database,
        };
        let game2 = Game {
            id: "game/test2".to_string(),
            rev: "1".to_string(),
            name: "Game 2".to_string(),
            description: None,
            year_published: Some(2021),
            bgg_id: Some(67890),
            source: GameSource::BGG,
        };
        repo.add_game(game1).await;
        repo.add_game(game2).await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/games").route("", web::get().to(get_all_games_handler_impl::<MockGameRepository>)))
        ).await;

        let req = test::TestRequest::get()
            .uri("/games")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Vec<GameDto> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 2);
        assert_eq!(body[0].name, "Game 1");
        assert_eq!(body[1].name, "Game 2");
    }

    #[tokio::test]
    async fn test_create_game_handler() {
        let repo = MockGameRepository::new();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/games").route("", web::post().to(create_game_handler_impl::<MockGameRepository>)))
        ).await;

        let create_request = GameDto {
            id: "".to_string(),
            name: "New Game".to_string(),
            description: Some("A new game".to_string()),
            year_published: None,
            bgg_id: None,
            source: GameSource::Database,
        };

        let req = test::TestRequest::post()
            .uri("/games")
            .set_json(&create_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: GameDto = test::read_body_json(resp).await;
        assert_eq!(body.name, "New Game");
        assert_eq!(body.description, Some("A new game".to_string()));
        assert_eq!(body.source, GameSource::Database);
    }

    #[tokio::test]
    async fn test_update_game_handler() {
        let repo = MockGameRepository::new();
        let existing_game = Game {
            id: "game/test123".to_string(),
            rev: "1".to_string(),
            name: "Original Game".to_string(),
            description: Some("Original description".to_string()),
            year_published: Some(2020),
            bgg_id: None,
            source: GameSource::Database,
        };
        repo.add_game(existing_game).await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/games").route("/{id}", web::put().to(update_game_handler_impl::<MockGameRepository>)))
        ).await;

        let update_request = GameDto {
            id: "game/test123".to_string(),
            name: "Updated Game".to_string(),
            description: Some("Updated description".to_string()),
            year_published: Some(2021),
            bgg_id: Some(12345),
            source: GameSource::Database,
        };

        let req = test::TestRequest::put()
            .uri("/games/test123")
            .set_json(&update_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: GameDto = test::read_body_json(resp).await;
        assert_eq!(body.name, "Updated Game");
        assert_eq!(body.description, Some("Updated description".to_string()));
        assert_eq!(body.year_published, Some(2021));
        assert_eq!(body.bgg_id, Some(12345));
    }

    #[tokio::test]
    async fn test_delete_game_handler() {
        let repo = MockGameRepository::new();
        let test_game = Game {
            id: "game/test123".to_string(),
            rev: "1".to_string(),
            name: "Test Game".to_string(),
            description: None,
            year_published: None,
            bgg_id: None,
            source: GameSource::Database,
        };
        repo.add_game(test_game).await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo.clone()))
                .service(web::scope("/games").route("/{id}", web::delete().to(delete_game_handler_impl::<MockGameRepository>)))
        ).await;

        let req = test::TestRequest::delete()
            .uri("/games/test123")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 204);

        // Verify game was deleted
        let games = repo.find_all().await;
        assert_eq!(games.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_game_handler_not_found() {
        let repo = MockGameRepository::new();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .service(web::scope("/games").route("/{id}", web::delete().to(delete_game_handler_impl::<MockGameRepository>)))
        ).await;

        let req = test::TestRequest::delete()
            .uri("/games/nonexistent")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[tokio::test]
    async fn test_id_normalization_logic() {
        // Test the ID normalization logic used in handlers
        let param_with_slash = "game/test123";
        let param_without_slash = "test123";
        
        let id_with_slash = if param_with_slash.contains('/') { 
            param_with_slash.to_string() 
        } else { 
            format!("game/{}", param_with_slash) 
        };
        
        let id_without_slash = if param_without_slash.contains('/') { 
            param_without_slash.to_string() 
        } else { 
            format!("game/{}", param_without_slash) 
        };
        
        assert_eq!(id_with_slash, "game/test123");
        assert_eq!(id_without_slash, "game/test123");
    }
}
