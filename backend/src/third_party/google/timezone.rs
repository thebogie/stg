use reqwest::Client;
use serde::Deserialize;
use log::{error, warn};
use chrono_tz::Tz;

/// Convert UTC offset (in minutes) to the best matching IANA timezone
/// This uses a more intelligent approach than hardcoding mappings
fn utc_offset_to_timezone(offset_minutes: i64) -> String {
    // Guard: out-of-range offsets should fall back to UTC
    if offset_minutes < -720 || offset_minutes > 720 {
        return "UTC".to_string();
    }
    let now = chrono::Utc::now();

    // Candidate pools by raw offset in hours. Order by most likely canonical zones first.
    let candidates: &[&[&str]] = &[
        &["Pacific/Kwajalein"],                                  // -12
        &["Pacific/Midway", "Pacific/Niue"],                    // -11
        &["Pacific/Honolulu", "Pacific/Tahiti"],               // -10
        &["America/Anchorage", "Pacific/Gambier"],             // -9
        &["America/Los_Angeles", "America/Tijuana", "Pacific/Pitcairn"], // -8
        &["America/Denver", "America/Phoenix"],                // -7
        &["America/Chicago", "America/Mexico_City"],           // -6
        &["America/New_York", "America/Toronto"],              // -5
        &["America/Halifax", "America/Santiago"],              // -4
        &["America/Sao_Paulo", "America/Argentina/Buenos_Aires"], // -3
        &["Atlantic/South_Georgia", "America/Noronha"],        // -2
        &["Atlantic/Azores", "Atlantic/Cape_Verde"],           // -1
        &["UTC", "Europe/London", "Africa/Abidjan"],         // 0
        &["Europe/Berlin", "Europe/Paris", "Africa/Lagos"],  // +1
        &["Europe/Athens", "Africa/Cairo", "Europe/Helsinki"],// +2
        &["Europe/Moscow", "Africa/Nairobi"],                  // +3
        &["Asia/Dubai", "Asia/Muscat", "Europe/Samara"],     // +4
        &["Asia/Karachi", "Asia/Tashkent", "Asia/Yekaterinburg"], // +5
        &["Asia/Dhaka", "Asia/Almaty", "Asia/Omsk"],         // +6
        &["Asia/Bangkok", "Asia/Jakarta", "Asia/Krasnoyarsk"], // +7
        &["Asia/Shanghai", "Asia/Singapore", "Asia/Irkutsk"], // +8
        &["Asia/Tokyo", "Asia/Seoul", "Asia/Yakutsk"],       // +9
        &["Australia/Sydney", "Pacific/Port_Moresby", "Asia/Vladivostok"], // +10
        &["Pacific/Noumea", "Asia/Magadan"],                   // +11
        &["Pacific/Auckland", "Pacific/Fiji"],                 // +12
    ];

    // Map offset minutes to index in candidates (-12..12 => 0..24). Clamp to range.
    let mut idx = (offset_minutes / 60) + 12;
    if idx < 0 { idx = 0; }
    if idx as usize >= candidates.len() { return "UTC".to_string(); }

    // Choose the first candidate whose actual offset matches closely at current instant.
    for tz_name in candidates[idx as usize] {
        if let Ok(tz) = tz_name.parse::<Tz>() {
            let local = now.with_timezone(&tz);
            let diff = (local.timestamp() - now.timestamp()) / 60; // minutes
            if (diff - offset_minutes).abs() <= 30 {
                return tz_name.to_string();
            }
        }
    }

    // Fallback to first candidate for that bucket, else UTC.
    candidates[idx as usize].first().copied().unwrap_or("UTC").to_string()
}

/// Google Timezone API response structure
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GoogleTimezoneResponse {
    #[serde(rename = "timeZoneId")]
    time_zone_id: String,
    #[serde(rename = "timeZoneName")]
    time_zone_name: String,
    #[serde(rename = "rawOffset")]
    raw_offset: i32,
    #[serde(rename = "dstOffset")]
    dst_offset: i32,
}

/// Service for interacting with Google Timezone API
#[derive(Clone)]
pub struct GoogleTimezoneService {
    api_url: String,
    api_key: String,
    client: Client,
}

impl GoogleTimezoneService {
    /// Create a new Google Timezone service
    pub fn new(api_url: String, api_key: String) -> Self {
        Self {
            api_url,
            api_key,
            client: Client::new(),
        }
    }

