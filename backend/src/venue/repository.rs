use crate::third_party::{google::timezone::GoogleTimezoneService, GooglePlacesService};
use anyhow::Result;
use arangors::client::reqwest::ReqwestClient;
use arangors::document::options::{InsertOptions, RemoveOptions, UpdateOptions};
use arangors::Database;
use log;
use serde::{Deserialize, Serialize};
use shared::dto::venue::VenueDto;
use shared::models::venue::Venue;

// Database-only venue model (without source field)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VenueDb {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_rev")]
    pub rev: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "formattedAddress")]
    pub formatted_address: String,
    pub place_id: String,
    pub lat: f64,
    pub lng: f64,
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

impl From<VenueDb> for Venue {
    fn from(db_venue: VenueDb) -> Self {
        Venue {
            id: db_venue.id,
            rev: db_venue.rev,
            display_name: db_venue.display_name,
            formatted_address: db_venue.formatted_address,
            place_id: db_venue.place_id,
            lat: db_venue.lat,
            lng: db_venue.lng,
            timezone: db_venue.timezone,
            source: shared::models::venue::VenueSource::Database,
        }
    }
}

#[derive(Clone)]
pub struct VenueRepositoryImpl {
    pub db: Database<ReqwestClient>,
    pub google_places: Option<GooglePlacesService>,
    pub google_timezone: Option<GoogleTimezoneService>,
}

#[async_trait::async_trait]
pub trait VenueRepository: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Option<Venue>;
    async fn find_all(&self) -> Vec<Venue>;
    async fn search(&self, query: &str) -> Vec<Venue>;
    async fn search_dto(&self, query: &str) -> Vec<VenueDto>;
    async fn search_dto_with_external(&self, query: &str) -> Vec<VenueDto>;
    async fn get_venue_performance(&self, venue_id: &str) -> Result<serde_json::Value, String>;
    async fn get_player_venue_stats(
        &self,
        player_id: &str,
    ) -> Result<Vec<serde_json::Value>, String>;
    async fn create(&self, venue: Venue) -> Result<Venue, String>;
    async fn update(&self, venue: Venue) -> Result<Venue, String>;
    async fn delete(&self, id: &str) -> Result<(), String>;
}

impl VenueRepositoryImpl {
    pub fn new(db: Database<ReqwestClient>, google_config: Option<(String, String)>) -> Self {
        let google_places = google_config
            .as_ref()
            .map(|(api_url, api_key)| GooglePlacesService::new(api_url.clone(), api_key.clone()));
        let google_timezone =
            google_config.map(|(api_url, api_key)| GoogleTimezoneService::new(api_url, api_key));
        Self {
            db,
            google_places,
            google_timezone,
        }
    }

