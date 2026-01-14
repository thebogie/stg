use crate::api::api_url;
use gloo_net::http::Request;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TimezoneResolveResponse {
    pub timezone: String,
}

pub async fn resolve_timezone(lat: f64, lng: f64) -> Result<String, String> {
    let url = format!(
        "{}?lat={}&lng={}",
        api_url("/api/timezone/resolve"),
        lat,
        lng
    );
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;
    if !resp.ok() {
        return Err(format!("Server error: {}", resp.status()));
    }
    let data: TimezoneResolveResponse = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;
    Ok(data.timezone)
}

pub async fn resolve_timezone_by_place_id(place_id: &str) -> Result<String, String> {
    let url = format!("{}?place_id={}", api_url("/api/timezone/resolve"), place_id);
    log::info!("Frontend: Calling timezone API with URL: {}", url);
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;
    if !resp.ok() {
        return Err(format!("Server error: {}", resp.status()));
    }
    let data: TimezoneResolveResponse = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;
    log::info!("Frontend: Received timezone response: {}", data.timezone);
    Ok(data.timezone)
}
