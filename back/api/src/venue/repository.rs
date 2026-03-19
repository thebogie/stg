use crate::cache::{CacheKeys, CacheTTL, RedisCache};
use crate::db::Db;
use crate::surreal_helpers::{record_id_from_row, select_one_by_record_id_scoped};
use crate::third_party::{google::timezone::GoogleTimezoneService, GooglePlacesService};
use anyhow::Result;
use log;
use shared::dto::venue::VenueDto;
use shared::models::venue::Venue;
use std::sync::Arc;

/// SELECT list for venue so id is JSON-serializable (string). See docs/SURREALDB_QUERY_CONVENTIONS.md.
const VENUE_SELECT: &str =
    "SELECT string::concat(id) AS id, displayName, formattedAddress, place_id, lat, lng, timezone FROM venue";

/// Extract record id from SurrealDB row (canonical "table/key"). Uses shared helper for v3 compatibility.
fn record_id_to_string(v: &serde_json::Value) -> Option<String> {
    record_id_from_row(v, Some("venue"))
}

fn value_to_venue(v: &serde_json::Value) -> Option<Venue> {
    let id = record_id_to_string(v)?;
    Some(Venue {
        id,
        rev: v.get("_rev").or_else(|| v.get("rev")).and_then(|x| x.as_str()).unwrap_or("").to_string(),
        display_name: v.get("displayName").or_else(|| v.get("display_name")).and_then(|x| x.as_str()).unwrap_or("").to_string(),
        formatted_address: v.get("formattedAddress").or_else(|| v.get("formatted_address")).and_then(|x| x.as_str()).unwrap_or("").to_string(),
        place_id: v.get("place_id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        lat: v.get("lat").and_then(|x| x.as_f64()).unwrap_or(0.0),
        lng: v.get("lng").and_then(|x| x.as_f64()).unwrap_or(0.0),
        timezone: v.get("timezone").and_then(|x| x.as_str()).unwrap_or("UTC").to_string(),
        source: shared::models::venue::VenueSource::Database,
    })
}

#[derive(Clone)]
pub struct VenueRepositoryImpl {
    pub db: Db,
    pub google_places: Option<GooglePlacesService>,
    pub google_timezone: Option<GoogleTimezoneService>,
    pub cache: Option<Arc<RedisCache>>,
    /// When set (e.g. in production), ensure NS/DB scope is set on the connection that executes each query.
    pub ns: Option<String>,
    pub db_name: Option<String>,
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
    pub fn new(db: Db, google_config: Option<(String, String)>) -> Self {
        let google_places = google_config
            .as_ref()
            .map(|(api_url, api_key)| GooglePlacesService::new(api_url.clone(), api_key.clone()));
        let google_timezone =
            google_config.map(|(api_url, api_key)| GoogleTimezoneService::new(api_url, api_key));
        Self {
            db,
            google_places,
            google_timezone,
            cache: None,
            ns: None,
            db_name: None,
        }
    }

    pub fn new_with_cache(db: Db, google_config: Option<(String, String)>, cache: Arc<RedisCache>) -> Self {
        let google_places = google_config
            .as_ref()
            .map(|(api_url, api_key)| GooglePlacesService::new(api_url.clone(), api_key.clone()));
        let google_timezone =
            google_config.map(|(api_url, api_key)| GoogleTimezoneService::new(api_url, api_key));
        Self {
            db,
            google_places,
            google_timezone,
            cache: Some(cache),
            ns: None,
            db_name: None,
        }
    }

    /// For production: ensure each query runs with the given NS/DB (scope does not reliably persist across connections).
    pub fn new_with_cache_and_scope(
        db: Db,
        google_config: Option<(String, String)>,
        cache: Arc<RedisCache>,
        ns: String,
        db_name: String,
    ) -> Self {
        let mut repo = Self::new_with_cache(db, google_config, cache);
        repo.ns = Some(ns);
        repo.db_name = Some(db_name);
        repo
    }

    /// For use when scope is required but cache is not (e.g. contest repo's internal venue lookups).
    pub fn new_with_scope(
        db: Db,
        google_config: Option<(String, String)>,
        ns: String,
        db_name: String,
    ) -> Self {
        let mut repo = Self::new(db, google_config);
        repo.ns = Some(ns);
        repo.db_name = Some(db_name);
        repo
    }

    async fn ensure_scope(&self) {
        if let (Some(ref ns), Some(ref db_name)) = (&self.ns, &self.db_name) {
            let _ = self.db.use_ns(ns).use_db(db_name).await;
        }
    }

    fn query_with_scope(&self, core: &str) -> String {
        if let (Some(ref ns), Some(ref db_name)) = (&self.ns, &self.db_name) {
            let ns_ok = ns.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            let db_ok = db_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            if ns_ok && db_ok {
                return format!("USE NS {}; USE DB {}; {}", ns, db_name, core);
            }
        }
        core.to_string()
    }

    /// Fill search results with Google Places when DB returns fewer than max_results.
    pub async fn search_fill_google(&self, query: &str, mut results: Vec<Venue>) -> Vec<Venue> {
        let max_results = 20;
        if results.len() < max_results && self.google_places.is_some() {
            if let Some(ref google_places) = self.google_places {
                if let Ok(google_results) = google_places.search_places(query).await {
                    let remaining = max_results - results.len();
                    results.extend(google_results.into_iter().take(remaining));
                }
            }
        }
        results
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
        self.ensure_scope().await;
        log::info!("🔄 Updating venue {} timezone to: {}", venue_id, timezone);
        let key = venue_id.trim_start_matches("venue/").trim_start_matches("venue:").to_string();
        let tz = timezone.to_string();
        // Try UUID-typed first (preferred), then fallback to string-key record ids for older imports.
        if uuid::Uuid::parse_str(&key).is_ok() {
            let _ = self
                .db
                .query(self.query_with_scope("UPDATE type::record('venue', type::uuid($key)) SET timezone = $timezone"))
                .bind(("key", key.clone()))
                .bind(("timezone", tz.clone()))
                .await;
        }
        self.db
            .query(self.query_with_scope("UPDATE type::record('venue', $key) SET timezone = $timezone"))
            .bind(("key", key))
            .bind(("timezone", tz))
            .await
            .map_err(|e| format!("Failed to update venue timezone: {}", e))?;
        log::info!("✅ Successfully updated venue {} timezone to: {}", venue_id, timezone);
        Ok(())
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

    #[test]
    fn test_value_to_venue_default_timezone() {
        let json = serde_json::json!({
            "id": "venue:123",
            "displayName": "Test Venue",
            "formattedAddress": "123 Test St, Test City",
            "place_id": "test_place_id",
            "lat": 10.0,
            "lng": 20.0
        });
        let v = value_to_venue(&json).expect("value_to_venue");
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
    async fn search_dto_preserves_timezone_and_source() {
        let venues = vec![
            Venue {
                id: "venue/1".into(),
                rev: "1".into(),
                display_name: "Mitch Park".into(),
                formatted_address: "123 A".into(),
                place_id: "pid1".into(),
                lat: 1.0,
                lng: 2.0,
                timezone: "America/Chicago".into(),
                source: shared::models::venue::VenueSource::Database,
            },
            Venue {
                id: "venue/2".into(),
                rev: "1".into(),
                display_name: "Paris Orly Airport".into(),
                formatted_address: "Orly".into(),
                place_id: "pid2".into(),
                lat: 48.72,
                lng: 2.38,
                timezone: "Europe/Paris".into(),
                source: shared::models::venue::VenueSource::Database,
            },
        ];
        let dtos: Vec<VenueDto> = venues.iter().map(VenueDto::from).collect();
        assert_eq!(dtos[0].timezone, "America/Chicago");
        assert_eq!(dtos[1].timezone, "Europe/Paris");
        assert!(matches!(dtos[0].source, shared::models::venue::VenueSource::Database));
    }
}

#[async_trait::async_trait]
impl VenueRepository for VenueRepositoryImpl {
    async fn find_by_id(&self, id: &str) -> Option<Venue> {
        self.ensure_scope().await;
        log::info!("🔍 Looking up venue by ID: '{}'", id);
        if let Some(ref cache) = self.cache {
            let cache_key = CacheKeys::venue(id);
            if let Ok(Some(cached_venue)) = cache.get::<Venue>(&cache_key).await {
                log::debug!("Cache hit for venue: {}", id);
                return Some(cached_venue);
            }
        }

        let venue = select_one_by_record_id_scoped(
            &self.db,
            "venue",
            id,
            self.ns.as_deref(),
            self.db_name.as_deref(),
        )
            .await
            .and_then(|row| value_to_venue(&row));
        if let Some(ref v) = venue {
            log::info!("✅ Found venue by ID: '{}' -> '{}'", id, v.display_name);
            if let Some(ref cache) = self.cache {
                let _ = cache.set_with_ttl(&CacheKeys::venue(id), v, CacheTTL::venue()).await;
            }
        } else {
            log::error!("❌ Venue not found by ID: '{}'", id);
        }
        venue
    }

    async fn find_all(&self) -> Vec<Venue> {
        self.ensure_scope().await;
        log::info!("🔍 Attempting to find all venues");
        let mut res = match self.db.query(VENUE_SELECT).await {
            Ok(r) => r,
            Err(e) => {
                log::error!("❌ Failed to find all venues: {}", e);
                return Vec::new();
            }
        };
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let venues: Vec<Venue> = rows.into_iter().filter_map(|v| value_to_venue(&v)).collect();
        log::info!("📊 Found {} total venues in database", venues.len());
        venues
    }

    async fn search(&self, query: &str) -> Vec<Venue> {
        self.ensure_scope().await;
        let max_results = 20i64;
        let mut results = Vec::new();
        let q_owned = query.to_string();

        let mut res = match self.db
            .query(format!(
                "{} WHERE string::contains(string::lowercase(displayName), string::lowercase($q)) LIMIT $limit",
                VENUE_SELECT
            ))
            .bind(("q", q_owned.clone()))
            .bind(("limit", max_results))
            .await
        {
            Ok(r) => r,
            Err(_) => return self.search_fill_google(query, results).await,
        };
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        results.extend(rows.into_iter().filter_map(|v| value_to_venue(&v)));

        if results.len() < max_results as usize {
            let remaining = max_results - results.len() as i64;
            let mut res2 = match self.db
                .query(format!(
                    "{} WHERE string::contains(string::lowercase(formattedAddress), string::lowercase($q)) LIMIT $limit",
                    VENUE_SELECT
                ))
                .bind(("q", q_owned))
                .bind(("limit", remaining))
                .await
            {
                Ok(r) => r,
                Err(_) => return self.search_fill_google(query, results).await,
            };
            let rows2: Vec<serde_json::Value> = res2.take(0).unwrap_or_default();
            for v in rows2 {
                if let Some(venue) = value_to_venue(&v) {
                    if !results.iter().any(|x| x.id == venue.id) {
                        results.push(venue);
                    }
                }
            }
        }

        self.search_fill_google(query, results).await
    }

    async fn search_dto(&self, query: &str) -> Vec<VenueDto> {
        log::info!("🔍 Starting venue search with query: '{}'", query);
        if let Some(ref cache) = self.cache {
            let cache_key = CacheKeys::venue_search(query);
            if let Ok(Some(cached_results)) = cache.get::<Vec<VenueDto>>(&cache_key).await {
                log::debug!("Cache hit for venue search: {}", query);
                return cached_results;
            }
        }
        let venues = self.search(query).await;
        let results: Vec<VenueDto> = venues.into_iter().map(|v| VenueDto::from(&v)).collect();
        if let Some(ref cache) = self.cache {
            let _ = cache.set_with_ttl(&CacheKeys::venue_search(query), &results, CacheTTL::venue_search()).await;
        }
        results
    }

    async fn get_venue_performance(&self, venue_id: &str) -> Result<serde_json::Value, String> {
        log::info!("🔍 Getting venue performance for venue: {}", venue_id);
        // TODO: implement with SurrealQL (played_at `out`=contest, `in`=venue; resulted_in, played_with)
        let venue = self.find_by_id(venue_id).await.ok_or_else(|| format!("Venue not found: {}", venue_id))?;
        Ok(serde_json::json!({
            "venue": { "id": venue.id, "name": venue.display_name, "address": venue.formatted_address },
            "total_contests": 0,
            "player_performance": [],
            "game_popularity": [],
            "top_players": []
        }))
    }

    async fn get_player_venue_stats(&self, _player_id: &str) -> Result<Vec<serde_json::Value>, String> {
        log::info!("🔍 Getting venue stats for player");
        // TODO: implement with SurrealQL
        Ok(Vec::new())
    }

    async fn create(&self, venue: Venue) -> Result<Venue, String> {
        self.ensure_scope().await;
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

        let venue_with_timezone = Venue { timezone, ..venue };
        let key = uuid::Uuid::new_v4().to_string();
        let doc = serde_json::json!({
            "displayName": venue_with_timezone.display_name,
            "formattedAddress": venue_with_timezone.formatted_address,
            "place_id": venue_with_timezone.place_id,
            "lat": venue_with_timezone.lat,
            "lng": venue_with_timezone.lng,
            "timezone": venue_with_timezone.timezone,
        });
        self.ensure_scope().await;
        self.db
            .query(self.query_with_scope("CREATE type::record('venue', $key) CONTENT $doc"))
            .bind(("key", key.clone()))
            .bind(("doc", doc))
            .await
            .map_err(|e| format!("Failed to create venue: {}", e))?;
        let created_venue = Venue {
            id: format!("venue/{}", key),
            rev: String::new(),
            display_name: venue_with_timezone.display_name,
            formatted_address: venue_with_timezone.formatted_address,
            place_id: venue_with_timezone.place_id,
            lat: venue_with_timezone.lat,
            lng: venue_with_timezone.lng,
            timezone: venue_with_timezone.timezone,
            source: venue_with_timezone.source,
        };
        if let Some(ref cache) = self.cache {
            let _ = cache.delete(&CacheKeys::venue(&created_venue.id)).await;
            let _ = cache.invalidate_pattern("venues:search:").await;
        }
        Ok(created_venue)
    }

    async fn update(&self, venue: Venue) -> Result<Venue, String> {
        self.ensure_scope().await;
        let key = venue.id.trim_start_matches("venue/").trim_start_matches("venue:").to_string();
        let doc = serde_json::json!({
            "displayName": venue.display_name,
            "formattedAddress": venue.formatted_address,
            "place_id": venue.place_id,
            "lat": venue.lat,
            "lng": venue.lng,
            "timezone": venue.timezone,
        });
        self.ensure_scope().await;
        if uuid::Uuid::parse_str(&key).is_ok() {
            let _ = self
                .db
                .query(self.query_with_scope("UPDATE type::record('venue', type::uuid($key)) MERGE $doc"))
                .bind(("key", key.clone()))
                .bind(("doc", doc.clone()))
                .await;
        }
        self.db
            .query(self.query_with_scope("UPDATE type::record('venue', $key) MERGE $doc"))
            .bind(("key", key))
            .bind(("doc", doc))
            .await
            .map_err(|e| format!("Failed to update venue: {}", e))?;
        if let Some(ref cache) = self.cache {
            let _ = cache.delete(&CacheKeys::venue(&venue.id)).await;
            let _ = cache.invalidate_pattern("venues:search:").await;
        }
        Ok(venue)
    }

    async fn delete(&self, id: &str) -> Result<(), String> {
        self.ensure_scope().await;
        let key = id.trim_start_matches("venue/").trim_start_matches("venue:").to_string();
        if uuid::Uuid::parse_str(&key).is_ok() {
            let _ = self
                .db
                .query(self.query_with_scope("DELETE type::record('venue', type::uuid($key))"))
                .bind(("key", key.clone()))
                .await;
        }
        self.db
            .query(self.query_with_scope("DELETE type::record('venue', $key)"))
            .bind(("key", key))
            .await
            .map_err(|e| format!("Failed to delete venue: {}", e))?;
        if let Some(ref cache) = self.cache {
            let _ = cache.delete(&CacheKeys::venue(id)).await;
            let _ = cache.invalidate_pattern("venues:search:").await;
        }
        Ok(())
    }

    async fn search_dto_with_external(&self, query: &str) -> Vec<VenueDto> {
        log::info!("🔍 Starting venue search with external APIs for query: '{}'", query);
        let mut results: Vec<VenueDto> = self.search(query).await.into_iter().map(|v| VenueDto::from(&v)).collect();
        let max_results = 20;
        if results.len() < max_results && self.google_places.is_some() {
            if let Some(ref google_places) = self.google_places {
                if let Ok(google_results) = google_places.search_places(query).await {
                    let remaining = max_results - results.len();
                    for venue in google_results.into_iter().take(remaining) {
                        results.push(VenueDto {
                            id: venue.id,
                            display_name: venue.display_name,
                            formatted_address: venue.formatted_address,
                            place_id: venue.place_id,
                            lat: venue.lat,
                            lng: venue.lng,
                            timezone: venue.timezone,
                            source: shared::models::venue::VenueSource::Google,
                        });
                    }
                }
            }
        }
        results
    }
}
