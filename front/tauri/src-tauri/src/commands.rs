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

fn default_api_base_url() -> String {
    if let Ok(url) = std::env::var("STG_API_URL") {
        let trimmed = url.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    #[cfg(debug_assertions)]
    {
        return "http://127.0.0.1:50002".to_string();
    }
    #[cfg(not(debug_assertions))]
    {
        option_env!("STG_API_URL")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("https://smacktalkgaming.com")
            .to_string()
    }
}

/// Returns app config for the frontend. Called once on load when running inside Tauri.
/// API URL: `STG_API_URL` env, else debug `http://127.0.0.1:50002`, else release `https://smacktalkgaming.com`.
#[tauri::command]
pub fn get_app_config() -> AppConfig {
    AppConfig {
        api_base_url: default_api_base_url(),
        is_tauri: true,
    }
}
