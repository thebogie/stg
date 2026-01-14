use crate::error::Result;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
// Provide a wrapper for the custom validator to be used in attribute
pub fn validate_place_id_optional(val: &str) -> std::result::Result<(), ValidationError> {
    if val.is_empty() {
        return Ok(());
    }
    if PLACE_ID_REGEX.is_match(val) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_place_id"))
    }
}

lazy_static! {
    static ref PLACE_ID_REGEX: Regex = Regex::new(r"^[A-Za-z0-9_-]+$").unwrap();
}

/// Default timezone for venues
fn default_timezone() -> String {
    "UTC".to_string()
}

/// Represents the source of venue data
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum VenueSource {
    #[serde(rename = "database")]
    Database,
    #[serde(rename = "google")]
    Google,
}

/// Represents a venue in the system
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Venue {
    /// Venue's ID
    #[serde(rename = "_id")]
    pub id: String,

    /// Venue's revision
    #[serde(rename = "_rev")]
    pub rev: String,

    /// Venue's display name
    #[validate(length(
        min = 1,
        max = 100,
        message = "Display name is required and must be at most 100 characters"
    ))]
    #[serde(rename = "displayName")]
    pub display_name: String,

    /// Venue's formatted address
    #[validate(length(
        min = 1,
        max = 200,
        message = "Formatted address is required and must be at most 200 characters"
    ))]
    #[serde(rename = "formattedAddress")]
    pub formatted_address: String,

    /// Google Places API place ID (required). Must match regex and length.
    #[validate(length(
        min = 1,
        max = 128,
        message = "Place ID is required and must be at most 128 characters"
    ))]
    #[validate(custom = "validate_place_id_optional")]
    pub place_id: String,

    /// Venue's latitude
    #[validate(range(min = -90.0, max = 90.0, message = "Latitude must be between -90 and 90"))]
    pub lat: f64,

    /// Venue's longitude
    #[validate(range(min = -180.0, max = 180.0, message = "Longitude must be between -180 and 180"))]
    pub lng: f64,

    /// Venue's timezone (IANA timezone name, e.g., "America/Chicago")
    #[serde(default = "default_timezone")]
    pub timezone: String,

    /// Source of the venue data
    pub source: VenueSource,
}

impl Venue {
    /// Custom validator: allow empty place_id; when non-empty, enforce regex
    fn validate_place_id_optional_str(value: &str) -> std::result::Result<(), ValidationError> {
        if value.is_empty() {
            return Ok(());
        }
        if PLACE_ID_REGEX.is_match(value) {
            Ok(())
        } else {
            Err(ValidationError::new("invalid_place_id"))
        }
    }
    /// Creates a new venue with validation
    pub fn new(
        id: String,
        rev: String,
        display_name: String,
        formatted_address: String,
        place_id: String,
        lat: f64,
        lng: f64,
        timezone: String,
        source: VenueSource,
    ) -> Result<Self> {
        let venue = Self {
            id,
            rev,
            display_name,
            formatted_address,
            place_id,
            lat,
            lng,
            timezone,
            source,
        };
        venue.validate()?;
        Ok(venue)
    }

    /// Creates a new venue for database insertion (ArangoDB will set id and rev)
    pub fn new_for_db(
        display_name: String,
        formatted_address: String,
        place_id: String,
        lat: f64,
        lng: f64,
        timezone: String,
        source: VenueSource,
    ) -> Result<Self> {
        let venue = Self {
            id: String::new(),  // Will be set by ArangoDB
            rev: String::new(), // Will be set by ArangoDB
            display_name,
            formatted_address,
            place_id,
            lat,
            lng,
            timezone,
            source,
        };
        venue.validate()?;
        Ok(venue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use test_log::test;
    use validator::Validate;

    fn create_test_venue() -> Venue {
        Venue {
            id: "venue/1".to_string(),
            rev: "1".to_string(),
            display_name: "Test Venue".to_string(),
            formatted_address: "123 Test St, Test City, TC 12345".to_string(),
            place_id: "test_place_id_123".to_string(),
            lat: 40.7128,
            lng: -74.0060,
            timezone: "America/New_York".to_string(),
            source: VenueSource::Database,
        }
    }

    #[test]
    fn test_venue_creation() {
        let venue = create_test_venue();
        assert_eq!(venue.display_name, "Test Venue");
        assert_eq!(venue.formatted_address, "123 Test St, Test City, TC 12345");
        assert_eq!(venue.place_id, "test_place_id_123");
        assert_eq!(venue.lat, 40.7128);
        assert_eq!(venue.lng, -74.0060);
        assert_eq!(venue.timezone, "America/New_York");
        assert_eq!(venue.source, VenueSource::Database);
    }

    #[test]
    fn test_venue_validation_success() {
        let venue = create_test_venue();
        assert!(venue.validate().is_ok());
    }

    #[test]
    fn test_venue_validation_empty_display_name() {
        let mut venue = create_test_venue();
        venue.display_name = "".to_string();
        let result = venue.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("displayName"));
    }

    #[test]
    fn test_venue_validation_empty_formatted_address() {
        let mut venue = create_test_venue();
        venue.formatted_address = "".to_string();
        let result = venue.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("formattedAddress"));
    }