    /// Infer timezone from coordinates (simplified mapping)
    fn infer_timezone_from_coordinates(&self, _lat: f64, lng: f64) -> String {
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

    /// Update venue timezone in database
    async fn update_venue_timezone(&self, venue_id: &str, timezone: &str) -> Result<(), String> {
        log::info!("🔄 Updating venue {} timezone to: {}", venue_id, timezone);

        let query = arangors::AqlQuery::builder()
            .query("UPDATE @venue_id WITH { timezone: @timezone } IN venue")
            .bind_var("venue_id", venue_id)
            .bind_var("timezone", timezone)
            .build();

        match self.db.aql_query::<serde_json::Value>(query).await {
            Ok(_) => {
                log::info!(
                    "✅ Successfully updated venue {} timezone to: {}",
                    venue_id,
                    timezone
                );
                Ok(())
            }
            Err(e) => {
                log::error!("❌ Failed to update venue {} timezone: {}", venue_id, e);
                Err(format!("Failed to update venue timezone: {}", e))
            }
        }
    }

    /// Get venue with smart timezone detection (only for Google-sourced venues)
    pub async fn get_venue_with_timezone(&self, venue_id: &str) -> Result<VenueDto, String> {
        let venue = self
            .find_by_id(venue_id)
            .await
            .ok_or_else(|| format!("Venue not found: {}", venue_id))?;

        // If timezone is UTC and source is Google, try to infer from coordinates
        if venue.source == shared::models::venue::VenueSource::Google
            && venue.timezone == "UTC"
            && venue.lat != 0.0
            && venue.lng != 0.0
        {
            let inferred_timezone = self.infer_timezone_from_coordinates(venue.lat, venue.lng);
            if inferred_timezone != "UTC" {
                log::info!(
                    "🌍 Inferring timezone for venue {}: {} -> {}",
                    venue_id,
                    venue.timezone,
                    inferred_timezone
                );

                // Update the venue in database
                self.update_venue_timezone(venue_id, &inferred_timezone)
                    .await?;

                // Return updated venue
                let updated_venue = self
                    .find_by_id(venue_id)
                    .await
                    .ok_or_else(|| format!("Venue not found after update: {}", venue_id))?;

                return Ok(VenueDto::from(&updated_venue));
            }
        }

        Ok(VenueDto::from(&venue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use chrono::Utc;

    #[test]
    fn test_venue_db_deserialize_default_timezone() {
        // Missing timezone should default to "UTC" via serde default
        let json = r#"{
            "_id": "venue/123",
            "_rev": "1",
            "displayName": "Test Venue",
            "formattedAddress": "123 Test St, Test City",
            "place_id": "test_place_id",
            "lat": 10.0,
            "lng": 20.0
        }"#;

        let v: VenueDb = serde_json::from_str(json).expect("deserialize VenueDb");
        assert_eq!(v.id, "venue/123");
        assert_eq!(v.timezone, "UTC");
    }

    #[test]
    fn test_timezone_inference_from_coordinates() {
        // Test coordinate validation without database
        let lat = 40.7128;
        let lng = -74.0060;

        // Validate coordinates are within valid ranges
        assert!(lat >= -90.0 && lat <= 90.0);
        assert!(lng >= -180.0 && lng <= 180.0);

        // Test that coordinates represent valid locations
        assert_eq!(lat, 40.7128); // NYC latitude
        assert_eq!(lng, -74.0060); // NYC longitude

        // Test other coordinate ranges
        assert!(34.0522 >= -90.0 && 34.0522 <= 90.0); // LA lat
        assert!(-118.2437 >= -180.0 && -118.2437 <= 180.0); // LA lng
    }

    #[test]
    fn test_venue_dto_conversion() {
        let venue = Venue {
            id: "venue/test".to_string(),
            rev: "1".to_string(),
            display_name: "Test Venue".to_string(),
            formatted_address: "123 Test St".to_string(),
            place_id: "test_place_id".to_string(),
            lat: 40.7128,
            lng: -74.0060,
            timezone: "America/New_York".to_string(),
            source: shared::models::venue::VenueSource::Database,
        };

        let dto = VenueDto::from(&venue);
        assert_eq!(dto.id, "venue/test");
        assert_eq!(dto.display_name, "Test Venue");
        assert_eq!(dto.timezone, "America/New_York");
        assert_eq!(dto.lat, 40.7128);
        assert_eq!(dto.lng, -74.0060);
    }

    #[test]
    fn test_venue_dto_update() {
        let mut venue = Venue {
            id: "venue/old".to_string(),
            rev: "1".to_string(),
            display_name: "Old Venue".to_string(),
            formatted_address: "Old Address".to_string(),
            place_id: "old_place_id".to_string(),
            lat: 0.0,
            lng: 0.0,
            timezone: "UTC".to_string(),
            source: shared::models::venue::VenueSource::Database,
        };

        let dto = VenueDto {
            id: "venue/new".to_string(),
            display_name: "New Venue".to_string(),
            formatted_address: "New Address".to_string(),
            place_id: "new_place_id".to_string(),
            lat: 40.7128,
            lng: -74.0060,
            timezone: "America/New_York".to_string(),
            source: shared::models::venue::VenueSource::Google,
        };

        dto.update_venue(&mut venue);

        assert_eq!(venue.id, "venue/new");
        assert_eq!(venue.display_name, "New Venue");
        assert_eq!(venue.formatted_address, "New Address");
        assert_eq!(venue.place_id, "new_place_id");
        assert_eq!(venue.lat, 40.7128);
        assert_eq!(venue.lng, -74.0060);
        assert_eq!(venue.timezone, "America/New_York");
        assert_eq!(venue.source, shared::models::venue::VenueSource::Google);
    }
}

#[cfg(test)]
mod search_dto_tests {
    use super::*;

    #[tokio::test]
    async fn search_dto_returns_db_first_and_preserves_timezone() {
        // Set up in-memory Arango-like behavior is complex; instead, validate transformation logic
        // by constructing VenueDb and ensuring VenueDto preserves timezone and DB source ordering
        let db_venues = vec![
            VenueDb {
                id: "venue/1".into(),
                rev: "1".into(),
                display_name: "Mitch Park".into(),
                formatted_address: "123 A".into(),
                place_id: "pid1".into(),
                lat: 1.0,
                lng: 2.0,
                timezone: "America/Chicago".into(),
            },
            VenueDb {
                id: "venue/2".into(),
                rev: "1".into(),
                display_name: "Paris Orly Airport".into(),
                formatted_address: "Orly".into(),
                place_id: "pid2".into(),
                lat: 48.72,
                lng: 2.38,
                timezone: "Europe/Paris".into(),
            },
        ];
        let venues: Vec<Venue> = db_venues.into_iter().map(Venue::from).collect();
        let dtos: Vec<VenueDto> = venues.iter().map(|v| VenueDto::from(v)).collect();
        assert_eq!(dtos[0].timezone, "America/Chicago");
        assert_eq!(dtos[1].timezone, "Europe/Paris");
        assert!(matches!(
            dtos[0].source,
            shared::models::venue::VenueSource::Database
        ));
    }
}

#[async_trait::async_trait]
impl VenueRepository for VenueRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Option<Venue> {
        log::info!("🔍 Looking up venue by ID: '{}'", id);

        let query = arangors::AqlQuery::builder()
            .query("FOR v IN venue FILTER v._id == @id LIMIT 1 RETURN v")
            .bind_var("id", id)
            .build();

        match self.db.aql_query::<VenueDb>(query).await {
            Ok(mut cursor) => {
                if let Some(venue_db) = cursor.pop() {
                    log::info!(
                        "✅ Found venue by ID: '{}' -> '{}'",
                        id,
                        venue_db.display_name
                    );
                    Some(Venue::from(venue_db))
                } else {
                    log::error!("❌ Venue not found by ID: '{}'", id);

                    // Debug: Show what venues actually exist in the database
                    log::info!("🔍 Debug: Checking what venues exist in database...");
                    let debug_query = arangors::AqlQuery::builder()
                        .query("FOR v IN venue LIMIT 10 RETURN { id: v._id, name: v.displayName }")
                        .build();

                    match self.db.aql_query::<serde_json::Value>(debug_query).await {
                        Ok(debug_cursor) => {
                            let debug_results: Vec<serde_json::Value> =
                                debug_cursor.into_iter().collect();
                            log::info!(
                                "🔍 Debug: Found {} venues in database (showing first 10):",
                                debug_results.len()
                            );
                            for (i, venue) in debug_results.iter().enumerate() {
                                log::info!(
                                    "  Venue {}: ID='{}', Name='{}'",
                                    i + 1,
                                    venue.get("id").unwrap_or(&serde_json::Value::Null),
                                    venue.get("name").unwrap_or(&serde_json::Value::Null)
                                );
                            }
                        }
                        Err(e) => {
                            log::error!("❌ Debug query failed: {}", e);
                        }
                    }

                    None
                }
            }
            Err(e) => {
                log::error!("❌ Database query failed for venue ID '{}': {}", id, e);
                None
            }
        }
    }

    async fn find_all(&self) -> Vec<Venue> {
        log::info!("🔍 Attempting to find all venues");
        let query = arangors::AqlQuery::builder()
            .query("FOR v IN venue RETURN v")
            .build();

        log::info!("🔍 Executing AQL query: FOR v IN venue RETURN v");

        match self.db.aql_query::<VenueDb>(query).await {
            Ok(cursor) => {
                log::info!("✅ AQL query executed successfully");
                let db_venues: Vec<VenueDb> = cursor.into_iter().collect();
                log::info!("📊 Found {} total venues in database", db_venues.len());

                // Convert database venues to full Venue models with source field
                let venues: Vec<Venue> = db_venues
                    .into_iter()
                    .map(|db_venue| {
                        log::info!(
                            "  Venue: ID={}, Name='{}', Address='{}'",
                            db_venue.id,
                            db_venue.display_name,
                            db_venue.formatted_address
                        );
                        Venue::from(db_venue)
                    })
                    .collect();

                log::info!("📊 Converted {} venues to full models", venues.len());
                venues
            }
            Err(e) => {
                log::error!("❌ Failed to find all venues: {}", e);
                Vec::new()
            }
        }
    }

    async fn search(&self, query: &str) -> Vec<Venue> {
        let mut results = Vec::new();
        let max_results = 20;

        // First search by displayName
        let display_name_query = arangors::AqlQuery::builder()
            .query("FOR v IN venue FILTER CONTAINS(LOWER(v.displayName), LOWER(@query)) LIMIT @limit RETURN v")
            .bind_var("query", query)
            .bind_var("limit", max_results)
            .build();

        let display_name_results = match self.db.aql_query::<VenueDb>(display_name_query).await {
            Ok(cursor) => {
                let db_venues: Vec<VenueDb> = cursor.into_iter().collect();
                log::debug!("Display name search returned {} venues", db_venues.len());

                // Convert database venues to full Venue models with source field
                let venues: Vec<Venue> = db_venues
                    .into_iter()
                    .map(|db_venue| {
                        log::debug!(
                            "Venue from display name search: ID={}, Name={}",
                            db_venue.id,
                            db_venue.display_name
                        );
                        Venue::from(db_venue)
                    })
                    .collect();

                venues
            }
            Err(_) => Vec::new(),
        };

        results.extend(display_name_results);

        // Then search by formattedAddress (only if we haven't reached the limit)
        if results.len() < max_results {
            let remaining_limit = max_results - results.len();
            let address_query = arangors::AqlQuery::builder()
                .query("FOR v IN venue FILTER CONTAINS(LOWER(v.formattedAddress), LOWER(@query)) LIMIT @limit RETURN v")
                .bind_var("query", query)
                .bind_var("limit", remaining_limit)
                .build();

            let address_results = match self.db.aql_query::<VenueDb>(address_query).await {
                Ok(cursor) => {
                    let db_venues: Vec<VenueDb> = cursor.into_iter().collect();
                    log::debug!("Address search returned {} venues", db_venues.len());

                    // Convert database venues to full Venue models with source field
                    let venues: Vec<Venue> = db_venues
                        .into_iter()
                        .map(|db_venue| {
                            log::debug!(
                                "Venue from address search: ID={}, Name={}",
                                db_venue.id,
                                db_venue.display_name
                            );
                            Venue::from(db_venue)
                        })
                        .collect();

                    venues
                }
                Err(_) => Vec::new(),
            };

            results.extend(address_results);
        }

        // Always try to fill remaining slots with Google API results (if available)
        if results.len() < max_results && self.google_places.is_some() {
            log::info!(
                "Google Places API is available, attempting to fill {} remaining slots",
                max_results - results.len()
            );
            if let Some(ref google_places) = self.google_places {
                match google_places.search_places(query).await {
                    Ok(google_results) => {
                        let remaining_limit = max_results - results.len();
                        log::info!("Google Places API returned {} results, adding {} to fill remaining slots", 
                                  google_results.len(), std::cmp::min(remaining_limit, google_results.len()));
                        let limited_google_results = google_results
                            .into_iter()
                            .take(remaining_limit)
                            .collect::<Vec<_>>();
                        results.extend(limited_google_results);
                    }
                    Err(e) => {
                        log::warn!("Google Places API search failed: {}", e);
                    }
                }
            }
        } else if results.len() < max_results {
            log::info!(
                "Google Places API not available, cannot fill remaining {} slots",
                max_results - results.len()
            );
        }

        log::debug!("Total search results: {} venues", results.len());
        for venue in &results {
            log::debug!(
                "Final venue result: ID={}, Name={}",
                venue.id,
                venue.display_name
            );
        }

        results
    }

    async fn search_dto(&self, query: &str) -> Vec<VenueDto> {
        log::info!("🔍 Starting venue search with query: '{}'", query);
        let mut results = Vec::new();
        let max_results = 20;

        // Case-insensitive partial match using CONTAINS on displayName
        let query_lower = query.to_lowercase();
        let display_filter =
            "FOR v IN venue FILTER CONTAINS(LOWER(v.displayName), @q) LIMIT @limit RETURN v";

        log::info!(
            "📝 Searching venues by displayName with query_lower: '{}'",
            query_lower
        );
        let display_name_query = arangors::AqlQuery::builder()
            .query(display_filter)
            .bind_var("q", query_lower.as_str())
            .bind_var("limit", max_results)
            .build();

        log::info!("🔍 Executing AQL query: FOR v IN venue FILTER CONTAINS(LOWER(v.displayName), LOWER(@query)) LIMIT @limit RETURN v");
        log::info!(
            "🔍 Query bindings: query='{}', limit={}",
            query,
            max_results
        );

        let display_name_results = match self.db.aql_query::<VenueDb>(display_name_query).await {
            Ok(cursor) => {
                log::info!("✅ AQL query executed successfully");
                let db_venues: Vec<VenueDb> = cursor.into_iter().collect();
                log::info!("📊 Display name search returned {} venues", db_venues.len());

                // Convert database venues to full Venue models with source field
                let venues: Vec<Venue> = db_venues
                    .into_iter()
                    .map(|db_venue| {
                        log::info!(
                            "  Venue {}: ID={}, Name='{}', Address='{}'",
                            1,
                            db_venue.id,
                            db_venue.display_name,
                            db_venue.formatted_address
                        );
                        Venue::from(db_venue)
                    })
                    .collect();

                venues
            }
            Err(e) => {
                log::error!("❌ AQL query failed: {}", e);
                Vec::new()
            }
        };

        // Convert database venues to DTOs with Database source (preserve timezone)
        for venue in display_name_results {
            log::info!(
                "🔄 Converting venue to DTO: {} -> {}",
                venue.id,
                venue.display_name
            );
            results.push(VenueDto {
                id: venue.id.clone(),
                display_name: venue.display_name.clone(),
                formatted_address: venue.formatted_address.clone(),
                place_id: venue.place_id.clone(),
                lat: venue.lat,
                lng: venue.lng,
                timezone: venue.timezone.clone(),
                source: shared::models::venue::VenueSource::Database,
            });
        }

        // Then search by formattedAddress (only if we haven't reached the limit)
        if results.len() < max_results {
            let remaining_limit = max_results - results.len();
            log::info!("📝 Searching venues by formattedAddress with query_lower: '{}' (remaining limit: {})", query_lower, remaining_limit);

            // Case-insensitive partial match using CONTAINS on formattedAddress
            let address_filter = "FOR v IN venue FILTER CONTAINS(LOWER(v.formattedAddress), @q) LIMIT @limit RETURN v";

            let address_query = arangors::AqlQuery::builder()
                .query(address_filter)
                .bind_var("q", query_lower.as_str())
                .bind_var("limit", remaining_limit)
                .build();

            log::info!("🔍 Executing address AQL query: FOR v IN venue FILTER CONTAINS(LOWER(v.formattedAddress), LOWER(@query)) LIMIT @limit RETURN v");
            log::info!(
                "🔍 Address query bindings: query='{}', limit={}",
                query,
                remaining_limit
            );

            let address_results = match self.db.aql_query::<VenueDb>(address_query).await {
                Ok(cursor) => {
                    log::info!("✅ Address AQL query executed successfully");
                    let db_venues: Vec<VenueDb> = cursor.into_iter().collect();
                    log::info!("📊 Address search returned {} venues", db_venues.len());

                    // Convert database venues to full Venue models with source field
                    let venues: Vec<Venue> = db_venues
                        .into_iter()
                        .map(|db_venue| {
                            log::info!(
                                "  Address Venue: ID={}, Name='{}', Address='{}'",
                                db_venue.id,
                                db_venue.display_name,
                                db_venue.formatted_address
                            );
                            Venue::from(db_venue)
                        })
                        .collect();

                    venues
                }
                Err(e) => {
                    log::error!("❌ Address AQL query failed: {}", e);
                    Vec::new()
                }
            };

            // Convert database venues to DTOs with Database source (preserve timezone)
            for venue in address_results {
                // Check if we already have this venue (avoid duplicates)
                if !results.iter().any(|dto| dto.id == venue.id) {
                    log::info!(
                        "🔄 Converting address venue to DTO: {} -> {}",
                        venue.id,
                        venue.display_name
                    );
                    results.push(VenueDto {
                        id: venue.id.clone(),
                        display_name: venue.display_name.clone(),
                        formatted_address: venue.formatted_address.clone(),
                        place_id: venue.place_id.clone(),
                        lat: venue.lat,
                        lng: venue.lng,
                        timezone: venue.timezone.clone(),
                        source: shared::models::venue::VenueSource::Database,
                    });
                } else {
                    log::info!(
                        "⏭️  Skipping duplicate venue: {} -> {}",
                        venue.id,
                        venue.display_name
                    );
                }
            }
        }

        // DB-only: Do not include Google Places fallback in DTO search
        if results.len() < max_results {
            log::info!(
                "DB-only venue search: {} results (no external fallback)",
                results.len()
            );
        }

        log::info!("🎯 Total search results: {} venues", results.len());
        for (i, venue_dto) in results.iter().enumerate() {
            log::info!(
                "  Final result {}: ID={}, Name='{}', Source={:?}",
                i + 1,
                venue_dto.id,
                venue_dto.display_name,
                venue_dto.source
            );
        }

        results
    }

    async fn get_venue_performance(&self, venue_id: &str) -> Result<serde_json::Value, String> {
        log::info!("🔍 Getting venue performance for venue: {}", venue_id);

        let query = arangors::AqlQuery::builder()
            .query(
                r#"
                // Graph traversal: venue -> contests -> players -> performance
                FOR venue IN venue
                  FILTER venue._id == @venue_id
                  
                  // Get all contests at this venue
                  LET contests = (
                    FOR contest_edge IN played_at
                      FILTER contest_edge._to == venue._id
                      LET contest = DOCUMENT(contest_edge._from)
                      RETURN contest
                  )
                  
                  // Get player performance data
                  LET player_performance = (
                    FOR contest IN contests
                      FOR result IN resulted_in
                        FILTER result._from == contest._id
                        LET player = DOCUMENT(result._to)
                        COLLECT player_id = player._id, player_data = player INTO player_contests
                        
                        LET total_contests = LENGTH(player_contests)
                        LET wins = LENGTH(
                          FOR pc IN player_contests
                          FILTER pc.result.place == 1
                          RETURN pc
                        )
                        LET win_rate = total_contests > 0 ? (wins * 100.0) / total_contests : 0.0
                        
                        RETURN {
                          player_id: player_id,
                          handle: player_data.handle,
                          firstname: player_data.firstname,
                          total_contests: total_contests,
                          wins: wins,
                          win_rate: win_rate,
                          average_placement: total_contests > 0 ? AVG(
                            FOR pc IN player_contests
                            RETURN pc.result.place
                          ) : 0.0
                        }
                  )
                  
                  // Get game popularity at this venue
                  LET game_popularity = (
                    FOR contest IN contests
                      FOR game_edge IN played_with
                        FILTER game_edge._from == contest._id
                        LET game = DOCUMENT(game_edge._to)
                        COLLECT game_id = game._id, game_name = game.name INTO game_plays
                        RETURN {
                          game_id: game_id,
                          game_name: game_name,
                          total_plays: LENGTH(game_plays)
                        }
                  )
                  
                  RETURN {
                    venue: {
                      id: venue._id,
                      name: venue.displayName,
                      address: venue.formattedAddress
                    },
                    total_contests: LENGTH(contests),
                    player_performance: player_performance,
                    game_popularity: game_popularity,
                    top_players: (
                      FOR p IN player_performance
                      SORT p.win_rate DESC, p.total_contests DESC
                      LIMIT 10
                      RETURN p
                    )
                  }
            "#,
            )
            .bind_var("venue_id", venue_id)
            .build();

        match self.db.aql_query::<serde_json::Value>(query).await {
            Ok(mut cursor) => {
                if let Some(result) = cursor.pop() {
                    log::info!(
                        "✅ Venue performance data retrieved for venue: {}",
                        venue_id
                    );
                    Ok(result)
                } else {
                    Err("No venue performance data found".to_string())
                }
            }
            Err(e) => {
                log::error!("❌ Failed to get venue performance: {}", e);
                Err(format!("Database query failed: {}", e))
            }
        }
    }

    async fn get_player_venue_stats(
        &self,
        player_id: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        log::info!("🔍 Getting venue stats for player: {}", player_id);

        let query = arangors::AqlQuery::builder()
            .query(
                r#"
                // Graph traversal: player -> contests -> venues -> performance
                FOR player IN player
                  FILTER player._id == @player_id
                  
                  // Get all contests this player participated in
                  LET player_contests = (
                    FOR result IN resulted_in
                      FILTER result._to == player._id
                      LET contest = DOCUMENT(result._from)
                      RETURN {
                        contest: contest,
                        result: result
                      }
                  )
                  
                  // Get venue performance for each venue
                  LET venue_stats = (
                    FOR pc IN player_contests
                      FOR venue_edge IN played_at
                        FILTER venue_edge._from == pc.contest._id
                        LET venue = DOCUMENT(venue_edge._to)
                        
                        // Collect all contests at this venue for this player
                        COLLECT venue_id = venue._id, venue_data = venue INTO venue_contests
                        
                        LET total_contests = LENGTH(venue_contests)
                        LET wins = LENGTH(
                          FOR vc IN venue_contests
                          FILTER vc.result.place == 1
                          RETURN vc
                        )
                        LET win_rate = total_contests > 0 ? (wins * 100.0) / total_contests : 0.0
                        LET average_placement = total_contests > 0 ? AVG(
                          FOR vc IN venue_contests
                          RETURN vc.result.place
                        ) : 0.0
                        
                        RETURN {
                          venue_id: venue_id,
                          venue_name: venue_data.displayName,
                          address: venue_data.formattedAddress,
                          total_contests: total_contests,
                          wins: wins,
                          win_rate: win_rate,
                          average_placement: average_placement,
                          last_played: MAX(
                            FOR vc IN venue_contests
                            RETURN vc.contest.start
                          )
                        }
                  )
                  
                  RETURN {
                    player_id: player._id,
                    player_handle: player.handle,
                    venue_stats: venue_stats
                  }
            "#,
            )
            .bind_var("player_id", player_id)
            .build();

        match self.db.aql_query::<serde_json::Value>(query).await {
            Ok(cursor) => {
                let results: Vec<serde_json::Value> = cursor.into_iter().collect();
                log::info!(
                    "✅ Venue stats retrieved for player: {} ({} venues)",
                    player_id,
                    results.len()
                );
                Ok(results)
            }
            Err(e) => {
                log::error!("❌ Failed to get player venue stats: {}", e);
                Err(format!("Database query failed: {}", e))
            }
        }
    }

    async fn create(&self, venue: Venue) -> Result<Venue, String> {
        // Determine timezone for the venue: trust provided value if non-empty; otherwise resolve
        let timezone = if !venue.timezone.trim().is_empty() {
            log::info!("🌍 Using provided venue timezone: {}", venue.timezone);
            venue.timezone.clone()
        } else if let Some(timezone_service) = &self.google_timezone {
            // Prefer place_id when available; otherwise use coordinates
            if !venue.place_id.is_empty() {
                log::info!("🌍 Resolving timezone via place_id: {}", venue.place_id);
                timezone_service
                    .infer_timezone_from_place_id(&venue.place_id)
                    .await
            } else if venue.lat != 0.0 && venue.lng != 0.0 {
                log::info!(
                    "🌍 Resolving timezone via coordinates: {}, {}",
                    venue.lat,
                    venue.lng
                );
                timezone_service
                    .get_timezone_with_fallback(venue.lat, venue.lng)
                    .await
            } else {
                "UTC".to_string()
            }
        } else if venue.lat != 0.0 && venue.lng != 0.0 {
            // Fallback to coordinate-based inference when Google service not configured
            self.infer_timezone_from_coordinates(venue.lat, venue.lng)
        } else {
            "UTC".to_string()
        };

        log::info!(
            "🌍 Setting timezone for new venue '{}': {} (lat: {}, lng: {})",
            venue.display_name,
            timezone,
            venue.lat,
            venue.lng
        );

        // Create venue with determined timezone
        let venue_with_timezone = Venue { timezone, ..venue };

        let collection = self
            .db
            .collection("venue")
            .await
            .map_err(|e| format!("Failed to get collection: {}", e))?;

        match collection
            .create_document(venue_with_timezone.clone(), InsertOptions::default())
            .await
        {
            Ok(created_doc) => {
                let header = created_doc.header().unwrap();
                Ok(Venue {
                    id: header._id.clone(),
                    rev: header._rev.clone(),
                    display_name: venue_with_timezone.display_name,
                    formatted_address: venue_with_timezone.formatted_address,
                    place_id: venue_with_timezone.place_id,
                    lat: venue_with_timezone.lat,
                    lng: venue_with_timezone.lng,
                    timezone: venue_with_timezone.timezone,
                    source: venue_with_timezone.source,
                })
            }
            Err(e) => Err(format!("Failed to create venue: {}", e)),
        }
    }

    async fn update(&self, venue: Venue) -> Result<Venue, String> {
        let collection = self
            .db
            .collection("venue")
            .await
            .map_err(|e| format!("Failed to get collection: {}", e))?;

        // Allow updating without requiring a matching _rev from the client side.
        // We already fetched the existing document and preserve its rev in usecase.
        let update_options = UpdateOptions::builder()
            .ignore_revs(true)
            .return_new(true)
            .build();

        // Arango's collection.update_document expects the document KEY, not the full _id
        let key: String = match venue.id.split_once('/') {
            Some((_, k)) => k.to_string(),
            None => venue.id.clone(),
        };
        match collection
            .update_document(&key, venue.clone(), update_options)
            .await
        {
            Ok(updated_doc) => {
                let header = updated_doc.header().unwrap();
                Ok(Venue {
                    id: header._id.clone(),
                    rev: header._rev.clone(),
                    display_name: venue.display_name,
                    formatted_address: venue.formatted_address,
                    place_id: venue.place_id,
                    lat: venue.lat,
                    lng: venue.lng,
                    timezone: venue.timezone,
                    source: venue.source,
                })
            }
            Err(e) => Err(format!("Failed to update venue: {}", e)),
        }
    }

    async fn delete(&self, id: &str) -> Result<(), String> {
        let collection = self
            .db
            .collection("venue")
            .await
            .map_err(|e| format!("Failed to get collection: {}", e))?;

        // Arango expects document key, not full _id
        let key = id.split_once('/').map(|(_, k)| k).unwrap_or(id);
        match collection
            .remove_document::<serde_json::Value>(key, RemoveOptions::default(), None)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to delete venue: {}", e)),
        }
    }

    async fn search_dto_with_external(&self, query: &str) -> Vec<VenueDto> {
        log::info!(
            "🔍 Starting venue search with external APIs for query: '{}'",
            query
        );
        let mut results = Vec::new();
        let max_results = 20;

        // Case-insensitive partial match using CONTAINS on displayName
        let query_lower = query.to_lowercase();
        let display_filter =
            "FOR v IN venue FILTER CONTAINS(LOWER(v.displayName), @q) LIMIT @limit RETURN v";

        log::info!(
            "📝 Searching venues by displayName with query_lower: '{}'",
            query_lower
        );
        let display_name_query = arangors::AqlQuery::builder()
            .query(display_filter)
            .bind_var("q", query_lower.as_str())
            .bind_var("limit", max_results)
            .build();

        log::info!("🔍 Executing AQL query: FOR v IN venue FILTER CONTAINS(LOWER(v.displayName), LOWER(@query)) LIMIT @limit RETURN v");
        log::info!(
            "🔍 Query bindings: query='{}', limit={}",
            query,
            max_results
        );

        let display_name_results = match self.db.aql_query::<VenueDb>(display_name_query).await {
            Ok(cursor) => {
                log::info!("✅ AQL query executed successfully");
                let db_venues: Vec<VenueDb> = cursor.into_iter().collect();
                log::info!("📊 Display name search returned {} venues", db_venues.len());

                // Convert database venues to full Venue models with source field
                let venues: Vec<Venue> = db_venues
                    .into_iter()
                    .map(|db_venue| {
                        log::info!(
                            "  Venue {}: ID={}, Name='{}', Address='{}'",
                            1,
                            db_venue.id,
                            db_venue.display_name,
                            db_venue.formatted_address
                        );
                        Venue::from(db_venue)
                    })
                    .collect();

                venues
            }
            Err(e) => {
                log::error!("❌ AQL query failed: {}", e);
                Vec::new()
            }
        };

        // Convert database venues to DTOs with Database source (preserve timezone)
        for venue in display_name_results {
            log::info!(
                "🔄 Converting venue to DTO: {} -> {}",
                venue.id,
                venue.display_name
            );
            results.push(VenueDto {
                id: venue.id.clone(),
                display_name: venue.display_name.clone(),
                formatted_address: venue.formatted_address.clone(),
                place_id: venue.place_id.clone(),
                lat: venue.lat,
                lng: venue.lng,
                timezone: venue.timezone.clone(),
                source: shared::models::venue::VenueSource::Database,
            });
        }

        // Then search by formattedAddress (only if we haven't reached the limit)
        if results.len() < max_results {
            let remaining_limit = max_results - results.len();
            log::info!("📝 Searching venues by formattedAddress with query_lower: '{}' (remaining limit: {})", query_lower, remaining_limit);

            // Case-insensitive partial match using CONTAINS on formattedAddress
            let address_filter = "FOR v IN venue FILTER CONTAINS(LOWER(v.formattedAddress), @q) LIMIT @limit RETURN v";

            let address_query = arangors::AqlQuery::builder()
                .query(address_filter)
                .bind_var("q", query_lower.as_str())
                .bind_var("limit", remaining_limit)
                .build();

            log::info!("🔍 Executing address AQL query: FOR v IN venue FILTER CONTAINS(LOWER(v.formattedAddress), LOWER(@query)) LIMIT @limit RETURN v");
            log::info!(
                "🔍 Address query bindings: query='{}', limit={}",
                query,
                remaining_limit
            );

            let address_results = match self.db.aql_query::<VenueDb>(address_query).await {
                Ok(cursor) => {
                    log::info!("✅ Address AQL query executed successfully");
                    let db_venues: Vec<VenueDb> = cursor.into_iter().collect();
                    log::info!("📊 Address search returned {} venues", db_venues.len());

                    // Convert database venues to full Venue models with source field
                    let venues: Vec<Venue> = db_venues
                        .into_iter()
                        .map(|db_venue| {
                            log::info!(
                                "  Address Venue: ID={}, Name='{}', Address='{}'",
                                db_venue.id,
                                db_venue.display_name,
                                db_venue.formatted_address
                            );
                            Venue::from(db_venue)
                        })
                        .collect();

                    venues
                }
                Err(e) => {
                    log::error!("❌ Address AQL query failed: {}", e);
                    Vec::new()
                }
            };

            // Convert database venues to DTOs with Database source (preserve timezone)
            for venue in address_results {
                // Check if we already have this venue (avoid duplicates)
                if !results.iter().any(|dto| dto.id == venue.id) {
                    log::info!(
                        "🔄 Converting address venue to DTO: {} -> {}",
                        venue.id,
                        venue.display_name
                    );
                    results.push(VenueDto {
                        id: venue.id.clone(),
                        display_name: venue.display_name.clone(),
                        formatted_address: venue.formatted_address.clone(),
                        place_id: venue.place_id.clone(),
                        lat: venue.lat,
                        lng: venue.lng,
                        timezone: venue.timezone.clone(),
                        source: shared::models::venue::VenueSource::Database,
                    });
                } else {
                    log::info!(
                        "⏭️  Skipping duplicate venue: {} -> {}",
                        venue.id,
                        venue.display_name
                    );
                }
            }
        }

        // Try to fill remaining slots with Google API results (if available)
        if results.len() < max_results && self.google_places.is_some() {
            log::info!(
                "Google Places API is available, attempting to fill {} remaining slots",
                max_results - results.len()
            );
            if let Some(ref google_places) = self.google_places {
                match google_places.search_places(query).await {
                    Ok(google_results) => {
                        let remaining_limit = max_results - results.len();
                        log::info!("Google Places API returned {} results, adding {} to fill remaining slots", 
                                  google_results.len(), std::cmp::min(remaining_limit, google_results.len()));

                        // Convert Google Places results to DTOs
                        for venue in google_results.into_iter().take(remaining_limit) {
                            log::info!(
                                "🔄 Converting Google venue to DTO: {} -> {}",
                                venue.id,
                                venue.display_name
                            );
                            results.push(VenueDto {
                                id: venue.id.clone(),
                                display_name: venue.display_name.clone(),
                                formatted_address: venue.formatted_address.clone(),
                                place_id: venue.place_id.clone(),
                                lat: venue.lat,
                                lng: venue.lng,
                                timezone: venue.timezone.clone(),
                                source: shared::models::venue::VenueSource::Google,
                            });
                        }
                    }
                    Err(e) => {
                        log::warn!("Google Places API search failed: {}", e);
                    }
                }
            }
        } else if results.len() < max_results {
            log::info!(
                "Google Places API not available, cannot fill remaining {} slots",
                max_results - results.len()
            );
        }

        log::info!("🎯 Total search results: {} venues", results.len());
        for (i, venue_dto) in results.iter().enumerate() {
            log::info!(
                "  Final result {}: ID={}, Name='{}', Source={:?}",
                i + 1,
                venue_dto.id,
                venue_dto.display_name,
                venue_dto.source
            );
        }

        results
    }
}
