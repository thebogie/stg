use std::fmt;
use crate::error::ApiError;

#[derive(Debug, PartialEq, Eq)]
pub enum PlayerError {
    NotFound,
    InvalidPassword,
    AlreadyExists,
    DatabaseError(String),
    SessionError(String),
}

impl fmt::Display for PlayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlayerError::NotFound => write!(f, "Player not found"),
            PlayerError::InvalidPassword => write!(f, "Invalid password"),
            PlayerError::AlreadyExists => write!(f, "Player already exists"),
            PlayerError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            PlayerError::SessionError(msg) => write!(f, "Session error: {}", msg),
        }
    }
}

impl From<PlayerError> for ApiError {
    fn from(err: PlayerError) -> Self {
        match err {
            PlayerError::NotFound => ApiError::not_found(&err.to_string()),
            PlayerError::InvalidPassword => ApiError::unauthorized(&err.to_string()),
            PlayerError::AlreadyExists => ApiError::bad_request(&err.to_string()),
            PlayerError::DatabaseError(msg) => ApiError::database_error(&msg),
            PlayerError::SessionError(msg) => ApiError::internal_error(&msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_error_display() {
        assert_eq!(PlayerError::NotFound.to_string(), "Player not found");
        assert_eq!(PlayerError::InvalidPassword.to_string(), "Invalid password");
        assert_eq!(
            PlayerError::DatabaseError("connection failed".to_string()).to_string(),
            "Database error: connection failed"
        );
        assert_eq!(
            PlayerError::SessionError("redis error".to_string()).to_string(),
            "Session error: redis error"
        );
    }

    #[test]
    fn test_player_error_to_api_error() {
        let api_error: ApiError = PlayerError::NotFound.into();
        assert_eq!(api_error.error, "NOT_FOUND");
        assert_eq!(api_error.message, "Player not found");
        assert_eq!(api_error.status_code, 404);

        let api_error: ApiError = PlayerError::InvalidPassword.into();
        assert_eq!(api_error.error, "UNAUTHORIZED");
        assert_eq!(api_error.message, "Invalid password");
        assert_eq!(api_error.status_code, 401);
    }
}