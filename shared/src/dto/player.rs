use serde::{Deserialize, Serialize};
use validator::Validate;
use chrono::{DateTime, FixedOffset};
use crate::models::player::Player;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref HANDLE_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
}

/// Data Transfer Object for Player
#[derive(Debug, Serialize, Deserialize, Validate, Clone, PartialEq)]
pub struct PlayerDto {
    /// Player's ID (ArangoDB _id field, serialized as "_id" in JSON)
    #[serde(rename = "_id")]
    pub id: String,
    #[validate(length(min = 1, message = "First name is required"))]
    pub firstname: String,
    #[validate(length(min = 1, message = "Handle is required"))]
    pub handle: String,
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[serde(rename = "createdAt")]  // Map to/from "createdAt" in database
    pub created_at: DateTime<FixedOffset>,

    /// Whether the player has administrative privileges
    #[serde(rename = "isAdmin")]
    pub is_admin: bool,
}

/// Request for player registration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreatePlayerRequest {
    /// User's display name
    #[serde(alias = "handle")]
    #[validate(length(min = 3, max = 50))]
    #[validate(regex = "HANDLE_REGEX")]
    pub username: String,
    
    /// User's password
    #[validate(length(min = 8))]
    pub password: String,
    
    /// User's email address
    #[validate(email)]
    pub email: String,

    /// Whether the user should have administrative privileges
    #[serde(default)]
    pub is_admin: bool,
}

/// Request for player login
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LoginRequest {
    /// User's email address
    #[validate(email)]
    pub email: String,
    
    /// User's password
    #[validate(length(min = 8))]
    pub password: String,
}

/// Response for successful login
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    /// The authenticated player's data
    pub player: PlayerDto,
    /// Session ID for authentication
    pub session_id: String,
}

/// Internal storage structure for player with password hash
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredPlayer {
    #[serde(flatten)]
    pub player: PlayerDto,
    #[serde(rename = "password")]  // Map to/from "password" in database
    pub password_hash: String,      // But use password_hash in code
}

/// Data Transfer Object for Player Profile
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PlayerProfileDto {
    /// Player's first name
    #[validate(length(min = 1, max = 100))]
    pub firstname: String,

    /// Player's handle/username
    #[validate(length(min = 1, max = 50))]
    pub handle: String,

    /// Player's email address
    #[validate(email)]
    pub email: String,
}

impl From<&Player> for PlayerDto {
    fn from(player: &Player) -> Self {
        Self {
            id: player.id.clone(),
            firstname: player.firstname.clone(),
            handle: player.handle.clone(),
            email: player.email.clone(),
            created_at: player.created_at,
            is_admin: player.is_admin,
        }
    }
}

impl From<PlayerDto> for Player {
    fn from(dto: PlayerDto) -> Self {
        Self::new_for_db(
            dto.firstname.clone(),
            dto.handle.clone(),
            dto.email.clone(),
            String::new(), // Password is handled separately
            dto.created_at,
            false, // Default to non-admin for new players
        ).unwrap_or_else(|_| Self {
            id: dto.id,
            rev: String::new(), // Let ArangoDB set this
            firstname: dto.firstname,
            handle: dto.handle,
            email: dto.email,
            password: String::new(), // Password is handled separately
            created_at: dto.created_at,
            is_admin: false,
        })
    }
}

impl PlayerDto {
    /// Updates a player with the DTO's values
    pub fn update_player(&self, player: &mut Player) {
        player.firstname = self.firstname.clone();
        player.handle = self.handle.clone();
        player.email = self.email.clone();
        // Note: Password is not updated from DTO
    }

    /// Validates the DTO and converts to Player if valid
    pub fn try_into_player(self) -> std::result::Result<Player, validator::ValidationErrors> {
        self.validate()?;
        Ok(Player::from(self))
    }
}

impl From<&Player> for PlayerProfileDto {
    fn from(player: &Player) -> Self {
        Self {
            firstname: player.firstname.clone(),
            handle: player.handle.clone(),
            email: player.email.clone(),
        }
    }
}

/// Request for updating player email
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateEmailRequest {
    /// New email address
    #[validate(email)]
    pub email: String,
    
    /// Current password for verification
    #[validate(length(min = 1))]
    pub password: String,
}

/// Request for updating player handle
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateHandleRequest {
    /// New handle/username
    #[validate(length(min = 3, max = 50))]
    #[validate(regex = "HANDLE_REGEX")]
    pub handle: String,
    
    /// Current password for verification
    #[validate(length(min = 1))]
    pub password: String,
}

/// Request for updating player password
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdatePasswordRequest {
    /// Current password for verification
    #[validate(length(min = 1))]
    pub current_password: String,
    
    /// New password
    #[validate(length(min = 8))]
    pub new_password: String,
}

