#[cfg(test)]
mod player_tests {
    // use super::*;
    use shared::dto::player::{PlayerDto, CreatePlayerRequest, LoginRequest};
    use shared::models::player::Player;
    use chrono::Utc;

    #[test]
    fn test_player_dto_creation() {
        let player_dto = PlayerDto {
            id: "player/test".to_string(),
            firstname: "John".to_string(),
            handle: "testuser".to_string(),
            email: "test@example.com".to_string(),
            created_at: Utc::now().fixed_offset(),
            is_admin: false,
        };
        
        assert_eq!(player_dto.firstname, "John");
        assert_eq!(player_dto.handle, "testuser");
        assert_eq!(player_dto.email, "test@example.com");
        assert_eq!(player_dto.is_admin, false);
    }

    #[test]
    fn test_create_player_request() {
        let request = CreatePlayerRequest {
            username: "janeuser".to_string(),
            password: "password123".to_string(),
            email: "jane@example.com".to_string(),
            is_admin: false,
        };
        
        assert_eq!(request.username, "janeuser");
        assert_eq!(request.password, "password123");
        assert_eq!(request.email, "jane@example.com");
        assert_eq!(request.is_admin, false);
    }

    #[test]
    fn test_login_request() {
        let request = LoginRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };
        
        assert_eq!(request.email, "test@example.com");
        assert_eq!(request.password, "password123");
    }

    #[test]
    fn test_player_model_creation() {
        let player = Player {
            id: "player/test".to_string(),
            rev: "1".to_string(),
            firstname: "John".to_string(),
            handle: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
            created_at: Utc::now().fixed_offset(),
            is_admin: false,
        };
        
        assert_eq!(player.firstname, "John");
        assert_eq!(player.handle, "testuser");
        assert_eq!(player.email, "test@example.com");
        assert_eq!(player.is_admin, false);
    }

    #[test]
    fn test_player_serialization() {
        let player = Player {
            id: "player/test".to_string(),
            rev: "1".to_string(),
            firstname: "John".to_string(),
            handle: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "hashed_password".to_string(),
            created_at: Utc::now().fixed_offset(),
            is_admin: false,
        };
        
        let json = serde_json::to_string(&player).unwrap();
        let deserialized: Player = serde_json::from_str(&json).unwrap();
        
        assert_eq!(player.id, deserialized.id);
        assert_eq!(player.firstname, deserialized.firstname);
        assert_eq!(player.handle, deserialized.handle);
        assert_eq!(player.is_admin, deserialized.is_admin);
    }
}