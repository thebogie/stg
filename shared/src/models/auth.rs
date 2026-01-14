use crate::error::{Result, SharedError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Request for user registration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RegisterRequest {
    /// User's display name
    #[validate(length(min = 3, max = 50))]
    pub username: String,

    /// User's password
    #[validate(length(min = 8))]
    pub password: String,

    /// User's email address
    #[validate(email)]
    pub email: String,

    /// User's full name
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    /// User's country code
    #[validate(length(equal = 2))]
    pub country: String,
}

/// Request for user login
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LoginRequest {
    /// User's email address
    #[validate(email)]
    pub email: String,

    /// User's password
    #[validate(length(min = 8))]
    pub password: String,
}

/// Represents a user in the system
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct User {
    /// Unique identifier for the user
    pub id: Uuid,

    /// User's display name
    #[validate(length(min = 3, max = 50))]
    pub username: String,

    /// User's email address
    #[validate(email)]
    pub email: String,
}

impl User {
    /// Creates a new user with validation
    pub fn new(id: Uuid, username: String, email: String) -> Result<Self> {
        let user = Self {
            id,
            username,
            email,
        };
        user.validate_fields()?;
        Ok(user)
    }

    /// Validates the user's data
    pub fn validate_fields(&self) -> Result<()> {
        self.validate()
            .map_err(|e| SharedError::Validation(e.to_string()))
    }
}

/// Represents an active user session
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UserSession {
    /// ID of the user this session belongs to
    pub user_id: Uuid,

    /// Unique session identifier
    #[validate(length(min = 32))]
    pub session_id: String,
}

impl UserSession {
    /// Creates a new user session with validation
    pub fn new(user_id: Uuid, session_id: String) -> Result<Self> {
        let session = Self {
            user_id,
            session_id,
        };
        session.validate_fields()?;
        Ok(session)
    }

    /// Validates the session data
    pub fn validate_fields(&self) -> Result<()> {
        self.validate()
            .map_err(|e| SharedError::Validation(e.to_string()))
    }
}
