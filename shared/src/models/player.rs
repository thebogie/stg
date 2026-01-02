use serde::{Deserialize, Serialize};
use validator::Validate;
use chrono::{DateTime, FixedOffset};
use crate::error::{Result, SharedError};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref HANDLE_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
}

/// Represents a player in the system
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Player {
    /// ArangoDB document ID (format: "player/{timestamp}")
    #[serde(rename = "_id")]
    pub id: String,

    /// ArangoDB document revision
    #[serde(rename = "_rev")]
    pub rev: String,

    /// Player's first name
    #[validate(length(min = 1, max = 100))]
    pub firstname: String,

    /// Player's handle/username
    #[validate(length(min = 3, max = 50))]
    #[validate(regex = "HANDLE_REGEX")]
    pub handle: String,

    /// Player's email address
    #[validate(email)]
    pub email: String,

    /// Hashed password
    #[validate(length(min = 1))]
    pub password: String,

    /// When the player was created
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<FixedOffset>,

    /// Whether the player has administrative privileges
    #[serde(rename = "isAdmin")]
    pub is_admin: bool,
}

impl Player {
    /// Creates a new player with validation
    pub fn new(
        id: String,
        rev: String,
        firstname: String,
        handle: String,
        email: String,
        password: String,
        created_at: DateTime<FixedOffset>,
        is_admin: bool,
    ) -> Result<Self> {
        let player = Self {
            id,
            rev,
            firstname,
            handle,
            email,
            password,
            created_at,
            is_admin,
        };
        player.validate_fields()?;
        Ok(player)
    }

    /// Creates a new player for database insertion (ArangoDB will set id and rev)
    pub fn new_for_db(
        firstname: String,
        handle: String,
        email: String,
        password: String,
        created_at: DateTime<FixedOffset>,
        is_admin: bool,
    ) -> Result<Self> {
        let player = Self {
            id: String::new(), // Will be set by ArangoDB
            rev: String::new(), // Will be set by ArangoDB
            firstname,
            handle,
            email,
            password,
            created_at,
            is_admin,
        };
        player.validate_fields()?;
        Ok(player)
    }

    /// Validates the player data
    pub fn validate_fields(&self) -> Result<()> {
        self.validate()
            .map_err(|e| SharedError::Validation(e.to_string()))
    }

    pub fn verify_password(&self, password: &str) -> bool {
        if let Ok(parsed_hash) = PasswordHash::new(&self.password) {
            Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
        } else {
            false
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerDto {
    pub id: String,
    pub handle: String,
    pub username: String,
    pub email: String,
    pub created_at: DateTime<FixedOffset>,
}

impl From<&Player> for PlayerDto {
    fn from(player: &Player) -> Self {
        Self {
            id: player.id.clone(),
            handle: player.handle.clone(),
            username: player.handle.clone(),
            email: player.email.clone(),
            created_at: player.created_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerRegistration {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerLogin {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerSession {
    pub player_id: String,
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserSessionDto {
    pub user_id: String,
    pub session_id: String,
}

impl From<&PlayerSession> for UserSessionDto {
    fn from(session: &PlayerSession) -> Self {
        Self {
            user_id: session.player_id.clone(),
            session_id: session.session_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use test_log::test;
    use fake::Fake;
    use fake::faker::internet::raw::SafeEmail;
    use fake::locales::EN;

    fn create_test_player() -> Player {
        Player {
            id: "player/1".to_string(),
            rev: "1".to_string(),
            firstname: "John".to_string(),
            handle: "john_doe".to_string(),
            email: "john@example.com".to_string(),
            password: "hashed_password".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        }
    }

    #[test]
    fn test_player_creation() {
        let player = create_test_player();
        assert_eq!(player.firstname, "John");
        assert_eq!(player.handle, "john_doe");
        assert_eq!(player.email, "john@example.com");
    }

    #[test]
    fn test_player_validation_success() {
        let player = create_test_player();
        assert!(player.validate().is_ok());
    }

    #[test]
    fn test_player_validation_empty_firstname() {
        let mut player = create_test_player();
        player.firstname = "".to_string();
        let result = player.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("firstname"));
    }

    #[test]
    fn test_player_validation_empty_handle() {
        let mut player = create_test_player();
        player.handle = "".to_string();
        let result = player.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("handle"));
    }

    #[test]
    fn test_player_validation_invalid_email() {
        let mut player = create_test_player();
        player.email = "invalid-email".to_string();
        let result = player.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("email"));
    }

    #[test]
    fn test_player_validation_handle_too_short() {
        let mut player = create_test_player();
        player.handle = "ab".to_string();
        let result = player.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("handle"));
    }

    #[test]
    fn test_player_validation_handle_too_long() {
        let mut player = create_test_player();
        player.handle = "a".repeat(51);
        let result = player.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("handle"));
    }

    #[test]
    fn test_player_validation_handle_invalid_chars() {
        let mut player = create_test_player();
        player.handle = "invalid handle".to_string();
        let result = player.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("handle"));
    }

    #[test]
    fn test_player_validation_handle_valid_chars() {
        let mut player = create_test_player();
        player.handle = "valid_handle123".to_string();
        assert!(player.validate().is_ok());
    }

    #[test]
    fn test_player_serialization() {
        let player = create_test_player();
        let json = serde_json::to_string(&player).unwrap();
        let deserialized: Player = serde_json::from_str(&json).unwrap();
        assert_eq!(player.id, deserialized.id);
        assert_eq!(player.firstname, deserialized.firstname);
        assert_eq!(player.handle, deserialized.handle);
        assert_eq!(player.email, deserialized.email);
    }

    #[test]
    fn test_player_with_fake_data() {
        let player = Player {
            id: "player/1".to_string(),
            rev: "1".to_string(),
            firstname: "Test".to_string(),
            handle: "test_user_123".to_string(),
            email: SafeEmail(EN).fake(),
            password: "hashed_password".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        };
        assert!(player.validate().is_ok());
    }

    #[test]
    fn test_player_id_format() {
        let player = create_test_player();
        assert!(player.id.starts_with("player/"));
    }

    #[test]
    fn test_player_rev_format() {
        let player = create_test_player();
        assert!(player.rev.parse::<i32>().is_ok());
    }

    #[test]
    fn test_player_created_at_format() {
        let player = create_test_player();
        let now = chrono::Utc::now().fixed_offset();
        assert!(player.created_at <= now);
    }

    #[test]
    fn test_player_password_not_empty() {
        let player = create_test_player();
        assert!(!player.password.is_empty());
    }

    #[test]
    fn test_player_email_case_insensitive() {
        let mut player1 = create_test_player();
        let mut player2 = create_test_player();
        player1.email = "TEST@EXAMPLE.COM".to_string();
        player2.email = "test@example.com".to_string();
        assert!(player1.validate().is_ok());
        assert!(player2.validate().is_ok());
    }

    #[test]
    fn test_player_handle_case_sensitive() {
        let mut player1 = create_test_player();
        let mut player2 = create_test_player();
        player1.handle = "TestHandle".to_string();
        player2.handle = "testhandle".to_string();
        assert!(player1.validate().is_ok());
        assert!(player2.validate().is_ok());
    }
} 