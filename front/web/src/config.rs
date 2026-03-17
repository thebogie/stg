//! Frontend config: API base URL and Tauri detection.
//! When running in Tauri, config is loaded once via invoke('get_app_config').
//! In the browser, config is derived from window.origin.

use crate::tauri::{self, AppConfig};
use std::cell::RefCell;

thread_local! {
    static APP_CONFIG: RefCell<Option<AppConfig>> = RefCell::new(None);
}

/// Set app config (called once at startup by ConfigLoader). When running in Tauri this comes from invoke.
pub fn set_app_config(config: AppConfig) {
    APP_CONFIG.with(|c| *c.borrow_mut() = Some(config));
}

/// Get the current app config if set.
pub fn get_app_config() -> Option<AppConfig> {
    APP_CONFIG.with(|c| c.borrow().clone())
}

/// Base URL for API requests.
///
/// When config has been set (Tauri or after ConfigLoader ran), uses that.
/// In Tauri dev (page from localhost), we use "" so requests stay same-origin and Trunk's proxy
/// forwards to the backend — avoids webview blocking cross-origin to 127.0.0.1:50002.
/// Otherwise (e.g. during first paint in browser) uses origin-based heuristic.
pub fn api_base_url() -> String {
    if let Some(ref c) = get_app_config() {
        if c.is_tauri && current_origin_is_localhost() {
            return String::new();
        }
        return c.api_base_url.clone();
    }
    fallback_api_base_url()
}

/// True when the current page origin is localhost or 127.0.0.1 (dev server).
fn current_origin_is_localhost() -> bool {
    let win = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };
    let origin = match win.location().origin() {
        Ok(o) => o,
        Err(_) => return false,
    };
    origin.starts_with("http://localhost:") || origin.starts_with("http://127.0.0.1:")
}

/// Initialize config from browser origin. Call when not running in Tauri.
pub fn init_browser_config() {
    set_app_config(AppConfig {
        api_base_url: fallback_api_base_url(),
        is_tauri: false,
    });
}

/// Origin-based fallback when global config is not set yet (browser only).
fn fallback_api_base_url() -> String {
    let win = match web_sys::window() {
        Some(w) => w,
        None => return String::new(),
    };
    let origin = match win.location().origin() {
        Ok(o) => o,
        Err(_) => return String::new(),
    };
    // In local dev (browser), prefer same-origin relative URLs so the dev server proxy can route
    // `/api/*` to the backend without triggering browser CORS.
    if origin.starts_with("http://localhost:") || origin.starts_with("http://127.0.0.1:") {
        String::new()
    } else {
        String::new()
    }
}

/// Returns true when the app is running inside the Tauri desktop shell.
pub fn is_tauri() -> bool {
    tauri::is_tauri()
}
