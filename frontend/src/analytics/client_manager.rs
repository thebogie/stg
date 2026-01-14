// use gloo_net::http::Request; // replaced by authenticated_request
use serde_json::json;
use shared::dto::client_sync::*;
use shared::models::client_analytics::*;
use shared::models::client_storage::*;

use wasm_bindgen_futures::spawn_local;
use web_sys::console;
// Add LRU cache for better memory management
use lru::LruCache;

/// Frontend analytics manager that coordinates client-side data and server sync
pub struct ClientAnalyticsManager {
    storage: Box<dyn ClientStorage>,
    memory_cache: LruCache<String, ClientAnalyticsCache>,
    sync_in_progress: bool,
}

impl ClientAnalyticsManager {
    pub fn new() -> Self {
        let config = StorageConfig::default();
        let storage = Box::new(LocalStorageClient::new(config));

        Self {
            storage,
            memory_cache: LruCache::new(std::num::NonZeroUsize::new(100).unwrap()), // Limit to 100 players for memory management
            sync_in_progress: false,
        }
    }

    /// Initializes analytics data for a player (called on login)
    pub async fn initialize_player_analytics(
        &mut self,
        player_id: &str,
    ) -> Result<ClientAnalyticsCache, String> {
        console::log_1(&format!("Initializing client analytics for player: {}", player_id).into());

        // Try to load from memory cache first
        if let Some(cache) = self.memory_cache.get(player_id) {
            if !cache.needs_refresh(24) {
                console::log_1(
                    &format!("Using cached analytics data for player: {}", player_id).into(),
                );
                return Ok(cache.clone());
            }
        }

        // Try to load from persistent storage
        if let Ok(Some(cache)) = self.storage.get_analytics_cache(player_id).await {
            if !cache.needs_refresh(24) {
                console::log_1(
                    &format!("Loaded analytics from storage for player: {}", player_id).into(),
                );
                self.memory_cache.put(player_id.to_string(), cache.clone());
                return Ok(cache);
            }
        }

        // Create new cache and trigger initial sync
        console::log_1(&format!("Creating new analytics cache for player: {}", player_id).into());
        let cache = ClientAnalyticsCache::new(player_id.to_string());
        self.memory_cache.put(player_id.to_string(), cache.clone());

        // Trigger background sync
        self.trigger_background_sync(player_id);

        Ok(cache)
    }

    /// Triggers background data synchronization
    fn trigger_background_sync(&self, player_id: &str) {
        let player_id = player_id.to_string();
        spawn_local(async move {
            if let Err(e) = Self::sync_data_from_server(&player_id).await {
                console::error_1(
                    &format!("Background sync failed for player {}: {}", player_id, e).into(),
                );
            }
        });
    }

