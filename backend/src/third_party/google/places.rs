use serde::{Deserialize, Serialize};
use shared::models::venue::Venue;
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
struct GoogleAutocompleteResponse {
    predictions: Vec<GooglePrediction>,
    status: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GooglePrediction {
    place_id: String,
    description: String,
    structured_formatting: GoogleStructuredFormatting,
}

#[derive(Debug, Serialize, Deserialize)]
struct GoogleStructuredFormatting {
    main_text: String,
    secondary_text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GooglePlaceDetailsResponse {
    result: GooglePlaceDetails,
    status: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GooglePlaceDetails {
    place_id: String,
    formatted_address: String,
    name: String,
    geometry: GoogleGeometry,
}

#[derive(Debug, Serialize, Deserialize)]
struct GoogleGeometry {
    location: GoogleLocation,
}

#[derive(Debug, Serialize, Deserialize)]
struct GoogleLocation {
    lat: f64,
    lng: f64,
}

#[derive(Clone)]
pub struct GooglePlacesService {
    api_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl GooglePlacesService {
    pub fn new(api_url: String, api_key: String) -> Self {
        Self {
            api_url,
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub async fn search_places(&self, query: &str) -> Result<Vec<Venue>> {
        log::info!("Searching Google Places API for query: '{}'", query);
        log::info!("Using API URL: '{}'", self.api_url);
        
        // Use the API URL exactly as provided and append query parameters
        let params = [
            ("input", query),
            ("key", &self.api_key),
            ("types", "establishment"),
        ];

        log::info!("Making autocomplete request to: {}", self.api_url);
        let key_prefix = self.api_key.get(..10).unwrap_or(&self.api_key);
        log::info!("With params: input={}, types=establishment, key={}...", query, key_prefix);

        let response = self.client
            .get(&self.api_url)
            .query(&params)
            .send()
            .await?;

        log::info!("Autocomplete response status: {}", response.status());
        
        // Log the response body for debugging
        let response_text = response.text().await?;
        log::debug!("Response body: {}", response_text);
        
        // Parse the JSON response
        let autocomplete_response: GoogleAutocompleteResponse = serde_json::from_str(&response_text)?;

        log::info!("Autocomplete response status: {}, predictions: {}", 
                  autocomplete_response.status, autocomplete_response.predictions.len());

        if autocomplete_response.status != "OK" && autocomplete_response.status != "ZERO_RESULTS" {
            return Err(anyhow::anyhow!("Google Places Autocomplete API error: {}", autocomplete_response.status));
        }

        if autocomplete_response.predictions.is_empty() {
            log::info!("No autocomplete predictions found for query: '{}'", query);
            return Ok(Vec::new());
        }

        // Get details for each prediction (limit to first 5 to avoid too many API calls)
        let mut venues = Vec::new();
        for (i, prediction) in autocomplete_response.predictions.iter().take(5).enumerate() {
            log::debug!("Getting details for prediction {}: {}", i + 1, prediction.description);
            if let Ok(venue) = self.get_place_details(&prediction.place_id).await {
                venues.push(venue);
            } else {
                log::warn!("Failed to get details for prediction {}: {}", i + 1, prediction.description);
            }
        }

        log::info!("Successfully retrieved {} venues from Google Places API", venues.len());
        Ok(venues)
    }

    async fn get_place_details(&self, place_id: &str) -> Result<Venue> {
        // For details, replace autocomplete with details in the URL
        let details_url = self.api_url.replace("/place/autocomplete/json", "/place/details/json");
        
        let params = [
            ("place_id", place_id),
            ("key", &self.api_key),
            ("fields", "place_id,formatted_address,name,geometry"),
        ];

        log::debug!("Getting place details for place_id: {}", place_id);

        let response = self.client
            .get(&details_url)
            .query(&params)
            .send()
            .await?;

        log::debug!("Place details response status: {}", response.status());

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Google Places Details API request failed: {}", response.status()));
        }

        let details_response: GooglePlaceDetailsResponse = response.json().await?;

        log::debug!("Place details response status: {}", details_response.status);

        if details_response.status != "OK" {
            return Err(anyhow::anyhow!("Google Places Details API error: {}", details_response.status));
        }

        let details = details_response.result;
        
        log::debug!("Retrieved place details: {} at {}", details.name, details.formatted_address);
        
        Ok(Venue::new_for_db(
            details.name,
            details.formatted_address,
            details.place_id,
            details.geometry.location.lat,
            details.geometry.location.lng,
            "UTC".to_string(), // Default timezone for Google Places
            shared::models::venue::VenueSource::Google,
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use test_log::test;

    #[test]
    fn test_google_places_service_creation() {
        let service = GooglePlacesService::new(
            "https://maps.googleapis.com/maps/api/place/autocomplete/json".to_string(),
            "test_api_key".to_string(),
        );
        assert_eq!(service.api_url, "https://maps.googleapis.com/maps/api/place/autocomplete/json");
        assert_eq!(service.api_key, "test_api_key");
    }

    #[test]
    fn test_google_places_service_api_url_format() {
        let service = GooglePlacesService::new(
            "https://maps.googleapis.com/maps/api/place/autocomplete/json".to_string(),
            "test_api_key".to_string(),
        );
        assert!(service.api_url.starts_with("https://maps.googleapis.com/maps/api/place"));
        assert!(service.api_url.contains("autocomplete"));
    }
} 