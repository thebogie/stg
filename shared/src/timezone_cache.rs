use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

static TIMEZONE_CACHE: Lazy<Mutex<HashMap<String, Tz>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Get a timezone from cache or parse and cache it
pub fn get_cached_timezone(timezone_name: &str) -> Option<Tz> {
    // Check cache first
    if let Ok(cache) = TIMEZONE_CACHE.lock() {
        if let Some(tz) = cache.get(timezone_name) {
            return Some(*tz);
        }
    }

    // Parse and cache
    if let Ok(tz) = timezone_name.parse::<Tz>() {
        if let Ok(mut cache) = TIMEZONE_CACHE.lock() {
            cache.insert(timezone_name.to_string(), tz);
        }
        Some(tz)
    } else {
        None
    }
}

/// Convert UTC datetime to timezone with caching
pub fn convert_to_timezone_cached(
    utc_dt: DateTime<Utc>,
    timezone_name: &str,
) -> Option<DateTime<Tz>> {
    get_cached_timezone(timezone_name).map(|tz| utc_dt.with_timezone(&tz))
}

/// Clear the timezone cache (useful for testing)
pub fn clear_timezone_cache() {
    if let Ok(mut cache) = TIMEZONE_CACHE.lock() {
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timezone_caching() {
        clear_timezone_cache();

        // First call should parse and cache
        let tz1 = get_cached_timezone("America/Chicago");
        assert!(tz1.is_some());

        // Second call should use cache
        let tz2 = get_cached_timezone("America/Chicago");
        assert!(tz2.is_some());
        assert_eq!(tz1.unwrap(), tz2.unwrap());

        // Test conversion
        let utc_time = Utc::now();
        let converted = convert_to_timezone_cached(utc_time, "America/Chicago");
        assert!(converted.is_some());
    }
}