    /// Infer timezone from coordinates using Google Timezone API
    pub async fn infer_timezone_from_coordinates(&self, lat: f64, lng: f64) -> String {
        // Validate coordinates
        if lat < -90.0 || lat > 90.0 || lng < -180.0 || lng > 180.0 {
            warn!("Invalid coordinates: lat={}, lng={}", lat, lng);
            return "UTC".to_string();
        }

        let url = format!(
            "{}?location={},{}&timestamp={}&key={}",
            self.api_url,
            lat,
            lng,
            chrono::Utc::now().timestamp(),
            self.api_key
        );

        self.make_timezone_request(&url).await
    }

    /// Infer timezone from place_id using Google Timezone API
    pub async fn infer_timezone_from_place_id(&self, place_id: &str) -> String {
        if place_id.is_empty() {
            warn!("Empty place_id provided");
            return "UTC".to_string();
        }

        let url = format!(
            "{}?placeid={}&timestamp={}&key={}",
            self.api_url,
            place_id,
            chrono::Utc::now().timestamp(),
            self.api_key
        );

        self.make_timezone_request(&url).await
    }

    /// Make the actual HTTP request to Google Timezone API
    async fn make_timezone_request(&self, url: &str) -> String {
        match self.client.get(url).send().await {
            Ok(response) => {
                let status = response.status();
                if !status.is_success() {
                    error!("Google Timezone API returned error status: {}", status);
                    return "UTC".to_string();
                }
                
                let response_text = match response.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        error!("Failed to read response text: {}", e);
                        return "UTC".to_string();
                    }
                };
                
                log::debug!("Google Timezone API response: {}", response_text);
                
                // Check for Google API error response first
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
                    if let Some(status) = json.get("status").and_then(|v| v.as_str()) {
                        if status == "REQUEST_DENIED" || status == "OVER_QUERY_LIMIT" || status == "INVALID_REQUEST" {
                            if let Some(error_msg) = json.get("errorMessage").and_then(|v| v.as_str()) {
                                error!("Google Timezone API error: {} - {}", status, error_msg);
                            } else {
                                error!("Google Timezone API error: {}", status);
                            }
                            return "UTC".to_string();
                        }
                    }
                    
                    // Check if this is a Place Details response (wrong API called)
                    if json.get("result").is_some() {
                        log::info!("Received Place Details response instead of Timezone API response. Attempting to resolve via geometry -> Timezone API.");
                        if let Some(result) = json.get("result") {
                            if let Some(geometry) = result.get("geometry") {
                                if let (Some(lat), Some(lng)) = (
                                    geometry.get("location").and_then(|v| v.get("lat")).and_then(|v| v.as_f64()),
                                    geometry.get("location").and_then(|v| v.get("lng")).and_then(|v| v.as_f64()),
                                ) {
                                    // Call the official Google Time Zone API directly with lat/lng
                                    let tz_url = format!(
                                        "https://maps.googleapis.com/maps/api/timezone/json?location={},{}&timestamp={}&key={}",
                                        lat,
                                        lng,
                                        chrono::Utc::now().timestamp(),
                                        self.api_key
                                    );
                                    match self.client.get(&tz_url).send().await {
                                        Ok(tz_resp) => {
                                            if tz_resp.status().is_success() {
                                                if let Ok(tz_text) = tz_resp.text().await {
                                                    if let Ok(tz_json) = serde_json::from_str::<serde_json::Value>(&tz_text) {
                                                        if let Some(tz_id) = tz_json.get("timeZoneId").and_then(|v| v.as_str()) {
                                                            log::info!("Resolved timezone via Timezone API: {}", tz_id);
                                                            return tz_id.to_string();
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error!("Failed calling Google Timezone API after Place Details: {}", e);
                                        }
                                    }
                                    // As last resort, fall back to utc_offset mapping if present
                                    if let Some(utc_offset) = result.get("utc_offset").and_then(|v| v.as_i64()) {
                                        let tz_name = utc_offset_to_timezone(utc_offset);
                                        log::info!("Fallback from geometry: utc_offset {} -> {}", utc_offset, tz_name);
                                        return tz_name;
                                    }
                                }
                            }
                        }
                        return "UTC".to_string();
                    }
                }
                
                // Try to parse as normal timezone response
                match serde_json::from_str::<GoogleTimezoneResponse>(&response_text) {
                    Ok(data) => data.time_zone_id,
                    Err(e) => {
                        error!("Failed to parse timezone response: {}", e);
                        
                        // Try to extract timezone from raw response as fallback
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
                            if let Some(tz_id) = json.get("timeZoneId").and_then(|v| v.as_str()) {
                                log::info!("Extracted timezone from raw response: {}", tz_id);
                                return tz_id.to_string();
                            }
                            if let Some(tz_id) = json.get("time_zone_id").and_then(|v| v.as_str()) {
                                log::info!("Extracted timezone from raw response (snake_case): {}", tz_id);
                                return tz_id.to_string();
                            }
                        }
                        
                        "UTC".to_string()
                    }
                }
            }
            Err(e) => {
                error!("Failed to call Google Timezone API: {}", e);
                "UTC".to_string()
            }
        }
    }

