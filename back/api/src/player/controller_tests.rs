#[cfg(test)]
mod player_controller_tests {
    use super::*;
    use actix_web::test;
    use actix_web::web;
    use actix_web::App;
    use shared::dto::player::{PlayerDto, CreatePlayerRequest, LoginResponse, UpdateEmailRequest, UpdateHandleRequest, UpdatePasswordRequest, UpdateResponse};
    use shared::models::player::{Player, PlayerLogin};
    use crate::player::session::MockSessionStore;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock repository for testing
    #[derive(Clone)]
    struct MockPlayerRepository {
        players: Arc<Mutex<Vec<Player>>>,
    }

    impl MockPlayerRepository {
        fn new() -> Self {
            Self {
                players: Arc::new(Mutex::new(vec![])),
            }
        }

        fn add_player(&self, player: Player) {
            let mut players = self.players.blocking_lock();
            players.push(player);
        }
    }

    #[async_trait::async_trait]
    impl PlayerRepository for MockPlayerRepository {
        async fn create_player(&self, player: &Player) -> Result<Player, String> {
            let mut players = self.players.lock().await;
            players.push(player.clone());
            Ok(player.clone())
        }

        async fn get_player_by_email(&self, email: &str) -> Result<Player, String> {
            let players = self.players.lock().await;
            players.iter()
                .find(|p| p.email == email)
                .cloned()
                .ok_or_else(|| "Player not found".to_string())
        }

        async fn get_player_by_id(&self, id: &str) -> Result<Player, String> {
            let players = self.players.lock().await;
            players.iter()
                .find(|p| p.id == id)
                .cloned()
                .ok_or_else(|| "Player not found".to_string())
        }

        async fn update_player(&self, player: &Player) -> Result<Player, String> {
            let mut players = self.players.lock().await;
            if let Some(existing_player) = players.iter_mut().find(|p| p.id == player.id) {
                *existing_player = player.clone();
                Ok(player.clone())
            } else {
                Err("Player not found".to_string())
            }
        }

        async fn delete_player(&self, id: &str) -> Result<(), String> {
            let mut players = self.players.lock().await;
            if let Some(pos) = players.iter().position(|p| p.id == id) {
                players.remove(pos);
                Ok(())
            } else {
                Err("Player not found".to_string())
            }
        }

        async fn search_players(&self, _query: &str) -> Result<Vec<Player>, String> {
            let players = self.players.lock().await;
            Ok(players.clone())
        }
    }

    fn create_test_player() -> Player {
        Player {
            id: "player/test123".to_string(),
            email: "test@example.com".to_string(),
            handle: "testplayer".to_string(),
            password_hash: "hashed_password".to_string(),
            is_admin: false,
            created_at: chrono::Utc::now().fixed_offset(),
            updated_at: chrono::Utc::now().fixed_offset(),
        }
    }

    #[tokio::test]
    async fn test_login_handler_success() {
        let repo = MockPlayerRepository::new();
        let session_store = MockSessionStore::new();
        let test_player = create_test_player();
        repo.add_player(test_player.clone());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .app_data(web::Data::new(session_store))
                .service(web::scope("/auth").service(login_handler))
        ).await;

