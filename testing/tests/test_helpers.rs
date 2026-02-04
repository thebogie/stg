//! Shared test helpers for E2E API tests

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::env;

#[derive(Debug, Deserialize)]
struct LoginResponse {
    pub player: PlayerDto,
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
struct PlayerDto {
    #[serde(rename = "_id")]
    pub id: String,
    pub email: String,
    pub handle: String,
    pub firstname: String,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    #[serde(rename = "isAdmin")]
    pub is_admin: Option<bool>,
}

/// Helper to get an authenticated session for API tests
/// This registers a new user and logs in, returning the session_id
pub async fn get_authenticated_session(base_url: &str) -> Result<Option<String>> {
    let client = reqwest::Client::new();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let email = format!("e2e_test_{}@example.com", timestamp);
    let username = format!("e2e_test_{}", timestamp);

    // Try to register a new user
    let register_url = format!("{}/api/players/register", base_url);
    let register_res = client
        .post(&register_url)
        .json(&json!({
            "username": username,
            "email": email.clone(),
            "password": "password123"
        }))
        .send()
        .await
        .context("Failed to register")?;

    // If registration fails, try to login instead (user might already exist)
    if !register_res.status().is_success() {
        let login_url = format!("{}/api/players/login", base_url);
        let login_res = client
            .post(&login_url)
            .json(&json!({
                "email": email,
                "password": "password123"
            }))
            .send()
            .await
            .context("Failed to login")?;

        if login_res.status().is_success() {
            let login_body: LoginResponse =
                login_res.json().await.context("Failed to parse login")?;
            return Ok(Some(login_body.session_id));
        }
        return Ok(None);
    }

    // Login with newly registered user
    let login_url = format!("{}/api/players/login", base_url);
    let login_res = client
        .post(&login_url)
        .json(&json!({
            "email": email,
            "password": "password123"
        }))
        .send()
        .await
        .context("Failed to login")?;

    if login_res.status().is_success() {
        let login_body: LoginResponse = login_res.json().await.context("Failed to parse login")?;
        return Ok(Some(login_body.session_id));
    }

    Ok(None)
}