    /// Syncs data from server
    async fn sync_data_from_server(player_id: &str) -> Result<(), String> {
        console::log_1(&format!("Starting server sync for player: {}", player_id).into());

        let request = ClientSyncRequest {
            player_id: player_id.to_string(),
            last_contest_id: None,
            last_sync: None,
            full_sync: true,
            limit: Some(1000), // Get up to 1000 contests
            include_related: true,
        };

        let request_builder = crate::api::utils::authenticated_request("POST", "/api/client/sync")
            .header("Content-Type", "application/json")
            .body(json!(request).to_string())
            .map_err(|e| format!("Request creation error: {}", e))?;

        let response = request_builder
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !response.ok() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!(
                "Server error: {} - {}",
                response.status(),
                error_text
            ));
        }

        let sync_response: ClientSyncResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse sync response: {}", e))?;

        console::log_1(
            &format!(
                "Server sync completed for player: {}, contests: {}",
                player_id,
                sync_response.contests.len()
            )
            .into(),
        );

        // Store the synced data (this would be handled by the storage layer)
        // For now, we'll just log success
        Ok(())
    }

    /// Gets analytics data with optional filtering
    pub async fn get_analytics(
        &mut self,
        player_id: &str,
        query: AnalyticsQuery,
    ) -> Result<ComputedAnalytics, String> {
        let cache = self
            .memory_cache
            .get(player_id)
            .ok_or_else(|| "Player analytics not loaded".to_string())?;

        console::log_1(&format!("Computing analytics for player: {} with query", player_id).into());

        Ok(cache.query_analytics(query))
    }

    /// Gets core stats (fast access)
    pub async fn get_core_stats(&mut self, player_id: &str) -> Result<CoreStats, String> {
        let cache = self
            .memory_cache
            .get(player_id)
            .ok_or_else(|| "Player analytics not loaded".to_string())?;

        Ok(cache.core_stats.clone())
    }

    /// Checks if data needs refresh
    pub async fn needs_refresh(&mut self, player_id: &str) -> bool {
        if let Some(cache) = self.memory_cache.get(player_id) {
            cache.needs_refresh(24)
        } else {
            true // No cache means refresh needed
        }
    }

    /// Manually triggers a data refresh
    pub async fn refresh_data(&mut self, player_id: &str) -> Result<(), String> {
        if self.sync_in_progress {
            return Err("Sync already in progress".to_string());
        }

        self.sync_in_progress = true;
        console::log_1(&format!("Manual refresh triggered for player: {}", player_id).into());

        // Trigger sync
        let result = Self::sync_data_from_server(player_id).await;

        self.sync_in_progress = false;
        result
    }

    /// Gets storage statistics
    pub async fn get_storage_stats(&self) -> Result<StorageStats, String> {
        self.storage
            .get_storage_stats()
            .await
            .map_err(|e| format!("Storage error: {}", e))
    }

    /// Clears all data for a player
    pub async fn clear_player_data(&mut self, player_id: &str) -> Result<(), String> {
        console::log_1(&format!("Clearing client data for player: {}", player_id).into());

        // Remove from memory cache
        self.memory_cache.pop(player_id);

        // Clear from persistent storage
        self.storage
            .clear_player_data(player_id)
            .await
            .map_err(|e| format!("Storage error: {}", e))
    }

    /// Gets cached contest data
    pub fn get_cached_contests(&mut self, player_id: &str) -> Option<Vec<ClientContest>> {
        self.memory_cache
            .get(player_id)
            .map(|cache| cache.contests.clone())
    }

    /// Gets cached game data
    pub fn get_cached_games(&mut self, player_id: &str) -> Option<Vec<ClientGame>> {
        self.memory_cache
            .get(player_id)
            .map(|cache| cache.game_lookup.values().cloned().collect())
    }

    /// Gets cached venue data
    pub fn get_cached_venues(&mut self, player_id: &str) -> Option<Vec<ClientVenue>> {
        self.memory_cache
            .get(player_id)
            .map(|cache| cache.venue_lookup.values().cloned().collect())
    }

    /// Gets cached opponent data
    pub fn get_cached_opponents(&mut self, player_id: &str) -> Option<Vec<ClientOpponent>> {
        self.memory_cache
            .get(player_id)
            .map(|cache| cache.opponent_lookup.values().cloned().collect())
    }

    /// Checks if player data is fully loaded
    pub fn is_player_data_loaded(&mut self, player_id: &str) -> bool {
        self.memory_cache.contains(player_id)
    }

    /// Gets cache status for a player
    pub fn get_cache_status(&mut self, player_id: &str) -> Option<CacheStatus> {
        self.memory_cache.get(player_id).map(|cache| CacheStatus {
            player_id: cache.player_id.clone(),
            last_updated: cache.last_updated,
            contest_count: cache.contests.len(),
            data_size_bytes: cache.estimate_size(),
            needs_refresh: cache.needs_refresh(24),
        })
    }

    /// Cleans up expired cache entries
    pub fn cleanup_cache(&mut self) {
        // LRU cache automatically manages memory, but we can clean up expired entries
        let mut keys_to_remove = Vec::new();

        for (key, cache) in self.memory_cache.iter() {
            if cache.needs_refresh(0) {
                // 0 hours means expired
                keys_to_remove.push(key.clone());
            }
        }

        for key in keys_to_remove {
            self.memory_cache.pop(&key);
        }
    }

    /// Gets cache statistics
    pub fn get_cache_stats(&self) -> (usize, usize) {
        let total_entries = self.memory_cache.len();
        let expired_entries = self
            .memory_cache
            .iter()
            .filter(|(_, cache)| cache.needs_refresh(24))
            .count();
        (total_entries, expired_entries)
    }
}

/// Cache status information
#[derive(Debug, Clone)]
pub struct CacheStatus {
    pub player_id: String,
    pub last_updated: chrono::DateTime<chrono::FixedOffset>,
    pub contest_count: usize,
    pub data_size_bytes: usize,
    pub needs_refresh: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_status_creation() {
        let status = CacheStatus {
            player_id: "player/test".to_string(),
            last_updated: chrono::Utc::now().fixed_offset(),
            contest_count: 10,
            data_size_bytes: 1024,
            needs_refresh: false,
        };

        assert_eq!(status.player_id, "player/test");
        assert_eq!(status.contest_count, 10);
        assert_eq!(status.data_size_bytes, 1024);
        assert!(!status.needs_refresh);
    }
}