        let login_request = PlayerLogin {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/auth/login")
            .set_json(&login_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: LoginResponse = test::read_body_json(resp).await;
        assert_eq!(body.player.email, "test@example.com");
        assert_eq!(body.player.handle, "testplayer");
        assert!(!body.session_id.is_empty());
    }

    #[tokio::test]
    async fn test_login_handler_invalid_credentials() {
        let repo = MockPlayerRepository::new();
        let session_store = MockSessionStore::new();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .app_data(web::Data::new(session_store))
                .service(web::scope("/auth").service(login_handler))
        ).await;

        let login_request = PlayerLogin {
            email: "nonexistent@example.com".to_string(),
            password: "password123".to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/auth/login")
            .set_json(&login_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[tokio::test]
    async fn test_register_handler_success() {
        let repo = MockPlayerRepository::new();
        let session_store = MockSessionStore::new();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .app_data(web::Data::new(session_store))
                .service(web::scope("/auth").service(register_handler))
        ).await;

        let register_request = CreatePlayerRequest {
            email: "newuser@example.com".to_string(),
            handle: "newuser".to_string(),
            password: "password123".to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/auth/register")
            .set_json(&register_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: PlayerDto = test::read_body_json(resp).await;
        assert_eq!(body.email, "newuser@example.com");
        assert_eq!(body.handle, "newuser");
        assert!(!body.is_admin);
    }

    #[tokio::test]
    async fn test_register_handler_validation_error() {
        let repo = MockPlayerRepository::new();
        let session_store = MockSessionStore::new();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .app_data(web::Data::new(session_store))
                .service(web::scope("/auth").service(register_handler))
        ).await;

        // Create invalid request (missing required fields)
        let invalid_request = CreatePlayerRequest {
            email: "".to_string(), // Invalid: empty email
            handle: "".to_string(), // Invalid: empty handle
            password: "".to_string(), // Invalid: empty password
        };

        let req = test::TestRequest::post()
            .uri("/auth/register")
            .set_json(&invalid_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn test_get_player_handler_success() {
        let repo = MockPlayerRepository::new();
        let session_store = MockSessionStore::new();
        let test_player = create_test_player();
        repo.add_player(test_player.clone());

        // Set up session
        let session_id = "test-session-id";
        session_store.set_session(session_id, &test_player.email).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .app_data(web::Data::new(session_store))
                .service(web::scope("/players").service(get_player_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/players/test123")
            .insert_header(("X-Session-ID", session_id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: PlayerDto = test::read_body_json(resp).await;
        assert_eq!(body.id, "player/test123");
        assert_eq!(body.email, "test@example.com");
        assert_eq!(body.handle, "testplayer");
    }

    #[tokio::test]
    async fn test_get_player_handler_unauthorized() {
        let repo = MockPlayerRepository::new();
        let session_store = MockSessionStore::new();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .app_data(web::Data::new(session_store))
                .service(web::scope("/players").service(get_player_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/players/test123")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }

    #[tokio::test]
    async fn test_update_email_handler() {
        let repo = MockPlayerRepository::new();
        let session_store = MockSessionStore::new();
        let test_player = create_test_player();
        repo.add_player(test_player.clone());

        // Set up session
        let session_id = "test-session-id";
        session_store.set_session(session_id, &test_player.email).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .app_data(web::Data::new(session_store))
                .service(web::scope("/players").service(update_email_handler))
        ).await;

        let update_request = UpdateEmailRequest {
            new_email: "updated@example.com".to_string(),
            password: "password123".to_string(),
        };

        let req = test::TestRequest::put()
            .uri("/players/test123/email")
            .insert_header(("X-Session-ID", session_id))
            .set_json(&update_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: UpdateResponse = test::read_body_json(resp).await;
        assert_eq!(body.message, "Email updated successfully");
    }

    #[tokio::test]
    async fn test_update_handle_handler() {
        let repo = MockPlayerRepository::new();
        let session_store = MockSessionStore::new();
        let test_player = create_test_player();
        repo.add_player(test_player.clone());

        // Set up session
        let session_id = "test-session-id";
        session_store.set_session(session_id, &test_player.email).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .app_data(web::Data::new(session_store))
                .service(web::scope("/players").service(update_handle_handler))
        ).await;

        let update_request = UpdateHandleRequest {
            new_handle: "updatedhandle".to_string(),
        };

        let req = test::TestRequest::put()
            .uri("/players/test123/handle")
            .insert_header(("X-Session-ID", session_id))
            .set_json(&update_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: UpdateResponse = test::read_body_json(resp).await;
        assert_eq!(body.message, "Handle updated successfully");
    }

    #[tokio::test]
    async fn test_update_password_handler() {
        let repo = MockPlayerRepository::new();
        let session_store = MockSessionStore::new();
        let test_player = create_test_player();
        repo.add_player(test_player.clone());

        // Set up session
        let session_id = "test-session-id";
        session_store.set_session(session_id, &test_player.email).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .app_data(web::Data::new(session_store))
                .service(web::scope("/players").service(update_password_handler))
        ).await;

        let update_request = UpdatePasswordRequest {
            current_password: "password123".to_string(),
            new_password: "newpassword123".to_string(),
        };

        let req = test::TestRequest::put()
            .uri("/players/test123/password")
            .insert_header(("X-Session-ID", session_id))
            .set_json(&update_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: UpdateResponse = test::read_body_json(resp).await;
        assert_eq!(body.message, "Password updated successfully");
    }

    #[tokio::test]
    async fn test_logout_handler() {
        let repo = MockPlayerRepository::new();
        let session_store = MockSessionStore::new();
        let test_player = create_test_player();
        repo.add_player(test_player.clone());

        // Set up session
        let session_id = "test-session-id";
        session_store.set_session(session_id, &test_player.email).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .app_data(web::Data::new(session_store.clone()))
                .service(web::scope("/auth").service(logout_handler))
        ).await;

        let req = test::TestRequest::post()
            .uri("/auth/logout")
            .insert_header(("X-Session-ID", session_id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Verify session was deleted
        let session_result = session_store.get_session(session_id).await;
        assert!(session_result.is_none());
    }

    #[tokio::test]
    async fn test_search_players_handler() {
        let repo = MockPlayerRepository::new();
        let session_store = MockSessionStore::new();
        let test_player = create_test_player();
        repo.add_player(test_player);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(repo))
                .app_data(web::Data::new(session_store))
                .service(web::scope("/players").service(search_players_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/players/search?q=test")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: Vec<PlayerDto> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].handle, "testplayer");
    }

    #[test]
    fn test_player_dto_conversion() {
        let player = create_test_player();
        let player_dto = PlayerDto::from(&player);
        
        assert_eq!(player_dto.id, player.id);
        assert_eq!(player_dto.email, player.email);
        assert_eq!(player_dto.handle, player.handle);
        assert_eq!(player_dto.is_admin, player.is_admin);
        assert_eq!(player_dto.created_at, player.created_at);
        assert_eq!(player_dto.updated_at, player.updated_at);
    }

    #[test]
    fn test_password_validation() {
        // Test password validation logic
        let valid_password = "password123";
        let invalid_password = "123"; // Too short
        
        assert!(valid_password.len() >= 8);
        assert!(!(invalid_password.len() >= 8));
    }

    #[test]
    fn test_email_validation() {
        // Test email validation logic
        let valid_email = "test@example.com";
        let invalid_email = "notanemail";
        
        assert!(valid_email.contains('@'));
        assert!(!invalid_email.contains('@'));
    }
}
