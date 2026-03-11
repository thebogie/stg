#[cfg(test)]
mod error_tests {
    use crate::error::ApiError;

    #[test]
    fn test_error_creation() {
        let error = ApiError::new("TEST_ERROR", "Test error message", 400);

        assert_eq!(error.error, "TEST_ERROR");
        assert_eq!(error.message, "Test error message");
        assert_eq!(error.status_code, 400);
    }

    #[test]
    fn test_bad_request_error() {
        let error = ApiError::bad_request("Invalid input data");

        assert_eq!(error.error, "BAD_REQUEST");
        assert_eq!(error.message, "Invalid input data");
        assert_eq!(error.status_code, 400);
    }

    #[test]
    fn test_unauthorized_error() {
        let error = ApiError::unauthorized("Authentication required");

        assert_eq!(error.error, "UNAUTHORIZED");
        assert_eq!(error.message, "Authentication required");
        assert_eq!(error.status_code, 401);
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
        let error = ApiError::internal_error("Internal server error");

        assert_eq!(error.error, "INTERNAL_ERROR");
        assert_eq!(error.message, "Internal server error");
        assert_eq!(error.status_code, 500);
    }

    #[test]
    fn test_database_error() {
        let error = ApiError::database_error("Database connection failed");

        assert_eq!(error.error, "DATABASE_ERROR");
        assert_eq!(error.message, "Database connection failed");
        assert_eq!(error.status_code, 500);
    }

    #[test]
    fn test_validation_error() {
        let error = ApiError::validation_error("Invalid email format");

        assert_eq!(error.error, "VALIDATION_ERROR");
        assert_eq!(error.message, "Invalid email format");
        assert_eq!(error.status_code, 400);
    }

    #[test]
    fn test_error_serialization() {
        let error = ApiError::bad_request("Test message");
        let json = serde_json::to_string(&error).unwrap();

        assert!(json.contains("BAD_REQUEST"));
        assert!(json.contains("Test message"));
        assert!(json.contains("400"));
    }

    #[test]
    fn test_error_json_contains_fields() {
        let error = ApiError::bad_request("Test message");
        let json = serde_json::to_string(&error).unwrap();

        assert!(json.contains("BAD_REQUEST"));
        assert!(json.contains("Test message"));
        assert!(json.contains("400"));
    }
}
