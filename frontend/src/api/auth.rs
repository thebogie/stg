use crate::api::api_url;
use gloo_net::http::Request;
use gloo_storage::Storage;
use js_sys::Date;
use log::debug;
use serde::Deserialize;
use shared::dto::player::{
    CreatePlayerRequest, LoginRequest, LoginResponse, PlayerDto, UpdateEmailRequest,
    UpdateHandleRequest, UpdatePasswordRequest, UpdateResponse,
};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
}

/// Result of checking current session: either success, session expired (401), or other error (network, 5xx, etc.)
#[derive(Debug)]
pub enum SessionCheckResult {
    Ok(PlayerDto),
    SessionExpired,
    Other(String),
}

pub async fn register(
    username: &str,
    email: &str,
    password: &str,
    _name: &str,
    _country: &str,
) -> Result<PlayerDto, String> {
    debug!("Registering new player: {}", email);

    let register_request = CreatePlayerRequest {
        username: username.to_string(),
        email: email.to_string(),
        password: password.to_string(),
        is_admin: false,
    };

    let response = Request::post(&api_url("/api/register"))
        .json(&register_request)
        .map_err(|e| format!("Failed to serialize register request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send register request: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|e| format!("Failed to parse error response: {}", e))?;
        return Err(error.error);
    }

    let player = response
        .json::<PlayerDto>()
        .await
        .map_err(|e| format!("Failed to parse player response: {}", e))?;

    debug!("Successfully registered player: {}", player.email);
    Ok(player)
}

pub async fn login(email: &str, password: &str) -> Result<LoginResponse, String> {
    debug!("Attempting login for user: {}", email);

    let login_request = LoginRequest {
        email: email.to_string(),
        password: password.to_string(),
    };

    let response = Request::post(&api_url("/api/players/login"))
        .json(&login_request)
        .map_err(|e| format!("Failed to serialize login request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send login request: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|e| format!("Failed to parse error response: {}", e))?;
        return Err(error.error);
    }

    let login_response = response
        .json::<LoginResponse>()
        .await
        .map_err(|e| format!("Failed to parse login response: {}", e))?;

    debug!("Login successful for user: {}", login_response.player.email);
    Ok(login_response)
}

pub async fn logout() -> Result<(), String> {
    debug!("Attempting logout");
    // Include Authorization header with current session_id if present
    let session_id = gloo_storage::LocalStorage::get::<String>("session_id").ok();
    let mut req = Request::post(&api_url("/api/players/logout"));
    if let Some(sid) = session_id {
        req = req.header("Authorization", &format!("Bearer {}", sid));
    }

    let response = req
        .send()
        .await
        .map_err(|e| format!("Failed to send logout request: {}", e))?;

    if !response.ok() {
        let error = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error occurred".to_string());
        return Err(format!("Logout failed: {}", error));
    }

    debug!("Logout successful");
    Ok(())
}

/// Min ms between any session check (stops request spam from any caller).
const SESSION_CHECK_THROTTLE_MS: u64 = 60_000;
static LAST_SESSION_CHECK_MS: AtomicU64 = AtomicU64::new(0);

/// Reset session-check throttle. Call after login so the next check runs.
pub fn reset_session_check_throttle() {
    LAST_SESSION_CHECK_MS.store(0, Ordering::Relaxed);
}

/// Check current session. Use this for heartbeat/validation: only SessionExpired should trigger logout.
/// Throttled globally to at most once per SESSION_CHECK_THROTTLE_MS.
pub async fn get_current_player_result() -> SessionCheckResult {
    let now_ms = Date::now() as u64;
    let last = LAST_SESSION_CHECK_MS.load(Ordering::Relaxed);
    if last != 0 && now_ms.saturating_sub(last) < SESSION_CHECK_THROTTLE_MS {
        return SessionCheckResult::Other("throttled".to_string());
    }
    LAST_SESSION_CHECK_MS.store(now_ms, Ordering::Relaxed);

    debug!("Fetching current player");

    let session_id = gloo_storage::LocalStorage::get::<String>("session_id").ok();
    let mut req = Request::get(&api_url("/api/players/me"));
    if let Some(sid) = session_id {
        req = req.header("Authorization", &format!("Bearer {}", sid));
    }

    let response = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            debug!("Session check request failed (network): {}", e);
            return SessionCheckResult::Other(format!("Connection error: {}", e));
        }
    };

    if response.status() == 401 || response.status() == 403 {
        return SessionCheckResult::SessionExpired;
    }

    if !response.ok() {
        let msg = response
            .json::<ErrorResponse>()
            .await
            .map(|e| e.error)
            .unwrap_or_else(|_| format!("HTTP {}", response.status()));
        return SessionCheckResult::Other(msg);
    }

    match response.json::<PlayerDto>().await {
        Ok(player) => {
            debug!("Successfully fetched current player: {}", player.email);
            SessionCheckResult::Ok(player)
        }
        Err(e) => SessionCheckResult::Other(format!("Invalid response: {}", e)),
    }
}

