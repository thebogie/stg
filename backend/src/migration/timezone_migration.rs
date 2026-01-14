use arangors::Database;
use log;

/// Migration to add timezone field to existing venues
pub async fn migrate_venues_to_timezone(
    db: &Database<arangors::client::reqwest::ReqwestClient>,
) -> Result<(), String> {
    log::info!("🔄 Starting timezone migration for venues...");

    // Get all venues that don't have timezone set
    let query = arangors::AqlQuery::builder()
        .query(
            r#"
            FOR venue IN venue
            FILTER venue.timezone == null || venue.timezone == ""
            RETURN venue
        "#,
        )
        .build();

    match db.aql_query::<serde_json::Value>(query).await {
        Ok(mut cursor) => {
            let mut updated_count = 0;
            let mut error_count = 0;

            while let Some(venue_data) = cursor.pop() {
                if let Some(venue_id) = venue_data["_id"].as_str() {
                    // Try to infer timezone from location data
                    let timezone = infer_timezone_from_location(&venue_data);

                    // Update the venue with timezone
                    let update_query = arangors::AqlQuery::builder()
                        .query(
                            r#"
                            UPDATE @venue_id WITH { timezone: @timezone } IN venue
                            RETURN NEW
                        "#,
                        )
                        .bind_var("venue_id", venue_id)
                        .bind_var("timezone", timezone.clone())
                        .build();

                    match db.aql_query::<serde_json::Value>(update_query).await {
                        Ok(_) => {
                            log::info!("✅ Updated venue {} with timezone: {}", venue_id, timezone);
                            updated_count += 1;
                        }
                        Err(e) => {
                            log::error!("❌ Failed to update venue {}: {}", venue_id, e);
                            error_count += 1;
                        }
                    }
                }
            }

            log::info!(
                "🎉 Migration completed: {} updated, {} errors",
                updated_count,
                error_count
            );
            Ok(())
        }
        Err(e) => {
            log::error!("❌ Failed to fetch venues for migration: {}", e);
            Err(format!("Failed to fetch venues: {}", e))
        }
    }
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
        lng if lng >= -180.0 && lng < -120.0 => "America/Los_Angeles".to_string(), // Pacific
        lng if lng >= -120.0 && lng < -90.0 => "America/Denver".to_string(),       // Mountain
        lng if lng >= -90.0 && lng < -60.0 => "America/Chicago".to_string(),       // Central
        lng if lng >= -60.0 && lng < -30.0 => "America/New_York".to_string(),      // Eastern
        lng if lng >= -30.0 && lng < 0.0 => "Europe/London".to_string(),           // GMT
        lng if lng >= 0.0 && lng < 30.0 => "Europe/Paris".to_string(),             // CET
        lng if lng >= 30.0 && lng < 60.0 => "Europe/Berlin".to_string(),           // CET
        lng if lng >= 60.0 && lng < 90.0 => "Asia/Kolkata".to_string(),            // IST
        lng if lng >= 90.0 && lng < 120.0 => "Asia/Shanghai".to_string(),          // CST
        lng if lng >= 120.0 && lng < 150.0 => "Asia/Tokyo".to_string(),            // JST
        lng if lng >= 150.0 && lng < 180.0 => "Australia/Sydney".to_string(),      // AEST
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