    #[test]
    fn test_venue_validation_empty_place_id() {
        let mut venue = create_test_venue();
        venue.place_id = "".to_string();
        let result = venue.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("place_id"));
    }

    #[test]
    fn test_venue_validation_latitude_range() {
        let mut venue = create_test_venue();
        venue.lat = 91.0; // Invalid latitude
        let result = venue.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("lat"));

        venue.lat = -91.0; // Invalid latitude
        let result = venue.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("lat"));

        venue.lat = 90.0; // Valid latitude
        assert!(venue.validate().is_ok());

        venue.lat = -90.0; // Valid latitude
        assert!(venue.validate().is_ok());
    }

    #[test]
    fn test_venue_validation_longitude_range() {
        let mut venue = create_test_venue();
        venue.lng = 181.0; // Invalid longitude
        let result = venue.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("lng"));

        venue.lng = -181.0; // Invalid longitude
        let result = venue.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("lng"));

        venue.lng = 180.0; // Valid longitude
        assert!(venue.validate().is_ok());

        venue.lng = -180.0; // Valid longitude
        assert!(venue.validate().is_ok());
    }

    #[test]
    fn test_venue_serialization() {
        let venue = create_test_venue();
        let json = serde_json::to_string(&venue).unwrap();
        let deserialized: Venue = serde_json::from_str(&json).unwrap();
        assert_eq!(venue.id, deserialized.id);
        assert_eq!(venue.display_name, deserialized.display_name);
        assert_eq!(venue.formatted_address, deserialized.formatted_address);
        assert_eq!(venue.place_id, deserialized.place_id);
        assert_eq!(venue.lat, deserialized.lat);
        assert_eq!(venue.lng, deserialized.lng);
        assert_eq!(venue.timezone, deserialized.timezone);
        assert_eq!(venue.source, deserialized.source);
    }

    #[test]
    fn test_venue_id_format() {
        let venue = create_test_venue();
        assert!(venue.id.starts_with("venue/"));
    }

    #[test]
    fn test_venue_rev_format() {
        let venue = create_test_venue();
        assert!(venue.rev.parse::<i32>().is_ok());
    }

    #[test]
    fn test_venue_coordinates_precision() {
        let mut venue = create_test_venue();
        venue.lat = 40.7128;
        venue.lng = -74.0060;
        assert!(venue.validate().is_ok());

        // Test with more decimal places
        venue.lat = 40.7128123;
        venue.lng = -74.0060123;
        assert!(venue.validate().is_ok());
    }

    #[test]
    fn test_venue_display_name_length() {
        let mut venue = create_test_venue();
        venue.display_name = "A".repeat(100);
        assert!(venue.validate().is_ok());

        venue.display_name = "A".repeat(101);
        let result = venue.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("displayName"));
    }

    #[test]
    fn test_venue_formatted_address_length() {
        let mut venue = create_test_venue();
        venue.formatted_address = "A".repeat(200);
        assert!(venue.validate().is_ok());

        venue.formatted_address = "A".repeat(201);
        let result = venue.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("formattedAddress"));
    }

    #[test]
    fn test_venue_place_id_format() {
        let mut venue = create_test_venue();
        venue.place_id = "ChIJN1t_tDeuEmsRUsoyG83frY4".to_string();
        assert!(venue.validate().is_ok());

        venue.place_id = "invalid place id with spaces".to_string();
        let result = venue.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("place_id"));
    }

    #[test]
    fn test_venue_with_special_characters() {
        let mut venue = create_test_venue();
        venue.display_name = "Test Venue & Bar (Downtown)".to_string();
        venue.formatted_address = "123 Test St, Suite #100, Test City, TC 12345".to_string();
        assert!(venue.validate().is_ok());
    }

    #[test]
    fn test_venue_coordinates_edge_cases() {
        let mut venue = create_test_venue();

        // Test exact boundary values
        venue.lat = 0.0;
        venue.lng = 0.0;
        assert!(venue.validate().is_ok());

        venue.lat = 90.0;
        venue.lng = 180.0;
        assert!(venue.validate().is_ok());

        venue.lat = -90.0;
        venue.lng = -180.0;
        assert!(venue.validate().is_ok());
    }
}
