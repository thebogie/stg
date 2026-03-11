//! Tauri commands for the STG desktop app. Invoked from the Yew frontend when running in Tauri.

use serde::Serialize;

/// Config returned to the frontend so it can talk to the backend (API base URL, etc.).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// Base URL for API requests (e.g. "http://127.0.0.1:50002"). Empty means use relative URLs.
    pub api_base_url: String,
    /// Always true when this command is used (frontend can also detect Tauri via window.__TAURI_INTERNALS__).
    pub is_tauri: bool,
}

/// Returns app config for the frontend. Called once on load when running inside Tauri.
/// API URL comes from env STG_API_URL or defaults to http://127.0.0.1:50002.
#[tauri::command]
pub fn get_app_config() -> AppConfig {
    let api_base_url = std::env::var("STG_API_URL").unwrap_or_else(|_| "http://127.0.0.1:50002".to_string());
    AppConfig {
        api_base_url,
        is_tauri: true,
    }
}
