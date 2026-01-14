use crate::api::api_url;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub name: String,
    pub build_date: Option<String>,
    pub git_commit: Option<String>,
    pub environment: String,
}

/// Get version information from the backend
pub async fn get_version_info() -> Result<VersionInfo, String> {
    let response = Request::get(&api_url("/api/version"))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch version info: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to get version info: HTTP {}",
            response.status()
        ));
    }

    response
        .json::<VersionInfo>()
        .await
        .map_err(|e| format!("Failed to parse version info: {}", e))
}
