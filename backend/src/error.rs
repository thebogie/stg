use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
    pub status_code: u16,
}

impl ApiError {
    pub fn new(error: &str, message: &str, status_code: u16) -> Self {
        Self {
            error: error.to_string(),
            message: message.to_string(),
            status_code,
        }
    }

    pub fn bad_request(message: &str) -> Self {
        Self::new("BAD_REQUEST", message, 400)
    }

    #[allow(dead_code)]
    pub fn unauthorized(message: &str) -> Self {
        Self::new("UNAUTHORIZED", message, 401)
    }

    #[allow(dead_code)]
    pub fn forbidden(message: &str) -> Self {
        Self::new("FORBIDDEN", message, 403)
    }

    #[allow(dead_code)]
    pub fn not_found(message: &str) -> Self {
        Self::new("NOT_FOUND", message, 404)
    }

    pub fn internal_error(message: &str) -> Self {
        Self::new("INTERNAL_ERROR", message, 500)
    }

    pub fn database_error(message: &str) -> Self {
        Self::new("DATABASE_ERROR", message, 500)
    }

    pub fn validation_error(message: &str) -> Self {
        Self::new("VALIDATION_ERROR", message, 400)
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let status = match actix_web::http::StatusCode::from_u16(self.status_code) {
            Ok(status) => status,
            Err(_) => {
                log::warn!("Invalid status code {}, defaulting to 500", self.status_code);
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        HttpResponse::build(status).json(self)
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.error, self.message)
    }
}

impl From<arangors::ClientError> for ApiError {
    fn from(err: arangors::ClientError) -> Self {
        Self::database_error(&format!("Database error: {}", err))
    }
}

impl From<redis::RedisError> for ApiError {
    fn from(err: redis::RedisError) -> Self {
        Self::internal_error(&format!("Redis error: {}", err))
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        Self::bad_request(&format!("JSON error: {}", err))
    }
}

impl From<validator::ValidationErrors> for ApiError {
    fn from(err: validator::ValidationErrors) -> Self {
        Self::validation_error(&format!("Validation error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_creation() {
        let error = ApiError::new("TEST_ERROR", "Test message", 400);
        assert_eq!(error.error, "TEST_ERROR");
        assert_eq!(error.message, "Test message");
        assert_eq!(error.status_code, 400);
    }

    #[test]
    fn test_bad_request_error() {
        let error = ApiError::bad_request("Invalid input");
        assert_eq!(error.error, "BAD_REQUEST");
        assert_eq!(error.message, "Invalid input");
        assert_eq!(error.status_code, 400);
    }

    #[test]
    fn test_unauthorized_error() {
        let error = ApiError::unauthorized("Not authenticated");
        assert_eq!(error.error, "UNAUTHORIZED");
        assert_eq!(error.message, "Not authenticated");
        assert_eq!(error.status_code, 401);
    }

    #[test]
    fn test_forbidden_error() {
        let error = ApiError::forbidden("Access denied");
        assert_eq!(error.error, "FORBIDDEN");
        assert_eq!(error.message, "Access denied");
        assert_eq!(error.status_code, 403);
    }

    #[test]
    fn test_not_found_error() {
        let error = ApiError::not_found("Resource not found");
        assert_eq!(error.error, "NOT_FOUND");
        assert_eq!(error.message, "Resource not found");
        assert_eq!(error.status_code, 404);
    }

    #[test]
    fn test_internal_error() {
        let error = ApiError::internal_error("Server error");
        assert_eq!(error.error, "INTERNAL_ERROR");
        assert_eq!(error.message, "Server error");
        assert_eq!(error.status_code, 500);
    }

    #[test]
    fn test_database_error() {
        let error = ApiError::database_error("Connection failed");
        assert_eq!(error.error, "DATABASE_ERROR");
        assert_eq!(error.message, "Connection failed");
        assert_eq!(error.status_code, 500);
    }

    #[test]
    fn test_validation_error() {
        let error = ApiError::validation_error("Invalid email");
        assert_eq!(error.error, "VALIDATION_ERROR");
        assert_eq!(error.message, "Invalid email");
        assert_eq!(error.status_code, 400);
    }

    #[test]
    fn test_display_format() {
        let error = ApiError::bad_request("Test message");
        let display = format!("{}", error);
        assert_eq!(display, "BAD_REQUEST: Test message");
    }

    #[test]
    fn test_error_response_format() {
        let error = ApiError::bad_request("Test message");
        let response = error.error_response();

        assert_eq!(response.status().as_u16(), 400);
    }

    #[test]
    fn test_from_arangors_error() {
        // Use a valid variant for arangors::ClientError
        let arango_error = arangors::ClientError::InvalidServer("test error".to_string());
        let api_error: ApiError = arango_error.into();

        assert_eq!(api_error.error, "DATABASE_ERROR");
        assert!(api_error.message.contains("Database error"));
        assert_eq!(api_error.status_code, 500);
    }

    #[test]
    fn test_from_redis_error() {
        // Create a mock Redis error
        let redis_error = redis::RedisError::from(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "connection refused"
        ));
        let api_error: ApiError = redis_error.into();

        assert_eq!(api_error.error, "INTERNAL_ERROR");
        assert!(api_error.message.contains("Redis error"));
        assert_eq!(api_error.status_code, 500);
    }

    #[test]
    fn test_from_serde_json_error() {
        // Create a mock JSON error
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let api_error: ApiError = json_error.into();

        assert_eq!(api_error.error, "BAD_REQUEST");
        assert!(api_error.message.contains("JSON error"));
        assert_eq!(api_error.status_code, 400);
    }

    #[test]
    fn test_from_validation_errors() {
        use validator::{ValidationError, ValidationErrors};

        // Create a mock validation error
        let mut errors = ValidationErrors::new();
        let mut validation_error = ValidationError::new("test");
        validation_error.message = Some("Invalid field".into());
        errors.add("field", validation_error);

        let api_error: ApiError = errors.into();

        assert_eq!(api_error.error, "VALIDATION_ERROR");
        assert!(api_error.message.contains("Validation error"));
        assert_eq!(api_error.status_code, 400);
    }
} 
