use chrono::{DateTime, Utc};
use chrono_tz::Tz;

/// Convert a UTC datetime to a specific timezone
pub fn convert_to_timezone(utc_dt: DateTime<Utc>, timezone_name: &str) -> Option<DateTime<Tz>> {
    // Parse the timezone name
    let tz: Tz = timezone_name.parse().ok()?;
    
    // Convert UTC to the target timezone
    Some(utc_dt.with_timezone(&tz))
}

/// Get timezone abbreviation (e.g., "CST", "EST", "PST")
pub fn get_timezone_abbreviation(timezone_name: &str) -> String {
    match timezone_name {
        "America/Chicago" => "CST".to_string(),
        "America/New_York" => "EST".to_string(),
        "America/Los_Angeles" => "PST".to_string(),
        "America/Denver" => "MST".to_string(),
        "America/Phoenix" => "MST".to_string(),
        "America/Anchorage" => "AKST".to_string(),
        "Pacific/Honolulu" => "HST".to_string(),
        "Europe/London" => "GMT".to_string(),
        "Europe/Paris" => "CET".to_string(),
        "Europe/Berlin" => "CET".to_string(),
        "Asia/Tokyo" => "JST".to_string(),
        "Asia/Shanghai" => "CST".to_string(),
        "Asia/Kolkata" => "IST".to_string(),
        "Australia/Sydney" => "AEST".to_string(),
        "Australia/Perth" => "AWST".to_string(),
        "UTC" => "UTC".to_string(),
        _ => {
            // For unknown timezones, try to parse and get abbreviation
            if let Ok(tz) = timezone_name.parse::<Tz>() {
                // Get current time in that timezone to determine abbreviation
                let _now = Utc::now().with_timezone(&tz);
                // This is a simplified approach - in production you'd want more sophisticated abbreviation detection
                timezone_name.split('/').last().unwrap_or(timezone_name).to_string()
            } else {
                timezone_name.to_string()
            }
        }
    }
}

/// Format a datetime with timezone abbreviation
pub fn format_with_timezone(utc_dt: DateTime<Utc>, timezone_name: &str) -> String {
    if let Some(local_dt) = convert_to_timezone(utc_dt, timezone_name) {
        let abbreviation = get_timezone_abbreviation(timezone_name);
        format!("{} ({})", local_dt.format("%B %d, %Y at %I:%M %p"), abbreviation)
    } else {
        // Fallback to UTC if timezone conversion fails
        format!("{} (UTC)", utc_dt.format("%B %d, %Y at %I:%M %p"))
    }
}

/// Get timezone offset in hours for display
pub fn get_timezone_offset_hours(timezone_name: &str) -> Option<i32> {
    // For now, return a simple mapping - in production you'd want proper offset calculation
    match timezone_name {
        "America/Chicago" => Some(-6),
        "America/New_York" => Some(-5),
        "America/Los_Angeles" => Some(-8),
        "America/Denver" => Some(-7),
        "America/Phoenix" => Some(-7),
        "America/Anchorage" => Some(-9),
        "Pacific/Honolulu" => Some(-10),
        "Europe/London" => Some(0),
        "Europe/Paris" => Some(1),
        "Europe/Berlin" => Some(1),
        "Asia/Tokyo" => Some(9),
        "Asia/Shanghai" => Some(8),
        "Asia/Kolkata" => Some(5),
        "Australia/Sydney" => Some(10),
        "Australia/Perth" => Some(8),
        "UTC" => Some(0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Timelike};

    #[test]
    fn test_timezone_conversion() {
        let utc_time = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        
        // Test Chicago timezone
        let chicago_time = convert_to_timezone(utc_time, "America/Chicago").unwrap();
        assert_eq!(chicago_time.hour(), 6); // UTC-6
        
        // Test New York timezone
        let ny_time = convert_to_timezone(utc_time, "America/New_York").unwrap();
        assert_eq!(ny_time.hour(), 7); // UTC-5
    }

    #[test]
    fn test_timezone_abbreviations() {
        assert_eq!(get_timezone_abbreviation("America/Chicago"), "CST");
        assert_eq!(get_timezone_abbreviation("America/New_York"), "EST");
        assert_eq!(get_timezone_abbreviation("America/Los_Angeles"), "PST");
        assert_eq!(get_timezone_abbreviation("UTC"), "UTC");
    }

    #[test]
    fn test_format_with_timezone() {
        let utc_time = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        let formatted = format_with_timezone(utc_time, "America/Chicago");
        assert!(formatted.contains("CST"));
        assert!(formatted.contains("January 15, 2024"));
    }

    #[test]
    fn test_timezone_offset_hours() {
        assert_eq!(get_timezone_offset_hours("America/Chicago"), Some(-6));
        assert_eq!(get_timezone_offset_hours("America/New_York"), Some(-5));
        assert_eq!(get_timezone_offset_hours("America/Los_Angeles"), Some(-8));
        assert_eq!(get_timezone_offset_hours("Europe/London"), Some(0));
        assert_eq!(get_timezone_offset_hours("Asia/Tokyo"), Some(9));
        assert_eq!(get_timezone_offset_hours("UTC"), Some(0));
        assert_eq!(get_timezone_offset_hours("Invalid/Timezone"), None);
    }

    #[test]
    fn test_invalid_timezone_conversion() {
        let utc_time = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        let result = convert_to_timezone(utc_time, "Invalid/Timezone");
        assert!(result.is_none());
    }

    #[test]
    fn test_timezone_conversion_edge_cases() {
        let utc_time = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        
        // Test UTC conversion
        let utc_result = convert_to_timezone(utc_time, "UTC").unwrap();
        assert_eq!(utc_result.hour(), 12);
        
        // Test extreme timezone
        let tokyo_time = convert_to_timezone(utc_time, "Asia/Tokyo").unwrap();
        assert_eq!(tokyo_time.hour(), 21); // UTC+9
    }

    #[test]
    fn test_timezone_abbreviation_edge_cases() {
        // Test unknown timezone
        let unknown_abbrev = get_timezone_abbreviation("Unknown/Timezone");
        assert_eq!(unknown_abbrev, "Unknown/Timezone");
        
        // Test empty string
        let empty_abbrev = get_timezone_abbreviation("");
        assert_eq!(empty_abbrev, "");
    }
}
