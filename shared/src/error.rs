use thiserror::Error;
use chrono::{DateTime, Utc};
use validator::ValidationErrors;
use serde_json::Error as JsonError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum SharedError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Conversion error: {0}")]
    Conversion(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Internal server error: {0}")]
    InternalServerError(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Date range error: start date {start} must be before end date {end}")]
    InvalidDateRange {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },

    #[error("Invalid email format: {0}")]
    InvalidEmail(String),

    #[error("Invalid UUID format: {0}")]
    InvalidUuid(String),

    #[error("Required field missing: {0}")]
    MissingField(String),
}

#[cfg(not(target_arch = "wasm32"))]
impl actix_web::ResponseError for SharedError {
    fn error_response(&self) -> actix_web::HttpResponse {
        match self {
            SharedError::Validation(_) => actix_web::HttpResponse::BadRequest().json(self),
            SharedError::NotFound(_) => actix_web::HttpResponse::NotFound().json(self),
            SharedError::Unauthorized(_) => actix_web::HttpResponse::Unauthorized().json(self),
            SharedError::Forbidden(_) => actix_web::HttpResponse::Forbidden().json(self),
            SharedError::BadRequest(_) => actix_web::HttpResponse::BadRequest().json(self),
            SharedError::Conflict(_) => actix_web::HttpResponse::Conflict().json(self),
            SharedError::Database(_) => actix_web::HttpResponse::InternalServerError().json(self),
            SharedError::Conversion(_) => actix_web::HttpResponse::BadRequest().json(self),
            SharedError::Internal(_) => actix_web::HttpResponse::InternalServerError().json(self),
            SharedError::InternalServerError(_) => actix_web::HttpResponse::InternalServerError().json(self),
            SharedError::NotImplemented(_) => actix_web::HttpResponse::NotImplemented().json(self),
            SharedError::InvalidDateRange { .. } => actix_web::HttpResponse::BadRequest().json(self),
            SharedError::InvalidEmail(_) => actix_web::HttpResponse::BadRequest().json(self),
            SharedError::InvalidUuid(_) => actix_web::HttpResponse::BadRequest().json(self),
            SharedError::MissingField(_) => actix_web::HttpResponse::BadRequest().json(self),
        }
    }
}

impl From<ValidationErrors> for SharedError {
    fn from(errors: ValidationErrors) -> Self {
        Self::Validation(errors.to_string())
    }
}

impl From<JsonError> for SharedError {
    fn from(error: JsonError) -> Self {
        Self::Conversion(error.to_string())
    }
}

pub type Result<T> = std::result::Result<T, SharedError>; 