use crate::models::venue::{Venue, VenueSource};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Data Transfer Object for Venue
#[derive(Debug, Serialize, Deserialize, Validate, Clone, PartialEq)]
pub struct VenueDto {
    /// Venue's ID (optional for creation, will be set by ArangoDB if empty)
    #[serde(rename = "_id", default)]
    pub id: String,
    #[validate(length(
        min = 1,
        max = 100,
        message = "Display name is required and must be at most 100 characters"
    ))]
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[validate(length(
        min = 1,
        max = 200,
        message = "Formatted address is required and must be at most 200 characters"
    ))]
    #[serde(rename = "formattedAddress")]
    pub formatted_address: String,
    // Place ID is required and must meet length constraints
    #[validate(length(
        min = 1,
        max = 128,
        message = "Place ID is required and must be at most 128 characters"
    ))]
    pub place_id: String,
    #[validate(range(min = -90.0, max = 90.0, message = "Latitude must be between -90 and 90"))]
    pub lat: f64,
    #[validate(range(min = -180.0, max = 180.0, message = "Longitude must be between -180 and 180"))]
    pub lng: f64,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default = "default_venue_source")]
    pub source: VenueSource,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_venue_source() -> VenueSource {
    VenueSource::Database
}

impl From<&Venue> for VenueDto {
    fn from(venue: &Venue) -> Self {
        Self {
            id: venue.id.clone(),
            display_name: venue.display_name.clone(),
            formatted_address: venue.formatted_address.clone(),
            place_id: venue.place_id.clone(),
            lat: venue.lat,
            lng: venue.lng,
            timezone: venue.timezone.clone(),
            source: venue.source.clone(),
        }
    }
}

impl From<VenueDto> for Venue {
    fn from(dto: VenueDto) -> Self {
        // Always preserve the ID from the DTO, even if new_for_db succeeds
        let mut venue = Self::new_for_db(
            dto.display_name.clone(),
            dto.formatted_address.clone(),
            dto.place_id.clone(),
            dto.lat,
            dto.lng,
            dto.timezone.clone(),
            dto.source,
        )
        .unwrap_or_else(|_| Self {
            id: String::new(),  // Will be overridden below
            rev: String::new(), // Let ArangoDB set this
            display_name: dto.display_name,
            formatted_address: dto.formatted_address,
            place_id: dto.place_id,
            lat: dto.lat,
            lng: dto.lng,
            timezone: dto.timezone,
            source: dto.source,
        });

        // Always use the ID from the DTO
        venue.id = dto.id;
        venue
    }
}

impl VenueDto {
    /// Updates the Venue with data from a VenueDto
    pub fn update_venue(&self, venue: &mut Venue) {
        venue.id = self.id.clone();
        venue.display_name = self.display_name.clone();
        venue.formatted_address = self.formatted_address.clone();
        venue.place_id = self.place_id.clone();
        venue.lat = self.lat;
        venue.lng = self.lng;
        venue.timezone = self.timezone.clone();
        venue.source = self.source.clone();
    }

    /// Validates the DTO and converts to Venue if valid
    pub fn try_into_venue(self) -> std::result::Result<Venue, validator::ValidationErrors> {
        self.validate()?;
        Ok(Venue::from(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::venue::VenueSource;
    use validator::Validate;

    #[test]
    fn test_venue_dto_validation_empty_display_name() {
        let dto = VenueDto {
            id: "venue/1".to_string(),
            display_name: "".to_string(),
            formatted_address: "Somewhere".to_string(),
            place_id: "pid".to_string(),
            lat: 1.0,
            lng: 2.0,
            timezone: "UTC".to_string(),
            source: VenueSource::Database,
        };
        assert!(dto.validate().is_err());
    }

    #[test]
    fn test_venue_dto_validation_empty_place_id() {
        let dto = VenueDto {
            id: "venue/1".to_string(),
            display_name: "Venue".to_string(),
            formatted_address: "Somewhere".to_string(),
            place_id: "".to_string(),
            lat: 1.0,
            lng: 2.0,
            timezone: "UTC".to_string(),
            source: VenueSource::Database,
        };
        assert!(dto.validate().is_err());
    }

    #[test]
    fn test_venue_dto_validation_negative_lat_lng() {
        let dto = VenueDto {
            id: "venue/1".to_string(),
            display_name: "Venue".to_string(),
            formatted_address: "Somewhere".to_string(),
            place_id: "pid".to_string(),
            lat: -91.0,
            lng: 200.0,
            timezone: "UTC".to_string(),
            source: VenueSource::Database,
        };
        assert!(dto.validate().is_err());
    }
}

#[cfg(test)]
mod conversion_tests {
    use super::*;
    use crate::models::venue::VenueSource;

    #[test]
    fn test_venue_dto_to_model_invalid() {
        let dto = VenueDto {
            id: "venue/1".to_string(),
            display_name: "".to_string(), // Invalid: empty display_name
            formatted_address: "Somewhere".to_string(),
            place_id: "pid".to_string(),
            lat: 1.0,
            lng: 2.0,
            timezone: "UTC".to_string(),
            source: VenueSource::Database,
        };
        let result = dto.try_into_venue();
        assert!(result.is_err());
    }
}