pub async fn get_current_player() -> Result<PlayerDto, String> {
    match get_current_player_result().await {
        SessionCheckResult::Ok(p) => Ok(p),
        SessionCheckResult::SessionExpired => Err("Session expired".to_string()),
        SessionCheckResult::Other(s) => Err(s),
    }
}

pub async fn update_profile(profile: PlayerDto) -> Result<PlayerDto, String> {
    debug!("Updating player profile");

    let response = Request::put(&api_url("/api/players/profile"))
        .json(&profile)
        .map_err(|e| format!("Failed to serialize profile update: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send profile update: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|e| format!("Failed to parse error response: {}", e))?;
        return Err(error.error);
    }

    let updated_player = response
        .json::<PlayerDto>()
        .await
        .map_err(|e| format!("Failed to parse profile update response: {}", e))?;

    debug!(
        "Successfully updated player profile: {}",
        updated_player.email
    );
    Ok(updated_player)
}

pub async fn refresh_token(refresh_token: &str) -> Result<LoginResponse, String> {
    debug!("Attempting to refresh token");

    let response = Request::post(&api_url("/api/auth/refresh"))
        .header("Authorization", &format!("Bearer {}", refresh_token))
        .send()
        .await
        .map_err(|e| format!("Failed to send refresh request: {}", e))?;

    if !response.ok() {
        let error = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error occurred".to_string());
        return Err(format!("Token refresh failed: {}", error));
    }

    let login_response = response
        .json::<LoginResponse>()
        .await
        .map_err(|e| format!("Failed to parse refresh response: {}", e))?;

    debug!(
        "Token refresh successful for user: {}",
        login_response.player.email
    );
    Ok(login_response)
}

pub async fn update_email(
    new_email: &str,
    current_password: &str,
) -> Result<UpdateResponse, String> {
    debug!("Attempting to update email to: {}", new_email);

    let update_request = UpdateEmailRequest {
        email: new_email.to_string(),
        password: current_password.to_string(),
    };

    let response = crate::api::utils::authenticated_put(&api_url("/api/players/me/email"))
        .json(&update_request)
        .map_err(|e| format!("Failed to serialize email update request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send email update request: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|e| format!("Failed to parse error response: {}", e))?;
        return Err(error.error);
    }

    let update_response = response
        .json::<UpdateResponse>()
        .await
        .map_err(|e| format!("Failed to parse update response: {}", e))?;

    debug!("Successfully updated email to: {}", new_email);
    Ok(update_response)
}

pub async fn update_handle(
    new_handle: &str,
    current_password: &str,
) -> Result<UpdateResponse, String> {
    debug!("Attempting to update handle to: {}", new_handle);

    let update_request = UpdateHandleRequest {
        handle: new_handle.to_string(),
        password: current_password.to_string(),
    };

    let response = crate::api::utils::authenticated_put(&api_url("/api/players/me/handle"))
        .json(&update_request)
        .map_err(|e| format!("Failed to serialize handle update request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send handle update request: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|e| format!("Failed to parse error response: {}", e))?;
        return Err(error.error);
    }

    let update_response = response
        .json::<UpdateResponse>()
        .await
        .map_err(|e| format!("Failed to parse update response: {}", e))?;

    debug!("Successfully updated handle to: {}", new_handle);
    Ok(update_response)
}

pub async fn update_password(
    current_password: &str,
    new_password: &str,
) -> Result<UpdateResponse, String> {
    debug!("Attempting to update password");

    let update_request = UpdatePasswordRequest {
        current_password: current_password.to_string(),
        new_password: new_password.to_string(),
    };

    let response = crate::api::utils::authenticated_put(&api_url("/api/players/me/password"))
        .json(&update_request)
        .map_err(|e| format!("Failed to serialize password update request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send password update request: {}", e))?;

    if !response.ok() {
        let error = response
            .json::<ErrorResponse>()
            .await
            .map_err(|e| format!("Failed to parse error response: {}", e))?;
        return Err(error.error);
    }

    let update_response = response
        .json::<UpdateResponse>()
        .await
        .map_err(|e| format!("Failed to parse update response: {}", e))?;

    debug!("Successfully updated password");
    Ok(update_response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn create_test_login_request() -> LoginRequest {
        LoginRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        }
    }

    fn create_test_login_response() -> LoginResponse {
        LoginResponse {
            player: PlayerDto {
                id: "player/1".to_string(),
                firstname: "John".to_string(),
                handle: "john_doe".to_string(),
                email: "test@example.com".to_string(),
                created_at: chrono::Utc::now().fixed_offset(),
                is_admin: false,
            },
            session_id: "session_123".to_string(),
        }
    }

    #[test]
    fn test_login_request_creation() {
        let request = create_test_login_request();
        assert_eq!(request.email, "test@example.com");
        assert_eq!(request.password, "password123");
    }

    #[test]
    fn test_login_request_validation() {
        let request = create_test_login_request();
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_login_request_serialization() {
        let request = create_test_login_request();
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: LoginRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.email, deserialized.email);
        assert_eq!(request.password, deserialized.password);
    }

    #[test]
    fn test_login_response_creation() {
        let response = create_test_login_response();
        assert_eq!(response.player.firstname, "John");
        assert_eq!(response.session_id, "session_123");
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
    fn test_player_dto_creation() {
        let player = PlayerDto {
            id: "player/1".to_string(),
            firstname: "John".to_string(),
            handle: "john_doe".to_string(),
            email: "test@example.com".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        };
        assert_eq!(player.firstname, "John");
        assert_eq!(player.handle, "john_doe");
        assert_eq!(player.email, "test@example.com");
    }

    #[test]
    fn test_player_dto_validation() {
        let player = PlayerDto {
            id: "player/1".to_string(),
            firstname: "John".to_string(),
            handle: "john_doe".to_string(),
            email: "test@example.com".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        };
        assert!(player.validate().is_ok());
    }

    #[test]
    fn test_player_dto_serialization() {
        let player = PlayerDto {
            id: "player/1".to_string(),
            firstname: "John".to_string(),
            handle: "john_doe".to_string(),
            email: "test@example.com".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        };
        let json = serde_json::to_string(&player).unwrap();
        let deserialized: PlayerDto = serde_json::from_str(&json).unwrap();
        assert_eq!(player.id, deserialized.id);
        assert_eq!(player.firstname, deserialized.firstname);
        assert_eq!(player.handle, deserialized.handle);
        assert_eq!(player.email, deserialized.email);
    }

    #[test]
    fn test_login_request_validation_empty_email() {
        let mut request = create_test_login_request();
        request.email = "".to_string();
        let result = request.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("email"));
    }

    #[test]
    fn test_login_request_validation_invalid_email() {
        let mut request = create_test_login_request();
        request.email = "invalid-email".to_string();
        let result = request.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("email"));
    }

    #[test]
    fn test_login_request_validation_empty_password() {
        let mut request = create_test_login_request();
        request.password = "".to_string();
        let result = request.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("password"));
    }

    #[test]
    fn test_login_request_validation_short_password() {
        let mut request = create_test_login_request();
        request.password = "123".to_string();
        let result = request.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("password"));
    }

    #[test]
    fn test_player_dto_validation_empty_firstname() {
        let mut player = PlayerDto {
            id: "player/1".to_string(),
            firstname: "John".to_string(),
            handle: "john_doe".to_string(),
            email: "test@example.com".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        };
        player.firstname = "".to_string();
        let result = player.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("firstname"));
    }

    #[test]
    fn test_player_dto_validation_invalid_email() {
        let mut player = PlayerDto {
            id: "player/1".to_string(),
            firstname: "John".to_string(),
            handle: "john_doe".to_string(),
            email: "test@example.com".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            is_admin: false,
        };
        player.email = "invalid-email".to_string();
        let result = player.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("email"));
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
}