/// Response for successful update operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResponse {
    /// Success message
    pub message: String,
    /// Updated player data
    pub player: PlayerDto,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use test_log::test;
    use fake::{Fake};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::internet::raw::Username;
    use fake::locales::EN;
    use validator::Validate;
    use crate::models::player::Player;

    fn create_test_player_dto() -> PlayerDto {
        PlayerDto {
            id: "player/1".to_string(),
            firstname: "John".to_string(),
            handle: "john_doe".to_string(),
            email: "john@example.com".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        }
    }

    fn create_test_create_player_request() -> CreatePlayerRequest {
        CreatePlayerRequest {
            username: "john_doe".to_string(),
            email: "john@example.com".to_string(),
            password: "password123".to_string(),
            is_admin: false,
        }
    }

    fn create_test_login_response() -> LoginResponse {
        LoginResponse {
            session_id: "test_session".to_string(),
            player: create_test_player_dto(),
        }
    }

    fn create_test_login_request() -> LoginRequest {
        LoginRequest {
            email: "testuser@example.com".to_string(),
            password: "testpass123".to_string(),
        }
    }

    #[test]
    fn test_player_dto_creation() {
        let dto = create_test_player_dto();
        assert_eq!(dto.firstname, "John");
        assert_eq!(dto.handle, "john_doe");
        assert_eq!(dto.email, "john@example.com");
    }

    #[test]
    fn test_player_dto_validation_success() {
        let dto = create_test_player_dto();
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn test_player_dto_validation_empty_firstname() {
        let mut dto = create_test_player_dto();
        dto.firstname = "".to_string();
        let result = dto.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("firstname"));
    }

    #[test]
    fn test_player_dto_validation_invalid_email_format() {
        let dto = PlayerDto {
            id: "player/1".to_string(),
            email: "not-an-email".to_string(),
            firstname: "Player".to_string(),
            handle: "john_doe".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        };
        assert!(dto.validate().is_err());
    }

    #[test]
    fn test_player_dto_serialization() {
        let dto = create_test_player_dto();
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: PlayerDto = serde_json::from_str(&json).unwrap();
        assert_eq!(dto.id, deserialized.id);
        assert_eq!(dto.firstname, deserialized.firstname);
        assert_eq!(dto.handle, deserialized.handle);
        assert_eq!(dto.email, deserialized.email);
    }

    #[test]
    fn test_create_player_request_validation_success() {
        let request = create_test_create_player_request();
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_create_player_request_validation_empty_firstname() {
        let mut request = create_test_create_player_request();
        request.username = "".to_string();
        let result = request.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("username"));
    }

    #[test]
    fn test_create_player_request_validation_short_password() {
        let mut request = create_test_create_player_request();
        request.password = "123".to_string();
        let result = request.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("password"));
    }

    #[test]
    fn test_create_player_request_validation_handle_too_short() {
        let mut request = create_test_create_player_request();
        request.username = "ab".to_string();
        let result = request.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("username"));
    }

    #[test]
    fn test_create_player_request_validation_handle_invalid_chars() {
        let mut request = create_test_create_player_request();
        request.username = "invalid handle".to_string();
        let result = request.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("username"));
    }

    #[test]
    fn test_create_player_request_serialization() {
        let request = create_test_create_player_request();
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: CreatePlayerRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.username, deserialized.username);
        assert_eq!(request.email, deserialized.email);
        assert_eq!(request.password, deserialized.password);
    }

    #[test]
    fn test_login_response_creation() {
        let response = create_test_login_response();
        assert_eq!(response.player.firstname, "John");
        assert_eq!(response.session_id, "test_session");
    }

    #[test]
    fn test_login_response_serialization() {
        let response = create_test_login_response();
        let json = serde_json::to_string(&response).unwrap();
        let deserialized: LoginResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response.player.id, deserialized.player.id);
        assert_eq!(response.session_id, deserialized.session_id);
    }

    #[test]
    fn test_stored_player_creation() {
        let stored = StoredPlayer {
            player: create_test_player_dto(),
            password_hash: "hashed_password".to_string(),
        };
        assert_eq!(stored.player.firstname, "John");
        assert_eq!(stored.password_hash, "hashed_password");
    }

    #[test]
    fn test_stored_player_validation() {
        let stored = StoredPlayer {
            player: create_test_player_dto(),
            password_hash: "hashed_password".to_string(),
        };
        assert!(stored.player.validate().is_ok());
    }

    #[test]
    fn test_player_profile_dto_creation() {
        let profile = PlayerProfileDto {
            firstname: "John".to_string(),
            handle: "john_doe".to_string(),
            email: "john@example.com".to_string(),
        };
        assert_eq!(profile.firstname, "John");
    }

    #[test]
    fn test_player_profile_dto_validation() {
        let profile = PlayerProfileDto {
            firstname: "John".to_string(),
            handle: "john_doe".to_string(),
            email: "john@example.com".to_string(),
        };
        assert!(profile.validate().is_ok());
    }

    #[test]
    fn test_player_dto_from_model() {
        let player = Player {
            id: "player/1".to_string(),
            rev: "1".to_string(),
            firstname: "John".to_string(),
            handle: "john_doe".to_string(),
            email: "john@example.com".to_string(),
            password: "hashed_password".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        };
        
        let dto = PlayerDto::from(&player);
        assert_eq!(dto.id, "player/1");
        assert_eq!(dto.firstname, "John");
        assert_eq!(dto.handle, "john_doe");
        assert_eq!(dto.email, "john@example.com");
    }

    #[test]
    fn test_stored_player_from_model() {
        let player = Player {
            id: "player/1".to_string(),
            rev: "1".to_string(),
            firstname: "John".to_string(),
            handle: "john_doe".to_string(),
            email: "john@example.com".to_string(),
            password: "hashed_password".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        };
        
        // Note: StoredPlayer doesn't have a From implementation, so we'll test manual creation
        let stored = StoredPlayer {
            player: PlayerDto::from(&player),
            password_hash: player.password,
        };
        assert_eq!(stored.player.id, "player/1");
        assert_eq!(stored.player.firstname, "John");
        assert_eq!(stored.password_hash, "hashed_password");
    }

    #[test]
    fn test_player_profile_dto_from_model() {
        let player = Player {
            id: "player/1".to_string(),
            rev: "1".to_string(),
            firstname: "John".to_string(),
            handle: "john_doe".to_string(),
            email: "john@example.com".to_string(),
            password: "hashed_password".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        };
        
        let profile = PlayerProfileDto::from(&player);
        assert_eq!(profile.firstname, "John");
        assert_eq!(profile.handle, "john_doe");
        assert_eq!(profile.email, "john@example.com");
    }

    #[test]
    fn test_dto_with_fake_data() {
        let dto = PlayerDto {
            id: "player/1".to_string(),
            firstname: "Test".to_string(),
            handle: Username(EN).fake(),
            email: SafeEmail().fake(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        };
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn test_create_player_request_with_fake_data() {
        let request = CreatePlayerRequest {
            username: Username(EN).fake(),
            email: SafeEmail().fake(),
            password: "secure_password_123".to_string(),
            is_admin: false,
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_email_case_insensitive_validation() {
        let mut request = create_test_login_request();
        request.email = "TEST@EXAMPLE.COM".to_string();
        assert!(request.validate().is_ok());
        
        request.email = "test@example.com".to_string();
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_handle_case_sensitive_validation() {
        let mut dto = create_test_player_dto();
        dto.handle = "TestHandle".to_string();
        assert!(dto.validate().is_ok());
        
        dto.handle = "testhandle".to_string();
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn test_password_validation_edge_cases() {
        let mut request = create_test_login_request();
        
        // Test minimum length (8 characters)
        request.password = "12345678".to_string();
        assert!(request.validate().is_ok());
        
        // Test just below minimum
        request.password = "1234567".to_string();
        let result = request.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("password"));
        
        // Test with special characters
        request.password = "pass@word123!".to_string();
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let request = create_test_login_request();
        let response = create_test_login_response();
        
        // LoginRequest roundtrip
        let request_json = serde_json::to_string(&request).unwrap();
        let request_deserialized: LoginRequest = serde_json::from_str(&request_json).unwrap();
        assert_eq!(request.email, request_deserialized.email);
        assert_eq!(request.password, request_deserialized.password);
        
        // LoginResponse roundtrip
        let response_json = serde_json::to_string(&response).unwrap();
        let response_deserialized: LoginResponse = serde_json::from_str(&response_json).unwrap();
        assert_eq!(response.player.id, response_deserialized.player.id);
        assert_eq!(response.session_id, response_deserialized.session_id);
    }

    #[test]
    fn test_player_dto_with_special_characters() {
        let player = PlayerDto {
            id: "player/1".to_string(),
            firstname: "John & Jane".to_string(),
            handle: "john_jane_123".to_string(),
            email: "john.jane+test@example.com".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        };
        assert!(player.validate().is_ok());
    }

    #[test]
    fn test_login_request_with_special_characters() {
        let request = LoginRequest {
            email: "user.name+tag@example.com".to_string(),
            password: "pass@word123!".to_string(),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_player_dto_validation_empty_email() {
        let dto = PlayerDto {
            id: "player/1".to_string(),
            email: "".to_string(),
            firstname: "Player".to_string(),
            handle: "john_doe".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        };
        assert!(dto.validate().is_err());
    }

    #[test]
    fn test_player_login_validation_empty_password() {
        let login = LoginRequest {
            email: "player@example.com".to_string(),
            password: "".to_string(),
        };
        assert!(login.validate().is_err());
    }

    #[test]
    fn test_player_dto_to_model_invalid() {
        let dto = PlayerDto {
            id: "player/1".to_string(),
            email: "".to_string(), // Invalid: empty email
            firstname: "Player".to_string(),
            handle: "john_doe".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        };
        let result = dto.try_into_player();
        assert!(result.is_err());
    }
} 