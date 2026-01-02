use arangors::Database;
use log;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueWithNullPlaceId {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_rev")]
    pub rev: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "formattedAddress")]
    pub formatted_address: String,
    pub place_id: Option<String>,
    pub lat: f64,
    pub lng: f64,
    pub timezone: String,
}

#[derive(Debug, Clone)]
pub struct GooglePlacesMigrationService {
    api_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl GooglePlacesMigrationService {
    pub fn new(api_url: String, api_key: String) -> Self {
        Self {
            api_url,
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub async fn find_place_id(&self, venue: &VenueWithNullPlaceId) -> Result<Option<String>> {
        // Create a search query combining name and address
        let search_query = format!("{} {}", venue.display_name, venue.formatted_address);
        log::info!("🔍 Searching for place_id: '{}'", search_query);

        // Use the Places API Text Search endpoint for better results
        let search_url = format!("{}/place/textsearch/json", self.api_url);
        
        let params = [
            ("query", &search_query),
            ("key", &self.api_key),
            ("type", &"establishment".to_string()),
        ];

        let response = self.client
            .get(&search_url)
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            log::warn!("⚠️  Google Places API request failed for '{}': {}", search_query, response.status());
            return Ok(None);
        }

        let response_text = response.text().await?;
        let search_response: serde_json::Value = serde_json::from_str(&response_text)?;

        if search_response["status"] != "OK" {
            log::warn!("⚠️  Google Places API returned status '{}' for '{}'", 
                      search_response["status"], search_query);
            return Ok(None);
        }

        if let Some(results) = search_response["results"].as_array() {
            if !results.is_empty() {
                if let Some(place_id) = results[0]["place_id"].as_str() {
                    log::info!("✅ Found place_id '{}' for venue '{}'", place_id, venue.display_name);
                    return Ok(Some(place_id.to_string()));
                }
            }
        }

        log::warn!("⚠️  No place_id found for venue '{}'", venue.display_name);
        Ok(None)
    }
}

pub async fn migrate_venues_place_id(
    db: &Database<arangors::client::reqwest::ReqwestClient>,
    google_api_url: &str,
    google_api_key: &str,
) -> Result<(), String> {
    log::info!("🔄 Starting place_id migration for venues...");
    
    // Initialize Google Places service
    let google_service = GooglePlacesMigrationService::new(
        google_api_url.to_string(),
        google_api_key.to_string(),
    );

    // Find all venues with null or empty place_id (but not venues that already have valid place_ids)
    let query = arangors::AqlQuery::builder()
        .query(r#"
            FOR venue IN venue
            FILTER venue.place_id == null 
                OR venue.place_id == "" 
                OR venue.place_id == "Unknown Place"
                OR venue.place_id == "unknown"
                OR venue.place_id == "Unknown"
            RETURN venue
        "#)
        .build();
    
    match db.aql_query::<VenueWithNullPlaceId>(query).await {
        Ok(cursor) => {
            let venues: Vec<VenueWithNullPlaceId> = cursor.into_iter().collect();
            log::info!("📊 Found {} venues with missing place_id", venues.len());
            
            if venues.is_empty() {
                log::info!("✅ No venues need place_id migration");
                return Ok(());
            }

            // Log what we're about to process
            log::info!("🔍 Venues to be processed:");
            for (i, venue) in venues.iter().enumerate() {
                log::info!("  {}. '{}' (current place_id: {:?})", 
                    i + 1, venue.display_name, venue.place_id);
            }

            let mut updated_count = 0;
            let mut error_count = 0;
            let mut skipped_count = 0;

            for (index, venue) in venues.iter().enumerate() {
                log::info!("🔄 Processing venue {}/{}: '{}'", index + 1, venues.len(), venue.display_name);
                
                // Rate limiting - Google Places API has rate limits
                if index > 0 {
                    sleep(Duration::from_millis(200)).await; // 5 requests per second
                }

                match google_service.find_place_id(venue).await {
                    Ok(Some(place_id)) => {
                        // Update the venue with the found place_id
                        let update_query = arangors::AqlQuery::builder()
                            .query(r#"
                                UPDATE @venue_id WITH { place_id: @place_id } IN venue
                                RETURN NEW
                            "#)
                            .bind_var("venue_id", venue.id.clone())
                            .bind_var("place_id", place_id.clone())
                            .build();

                        match db.aql_query::<serde_json::Value>(update_query).await {
                            Ok(_) => {
                                log::info!("✅ Updated venue '{}' with place_id: {}", venue.display_name, place_id);
                                updated_count += 1;
                            },
                            Err(e) => {
                                log::error!("❌ Failed to update venue '{}': {}", venue.display_name, e);
                                error_count += 1;
                            }
                        }
                    },
                    Ok(None) => {
                        log::warn!("⚠️  No place_id found for venue '{}', skipping", venue.display_name);
                        skipped_count += 1;
                    },
                    Err(e) => {
                        log::error!("❌ Error searching for venue '{}': {}", venue.display_name, e);
                        error_count += 1;
                    }
                }
            }

            log::info!("📊 Migration completed:");
            log::info!("  ✅ Updated: {} venues", updated_count);
            log::info!("  ⚠️  Skipped: {} venues", skipped_count);
            log::info!("  ❌ Errors: {} venues", error_count);
        },
        Err(e) => {
            log::error!("❌ Failed to query venues: {}", e);
            return Err(format!("Database query failed: {}", e));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_places_service_creation() {
        let service = GooglePlacesMigrationService::new(
            "https://maps.googleapis.com/maps/api".to_string(),
            "test_key".to_string(),
        );
        assert_eq!(service.api_url, "https://maps.googleapis.com/maps/api");
        assert_eq!(service.api_key, "test_key");
    }
}
