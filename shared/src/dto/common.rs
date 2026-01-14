use serde::{Deserialize, Serialize};
use validator::Validate;

/// Common search query parameters
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SearchQuery {
    /// The search query string
    #[validate(length(min = 1, message = "Search query cannot be empty"))]
    pub query: String,
}

/// Common error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
}

/// Common authentication response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    /// The authenticated player's data
    pub player: crate::dto::player::PlayerDto,
    /// The session data
    pub session: crate::dto::auth::UserSessionDto,
}
