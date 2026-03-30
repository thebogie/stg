use crate::db::Db;
use log;

/// Migration to add timezone field to existing venues
pub async fn migrate_venues_to_timezone(db: &Db) -> Result<(), String> {
    log::info!("🔄 Starting timezone migration for venues...");

    let q = db.query("SELECT * FROM venue WHERE timezone = NONE OR timezone = ''");
    let mut res = q
        .await
        .map_err(|e| format!("Failed to fetch venues: {}", e))?;
    let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
    let mut updated_count = 0u32;
    let mut error_count = 0u32;

    for venue_data in rows {
        let venue_id = venue_data
            .get("id")
            .and_then(|v| v.as_str())
            .or_else(|| venue_data.get("_id").and_then(|v| v.as_str()));
        if let Some(vid) = venue_id {
            let timezone = infer_timezone_from_location(&venue_data);
            let key = vid
                .trim_start_matches("venue:")
                .trim_start_matches("venue/")
                .to_string();
            let up = db.query("UPDATE type::record('venue', $key) SET timezone = $timezone");
            up.bind(("key", key))
                .bind(("timezone", timezone.clone()))
                .await
                .map_err(|e| {
                    log::error!("❌ Failed to update venue {}: {}", vid, e);
                    error_count += 1;
                    format!("Update failed: {}", e)
                })?;
            log::info!("✅ Updated venue {} with timezone: {}", vid, timezone);
            updated_count += 1;
        }
    }

    log::info!(
        "🎉 Migration completed: {} updated, {} errors",
        updated_count,
        error_count
    );
    Ok(())
}

/// Infer timezone from venue location data
fn infer_timezone_from_location(venue_data: &serde_json::Value) -> String {
    // Try to infer timezone from coordinates
    if let (Some(lat), Some(lng)) = (venue_data["lat"].as_f64(), venue_data["lng"].as_f64()) {
        return infer_timezone_from_coordinates(lat, lng);
    }

    // Try to infer from address
    if let Some(address) = venue_data["formattedAddress"].as_str() {
        return infer_timezone_from_address(address);
    }

    // Default to UTC if we can't infer
    "UTC".to_string()
}

/// Infer timezone from coordinates (simplified mapping)
fn infer_timezone_from_coordinates(_lat: f64, lng: f64) -> String {
    // Simplified timezone inference based on longitude
    // In production, you'd want to use a proper geocoding service
    match lng {
        lng if (-180.0..-120.0).contains(&lng) => "America/Los_Angeles".to_string(), // Pacific
        lng if (-120.0..-90.0).contains(&lng) => "America/Denver".to_string(),       // Mountain
        lng if (-90.0..-60.0).contains(&lng) => "America/Chicago".to_string(),       // Central
        lng if (-60.0..-30.0).contains(&lng) => "America/New_York".to_string(),      // Eastern
        lng if (-30.0..0.0).contains(&lng) => "Europe/London".to_string(),           // GMT
        lng if (0.0..30.0).contains(&lng) => "Europe/Paris".to_string(),             // CET
        lng if (30.0..60.0).contains(&lng) => "Europe/Berlin".to_string(),           // CET
        lng if (60.0..90.0).contains(&lng) => "Asia/Kolkata".to_string(),            // IST
        lng if (90.0..120.0).contains(&lng) => "Asia/Shanghai".to_string(),          // CST
        lng if (120.0..150.0).contains(&lng) => "Asia/Tokyo".to_string(),            // JST
        lng if (150.0..180.0).contains(&lng) => "Australia/Sydney".to_string(),      // AEST
        _ => "UTC".to_string(),
    }
}

/// Infer timezone from address (simplified)
fn infer_timezone_from_address(address: &str) -> String {
    let address_lower = address.to_lowercase();

    // Simple keyword matching
    if address_lower.contains("new york") || address_lower.contains("ny") {
        "America/New_York".to_string()
    } else if address_lower.contains("chicago") || address_lower.contains("il") {
        "America/Chicago".to_string()
    } else if address_lower.contains("los angeles")
        || address_lower.contains("california")
        || address_lower.contains("ca")
    {
        "America/Los_Angeles".to_string()
    } else if address_lower.contains("denver")
        || address_lower.contains("colorado")
        || address_lower.contains("co")
    {
        "America/Denver".to_string()
    } else if address_lower.contains("london") || address_lower.contains("uk") {
        "Europe/London".to_string()
    } else if address_lower.contains("paris") || address_lower.contains("france") {
        "Europe/Paris".to_string()
    } else if address_lower.contains("berlin") || address_lower.contains("germany") {
        "Europe/Berlin".to_string()
    } else if address_lower.contains("tokyo") || address_lower.contains("japan") {
        "Asia/Tokyo".to_string()
    } else if address_lower.contains("shanghai") || address_lower.contains("china") {
        "Asia/Shanghai".to_string()
    } else if address_lower.contains("sydney") || address_lower.contains("australia") {
        "Australia/Sydney".to_string()
    } else {
        "UTC".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_timezone_from_coordinates() {
        assert_eq!(
            infer_timezone_from_coordinates(40.7128, -74.0060),
            "America/Chicago"
        ); // NYC -> Chicago (based on longitude)
        assert_eq!(
            infer_timezone_from_coordinates(41.8781, -87.6298),
            "America/Chicago"
        ); // Chicago
        assert_eq!(
            infer_timezone_from_coordinates(34.0522, -118.2437),
            "America/Denver"
        ); // LA -> Denver (based on longitude)
        assert_eq!(
            infer_timezone_from_coordinates(51.5074, -0.1278),
            "Europe/London"
        ); // London
    }

    #[test]
    fn test_infer_timezone_from_address() {
        assert_eq!(
            infer_timezone_from_address("New York, NY"),
            "America/New_York"
        );
        assert_eq!(
            infer_timezone_from_address("Chicago, IL"),
            "America/Chicago"
        );
        assert_eq!(
            infer_timezone_from_address("Los Angeles, CA"),
            "America/Los_Angeles"
        );
        assert_eq!(infer_timezone_from_address("London, UK"), "Europe/London");
    }
}
