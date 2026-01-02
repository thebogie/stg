// Re-export all API modules
pub mod auth;
pub mod cache;
pub mod contests;
pub mod games;
pub mod players;
pub mod venues;
pub mod version;
pub mod utils;
pub mod timezone;

use crate::config::Config;

pub fn api_url(path: &str) -> String {
    let base_url = Config::api_base_url();
    if base_url.is_empty() {
        // Use relative URL
        path.to_string()
    } else {
        // Use absolute URL
        format!("{}{}", base_url, path)
    }
}