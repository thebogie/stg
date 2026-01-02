use uuid::Uuid;
use serde::{Deserialize, Serialize};
use validator::Validate;
use crate::models::auth::{User, UserSession};
use crate::PlayerDto;

/// Data Transfer Object for User
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UserDto {
    /// Unique identifier for the user
    pub id: Uuid,
    
    /// User's display name
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    
    /// User's email address
    #[validate(email)]
    pub email: String,
}

impl From<&User> for UserDto {
    fn from(user: &User) -> Self {
        Self {
            id: user.id,
            username: user.username.clone(),
            email: user.email.clone(),
        }
    }
}

/// Data Transfer Object for User Session
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UserSessionDto {
    /// ID of the user this session belongs to
    pub user_id: Uuid,
    
    /// Unique session identifier
    #[validate(length(min = 32))]
    pub session_id: String,
}

impl From<&UserSession> for UserSessionDto {
    fn from(session: &UserSession) -> Self {
        Self {
            user_id: session.user_id,
            session_id: session.session_id.clone(),
        }
    }
}

/// Response for successful authentication (login/register)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    /// The authenticated player's data
    pub player: PlayerDto,
    
    /// The session data for the authenticated player
    pub session: UserSessionDto,
} 