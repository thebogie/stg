// Re-export all API modules
pub mod auth;
pub mod ai;
pub mod cache;
pub mod contests;
pub mod games;
pub mod players;
pub mod timezone;
pub mod utils;
pub mod venues;
pub mod version;

use crate::config;

pub fn api_url(path: &str) -> String {
    let base_url = config::api_base_url();
    if base_url.is_empty() {
        // Use relative URL
        path.to_string()
    } else {
        // Use absolute URL
        format!("{}{}", base_url, path)
    }
}