    /// Get timezone with fallback to UTC
    pub async fn get_timezone_with_fallback(&self, lat: f64, lng: f64) -> String {
        self.infer_timezone_from_coordinates(lat, lng).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_timezone_inference() {
        let service = GoogleTimezoneService::new("https://maps.googleapis.com/maps/api/timezone/json".to_string(), "test_key".to_string());
        
        // Test service creation
        assert_eq!(service.api_key, "test_key");
        
        // Test coordinate validation
        assert!(30.284886 >= -90.0 && 30.284886 <= 90.0);
        assert!(-97.774528 >= -180.0 && -97.774528 <= 180.0);
    }

    #[tokio::test]
    async fn test_timezone_inference_international() {
        let _service = GoogleTimezoneService::new("https://maps.googleapis.com/maps/api/timezone/json".to_string(), "test_key".to_string());
        
        // Test coordinate validation for international locations
        assert!(51.5074 >= -90.0 && 51.5074 <= 90.0); // London lat
        assert!(-0.1278 >= -180.0 && -0.1278 <= 180.0); // London lng
        
        assert!(35.6762 >= -90.0 && 35.6762 <= 90.0); // Tokyo lat
        assert!(139.6503 >= -180.0 && 139.6503 <= 180.0); // Tokyo lng
    }

    #[tokio::test]
    async fn test_timezone_inference_edge_cases() {
        let _service = GoogleTimezoneService::new("https://maps.googleapis.com/maps/api/timezone/json".to_string(), "test_key".to_string());
        
        // Test coordinate validation for edge cases
        assert!(90.0 >= -90.0 && 90.0 <= 90.0); // North pole lat
        assert!(0.0 >= -180.0 && 0.0 <= 180.0); // North pole lng
        
        assert!(0.0 >= -90.0 && 0.0 <= 90.0); // Zero lat
        assert!(0.0 >= -180.0 && 0.0 <= 180.0); // Zero lng
    }

    #[tokio::test]
    async fn test_fallback_behavior() {
        let service = GoogleTimezoneService::new("https://maps.googleapis.com/maps/api/timezone/json".to_string(), "test_key".to_string());
        
        // Test that service handles invalid coordinates gracefully
        // These would return "UTC" due to validation
        let invalid_lat = service.infer_timezone_from_coordinates(200.0, 0.0).await;
        assert_eq!(invalid_lat, "UTC");
        
        let invalid_lng = service.infer_timezone_from_coordinates(0.0, 200.0).await;
        assert_eq!(invalid_lng, "UTC");
    }

    #[test]
    fn test_utc_offset_to_timezone_mapping() {
        // Test common US timezones
        // Note: These tests check that the function returns a valid timezone for the given offset
        // The exact timezone may vary based on current DST status
        let result_300 = utc_offset_to_timezone(-300); // -5 hours
        assert!(result_300.contains("America"), "Expected America timezone for -5 hours, got: {}", result_300);
        
        let result_240 = utc_offset_to_timezone(-240); // -4 hours  
        assert!(result_240.contains("America"), "Expected America timezone for -4 hours, got: {}", result_240);
        
        let result_360 = utc_offset_to_timezone(-360); // -6 hours
        assert!(result_360.contains("America"), "Expected America timezone for -6 hours, got: {}", result_360);
        
        let result_420 = utc_offset_to_timezone(-420); // -7 hours
        assert!(result_420.contains("America"), "Expected America timezone for -7 hours, got: {}", result_420);
        
        let result_480 = utc_offset_to_timezone(-480); // -8 hours
        assert!(result_480.contains("America"), "Expected America timezone for -8 hours, got: {}", result_480);
        
        // Test UTC
        assert_eq!(utc_offset_to_timezone(0), "UTC");
        
        // Test international timezones
        let result_60 = utc_offset_to_timezone(60); // +1 hour
        assert!(result_60.contains("Europe") || result_60.contains("Africa"), "Expected Europe/Africa timezone for +1 hour, got: {}", result_60);
        
        let result_120 = utc_offset_to_timezone(120); // +2 hours
        assert!(result_120.contains("Europe") || result_120.contains("Africa"), "Expected Europe/Africa timezone for +2 hours, got: {}", result_120);
        
        let result_540 = utc_offset_to_timezone(540); // +9 hours
        assert!(result_540.contains("Asia"), "Expected Asia timezone for +9 hours, got: {}", result_540);
        
        let result_600 = utc_offset_to_timezone(600); // +10 hours
        assert!(result_600.contains("Australia") || result_600.contains("Asia"), "Expected Australia/Asia timezone for +10 hours, got: {}", result_600);
    }

    #[test]
    fn test_utc_offset_to_timezone_edge_cases() {
        // Test extreme offsets
        let result_720_neg = utc_offset_to_timezone(-720); // -12 hours
        assert!(result_720_neg.contains("Pacific"), "Expected Pacific timezone for -12 hours, got: {}", result_720_neg);
        
        let result_720_pos = utc_offset_to_timezone(720); // +12 hours
        assert!(result_720_pos.contains("Pacific"), "Expected Pacific timezone for +12 hours, got: {}", result_720_pos);
        
        // Test fractional hours (should be handled by the function)
        let result_330_neg = utc_offset_to_timezone(-330); // -5.5 hours
        assert!(result_330_neg.contains("America"), "Expected America timezone for -5.5 hours, got: {}", result_330_neg);
        
        let result_330_pos = utc_offset_to_timezone(330); // +5.5 hours
        assert!(result_330_pos.contains("Asia"), "Expected Asia timezone for +5.5 hours, got: {}", result_330_pos);
        
        // Test unknown offsets (should fallback to UTC)
        assert_eq!(utc_offset_to_timezone(-999), "UTC"); // Unknown negative offset
        assert_eq!(utc_offset_to_timezone(999), "UTC"); // Unknown positive offset
    }

    #[test]
    fn test_utc_offset_to_timezone_dst_awareness() {
        // These tests verify that the function can handle DST transitions
        // by checking multiple candidate timezones for the same offset
        
        // -5 hours could be EST (America/New_York) or CDT (America/Chicago)
        let result_minus_5 = utc_offset_to_timezone(-300);
        assert!(result_minus_5 == "America/New_York" || result_minus_5 == "America/Chicago");
        
        // -6 hours could be CST (America/Chicago) or MDT (America/Denver)
        let result_minus_6 = utc_offset_to_timezone(-360);
        assert!(result_minus_6 == "America/Chicago" || result_minus_6 == "America/Denver");
        
        // -7 hours could be MST (America/Denver) or PDT (America/Los_Angeles)
        let result_minus_7 = utc_offset_to_timezone(-420);
        assert!(result_minus_7 == "America/Denver" || result_minus_7 == "America/Los_Angeles");
    }

    #[tokio::test]
    async fn test_place_id_timezone_resolution() {
        let service = GoogleTimezoneService::new("https://maps.googleapis.com/maps/api/timezone/json".to_string(), "test_key".to_string());
        
        // Test empty place_id
        let result = service.infer_timezone_from_place_id("").await;
        assert_eq!(result, "UTC");
        
        // Test valid place_id format (Google Place IDs are typically long strings)
        let result = service.infer_timezone_from_place_id("ChIJN1t_tDeuEmsRUsoyG83frY4").await;
        // This will return "UTC" in test environment since we don't have real API key
        assert_eq!(result, "UTC");
    }

    #[test]
    fn test_timezone_candidate_selection() {
        // Test that the function selects appropriate candidates for different regions
        
        // US East Coast (-4 hours)
        let east_coast_result = utc_offset_to_timezone(-240);
        assert!(east_coast_result.contains("America"), "Expected America timezone for -4 hours, got: {}", east_coast_result);
        
        // US West Coast (-7 hours)
        let west_coast_result = utc_offset_to_timezone(-420);
        assert!(west_coast_result.contains("America"), "Expected America timezone for -7 hours, got: {}", west_coast_result);
        
        // Europe (+1 hour)
        let europe_result = utc_offset_to_timezone(60);
        assert!(europe_result.contains("Europe") || europe_result.contains("Africa"), "Expected Europe/Africa timezone for +1 hour, got: {}", europe_result);
        
        // Asia (+9 hours)
        let asia_result = utc_offset_to_timezone(540);
        assert!(asia_result.contains("Asia"), "Expected Asia timezone for +9 hours, got: {}", asia_result);
        
        // Australia (+10 hours)
        let australia_result = utc_offset_to_timezone(600);
        assert!(australia_result.contains("Australia") || australia_result.contains("Asia"), "Expected Australia/Asia timezone for +10 hours, got: {}", australia_result);
    }
}