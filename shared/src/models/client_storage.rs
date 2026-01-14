use crate::models::client_analytics::*;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Storage configuration for client-side analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Maximum age of cached data before refresh (in hours)
    pub max_cache_age_hours: i64,
    /// Maximum size of cached data (in MB)
    pub max_cache_size_mb: usize,
    /// Whether to use IndexedDB (true) or localStorage (false)
    pub use_indexed_db: bool,
    /// Compression threshold (data larger than this gets compressed)
    pub compression_threshold_bytes: usize,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            max_cache_age_hours: 24, // 1 day
            max_cache_size_mb: 50,   // 50MB limit
            use_indexed_db: true,
            compression_threshold_bytes: 1024 * 1024, // 1MB
        }
    }
}

/// Storage keys for different data types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StorageKey {
    AnalyticsCache(String), // player_id
    ContestData(String),    // player_id
    GameData(String),       // game_id
    VenueData(String),      // venue_id
    PlayerData(String),     // player_id
    SyncMetadata(String),   // player_id
}

impl StorageKey {
    pub fn to_string(&self) -> String {
        match self {
            StorageKey::AnalyticsCache(player_id) => format!("analytics_cache_{}", player_id),
            StorageKey::ContestData(player_id) => format!("contest_data_{}", player_id),
            StorageKey::GameData(game_id) => format!("game_data_{}", game_id),
            StorageKey::VenueData(venue_id) => format!("venue_data_{}", venue_id),
            StorageKey::PlayerData(player_id) => format!("player_data_{}", player_id),
            StorageKey::SyncMetadata(player_id) => format!("sync_metadata_{}", player_id),
        }
    }
}

/// Metadata about data synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    pub player_id: String,
    pub last_sync: DateTime<FixedOffset>,
    pub last_contest_id: Option<String>,
    pub data_version: String,
    pub total_contests: usize,
    pub data_size_bytes: usize,
    pub compression_ratio: Option<f64>,
}

/// Client storage interface for analytics data
#[async_trait::async_trait]
pub trait ClientStorage: Send + Sync {
    /// Stores analytics cache data
    async fn store_analytics_cache(&self, cache: &ClientAnalyticsCache)
        -> Result<(), StorageError>;

    /// Retrieves analytics cache data
    async fn get_analytics_cache(
        &self,
        player_id: &str,
    ) -> Result<Option<ClientAnalyticsCache>, StorageError>;

    /// Stores raw contest data
    async fn store_contest_data(
        &self,
        player_id: &str,
        contests: &[ClientContest],
    ) -> Result<(), StorageError>;

    /// Retrieves raw contest data
    async fn get_contest_data(
        &self,
        player_id: &str,
    ) -> Result<Option<Vec<ClientContest>>, StorageError>;

    /// Stores game data
    async fn store_game_data(&self, game: &ClientGame) -> Result<(), StorageError>;

    /// Retrieves game data
    async fn get_game_data(&self, game_id: &str) -> Result<Option<ClientGame>, StorageError>;

    /// Stores venue data
    async fn store_venue_data(&self, venue: &ClientVenue) -> Result<(), StorageError>;

    /// Retrieves venue data
    async fn get_venue_data(&self, venue_id: &str) -> Result<Option<ClientVenue>, StorageError>;

    /// Stores player data
    async fn store_player_data(&self, player: &ClientPlayer) -> Result<(), StorageError>;

    /// Retrieves player data
    async fn get_player_data(&self, player_id: &str) -> Result<Option<ClientPlayer>, StorageError>;

    /// Updates sync metadata
    async fn update_sync_metadata(&self, metadata: &SyncMetadata) -> Result<(), StorageError>;

    /// Gets sync metadata
    async fn get_sync_metadata(
        &self,
        player_id: &str,
    ) -> Result<Option<SyncMetadata>, StorageError>;

    /// Clears all data for a player
    async fn clear_player_data(&self, player_id: &str) -> Result<(), StorageError>;

    /// Gets storage usage statistics
    async fn get_storage_stats(&self) -> Result<StorageStats, StorageError>;
}

/// Simplified player data for client storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientPlayer {
    pub id: String,
    pub handle: String,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub email: Option<String>,
    pub last_seen: DateTime<FixedOffset>,
}

/// Storage error types
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Storage not available: {0}")]
    NotAvailable(String),

    #[error("Data too large: {0} bytes (max: {1} bytes)")]
    DataTooLarge(usize, usize),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Storage quota exceeded")]
    QuotaExceeded,

    #[error("Data corrupted: {0}")]
    DataCorrupted(String),

    #[error("Network error: {0}")]
    Network(String),
}

/// Storage usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_size_bytes: usize,
    pub available_space_bytes: Option<usize>,
    pub player_count: usize,
    pub contest_count: usize,
    pub game_count: usize,
    pub venue_count: usize,
    pub compression_enabled: bool,
}

/// LocalStorage implementation (fallback for older browsers)
pub struct LocalStorageClient {
    config: StorageConfig,
}

// Ensure LocalStorageClient is Send + Sync
unsafe impl Send for LocalStorageClient {}
unsafe impl Sync for LocalStorageClient {}

impl LocalStorageClient {
    pub fn new(config: StorageConfig) -> Self {
        Self { config }
    }

    fn get_item(&self, _key: &str) -> Option<String> {
        // In a real implementation, this would use web_sys::Storage
        // For now, we'll simulate it
        None
    }

    fn set_item(&self, _key: &str, _value: &str) -> Result<(), StorageError> {
        // In a real implementation, this would use web_sys::Storage
        // For now, we'll simulate it
        Ok(())
    }

    fn remove_item(&self, _key: &str) -> Result<(), StorageError> {
        // In a real implementation, this would use web_sys::Storage
        // For now, we'll simulate it
        Ok(())
    }
}

#[async_trait::async_trait]
impl ClientStorage for LocalStorageClient {
    async fn store_analytics_cache(
        &self,
        cache: &ClientAnalyticsCache,
    ) -> Result<(), StorageError> {
        let key = StorageKey::AnalyticsCache(cache.player_id.clone()).to_string();
        let data =
            serde_json::to_string(cache).map_err(|e| StorageError::Serialization(e.to_string()))?;

        // Check size limits
        if data.len() > self.config.max_cache_size_mb * 1024 * 1024 {
            return Err(StorageError::DataTooLarge(
                data.len(),
                self.config.max_cache_size_mb * 1024 * 1024,
            ));
        }

        self.set_item(&key, &data)
    }

    async fn get_analytics_cache(
        &self,
        player_id: &str,
    ) -> Result<Option<ClientAnalyticsCache>, StorageError> {
        let key = StorageKey::AnalyticsCache(player_id.to_string()).to_string();

        if let Some(data) = self.get_item(&key) {
            let cache: ClientAnalyticsCache = serde_json::from_str(&data)
                .map_err(|e| StorageError::Deserialization(e.to_string()))?;

            // Check if cache is still valid
            if cache.needs_refresh(self.config.max_cache_age_hours) {
                return Ok(None); // Cache expired
            }

            Ok(Some(cache))
        } else {
            Ok(None)
        }
    }

    async fn store_contest_data(
        &self,
        player_id: &str,
        contests: &[ClientContest],
    ) -> Result<(), StorageError> {
        let key = StorageKey::ContestData(player_id.to_string()).to_string();
        let data = serde_json::to_string(contests)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        self.set_item(&key, &data)
    }

    async fn get_contest_data(
        &self,
        player_id: &str,
    ) -> Result<Option<Vec<ClientContest>>, StorageError> {
        let key = StorageKey::ContestData(player_id.to_string()).to_string();

        if let Some(data) = self.get_item(&key) {
            let contests: Vec<ClientContest> = serde_json::from_str(&data)
                .map_err(|e| StorageError::Deserialization(e.to_string()))?;
            Ok(Some(contests))
        } else {
            Ok(None)
        }
    }

    async fn store_game_data(&self, game: &ClientGame) -> Result<(), StorageError> {
        let key = StorageKey::GameData(game.id.clone()).to_string();
        let data =
            serde_json::to_string(game).map_err(|e| StorageError::Serialization(e.to_string()))?;

        self.set_item(&key, &data)
    }

    async fn get_game_data(&self, game_id: &str) -> Result<Option<ClientGame>, StorageError> {
        let key = StorageKey::GameData(game_id.to_string()).to_string();

        if let Some(data) = self.get_item(&key) {
            let game: ClientGame = serde_json::from_str(&data)
                .map_err(|e| StorageError::Deserialization(e.to_string()))?;
            Ok(Some(game))
        } else {
            Ok(None)
        }
    }

    async fn store_venue_data(&self, venue: &ClientVenue) -> Result<(), StorageError> {
        let key = StorageKey::VenueData(venue.id.clone()).to_string();
        let data =
            serde_json::to_string(venue).map_err(|e| StorageError::Serialization(e.to_string()))?;

        self.set_item(&key, &data)
    }

    async fn get_venue_data(&self, venue_id: &str) -> Result<Option<ClientVenue>, StorageError> {
        let key = StorageKey::VenueData(venue_id.to_string()).to_string();

        if let Some(data) = self.get_item(&key) {
            let venue: ClientVenue = serde_json::from_str(&data)
                .map_err(|e| StorageError::Deserialization(e.to_string()))?;
            Ok(Some(venue))
        } else {
            Ok(None)
        }
    }

    async fn store_player_data(&self, player: &ClientPlayer) -> Result<(), StorageError> {
        let key = StorageKey::PlayerData(player.id.clone()).to_string();
        let data = serde_json::to_string(player)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        self.set_item(&key, &data)
    }

    async fn get_player_data(&self, player_id: &str) -> Result<Option<ClientPlayer>, StorageError> {
        let key = StorageKey::PlayerData(player_id.to_string()).to_string();

        if let Some(data) = self.get_item(&key) {
            let player: ClientPlayer = serde_json::from_str(&data)
                .map_err(|e| StorageError::Deserialization(e.to_string()))?;
            Ok(Some(player))
        } else {
            Ok(None)
        }
    }

    async fn update_sync_metadata(&self, metadata: &SyncMetadata) -> Result<(), StorageError> {
        let key = StorageKey::SyncMetadata(metadata.player_id.clone()).to_string();
        let data = serde_json::to_string(metadata)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        self.set_item(&key, &data)
    }

    async fn get_sync_metadata(
        &self,
        player_id: &str,
    ) -> Result<Option<SyncMetadata>, StorageError> {
        let key = StorageKey::SyncMetadata(player_id.to_string()).to_string();

        if let Some(data) = self.get_item(&key) {
            let metadata: SyncMetadata = serde_json::from_str(&data)
                .map_err(|e| StorageError::Deserialization(e.to_string()))?;
            Ok(Some(metadata))
        } else {
            Ok(None)
        }
    }

    async fn clear_player_data(&self, player_id: &str) -> Result<(), StorageError> {
        let keys = vec![
            StorageKey::AnalyticsCache(player_id.to_string()),
            StorageKey::ContestData(player_id.to_string()),
            StorageKey::SyncMetadata(player_id.to_string()),
        ];

        for key in keys {
            self.remove_item(&key.to_string())?;
        }

        Ok(())
    }

    async fn get_storage_stats(&self) -> Result<StorageStats, StorageError> {
        // In a real implementation, this would calculate actual storage usage
        Ok(StorageStats {
            total_size_bytes: 0,
            available_space_bytes: None,
            player_count: 0,
            contest_count: 0,
            game_count: 0,
            venue_count: 0,
            compression_enabled: false,
        })
    }
}

/// Analytics data manager that coordinates storage and caching
pub struct AnalyticsDataManager {
    storage: Box<dyn ClientStorage>,
    config: StorageConfig,
    memory_cache: HashMap<String, ClientAnalyticsCache>,
}

impl AnalyticsDataManager {
    pub fn new(storage: Box<dyn ClientStorage>, config: StorageConfig) -> Self {
        Self {
            storage,
            config,
            memory_cache: HashMap::new(),
        }
    }

    /// Initializes analytics data for a player (called on login)
    pub async fn initialize_player_analytics(
        &mut self,
        player_id: &str,
    ) -> Result<ClientAnalyticsCache, StorageError> {
        // Try to load from memory cache first
        if let Some(cache) = self.memory_cache.get(player_id) {
            if !cache.needs_refresh(self.config.max_cache_age_hours) {
                return Ok(cache.clone());
            }
        }

        // Try to load from persistent storage
        if let Some(cache) = self.storage.get_analytics_cache(player_id).await? {
            if !cache.needs_refresh(self.config.max_cache_age_hours) {
                self.memory_cache
                    .insert(player_id.to_string(), cache.clone());
                return Ok(cache);
            }
        }

        // Create new cache (will be populated by sync)
        let cache = ClientAnalyticsCache::new(player_id.to_string());
        self.memory_cache
            .insert(player_id.to_string(), cache.clone());
        Ok(cache)
    }

    /// Syncs analytics data from server
    pub async fn sync_analytics_data(
        &mut self,
        player_id: &str,
        contests: Vec<ClientContest>,
        games: Vec<ClientGame>,
        venues: Vec<ClientVenue>,
        players: Vec<ClientPlayer>,
    ) -> Result<ClientAnalyticsCache, StorageError> {
        // Update memory cache
        let mut cache = self
            .memory_cache
            .get(player_id)
            .cloned()
            .unwrap_or_else(|| ClientAnalyticsCache::new(player_id.to_string()));

        // Update contest data
        cache.contests = contests.clone();

        // Compute analytics
        cache.compute_core_stats();
        cache.build_lookups();

        // Store in memory
        self.memory_cache
            .insert(player_id.to_string(), cache.clone());

        // Store in persistent storage
        self.storage.store_analytics_cache(&cache).await?;
        self.storage
            .store_contest_data(player_id, &contests)
            .await?;

        // Store related data
        for game in games {
            self.storage.store_game_data(&game).await?;
        }

        for venue in venues {
            self.storage.store_venue_data(&venue).await?;
        }

        for player in players {
            self.storage.store_player_data(&player).await?;
        }

        // Update sync metadata
        let metadata = SyncMetadata {
            player_id: player_id.to_string(),
            last_sync: chrono::Utc::now().fixed_offset(),
            last_contest_id: cache.contests.first().map(|c| c.id.clone()),
            data_version: cache.cache_version.clone(),
            total_contests: cache.contests.len(),
            data_size_bytes: cache.estimate_size(),
            compression_ratio: None,
        };

        self.storage.update_sync_metadata(&metadata).await?;

        Ok(cache)
    }

    /// Gets analytics data with optional filtering
    pub async fn get_analytics(
        &self,
        player_id: &str,
        query: AnalyticsQuery,
    ) -> Result<ComputedAnalytics, StorageError> {
        let cache = self
            .memory_cache
            .get(player_id)
            .ok_or_else(|| StorageError::NotAvailable("Player analytics not loaded".to_string()))?;

        Ok(cache.query_analytics(query))
    }

    /// Gets core stats (fast access)
    pub async fn get_core_stats(&self, player_id: &str) -> Result<CoreStats, StorageError> {
        let cache = self
            .memory_cache
            .get(player_id)
            .ok_or_else(|| StorageError::NotAvailable("Player analytics not loaded".to_string()))?;

        Ok(cache.core_stats.clone())
    }

    /// Checks if data needs refresh
    pub async fn needs_refresh(&self, player_id: &str) -> Result<bool, StorageError> {
        if let Some(cache) = self.memory_cache.get(player_id) {
            Ok(cache.needs_refresh(self.config.max_cache_age_hours))
        } else {
            Ok(true) // No cache means refresh needed
        }
    }

    /// Clears all data for a player
    pub async fn clear_player_data(&self, player_id: &str) -> Result<(), StorageError> {
        self.storage.clear_player_data(player_id).await
    }

    /// Gets storage statistics
    pub async fn get_storage_stats(&self) -> Result<StorageStats, StorageError> {
        self.storage.get_storage_stats().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_key_conversion() {
        let key = StorageKey::AnalyticsCache("player123".to_string());
        assert_eq!(key.to_string(), "analytics_cache_player123");
    }

    #[test]
    fn test_storage_config_default() {
        let config = StorageConfig::default();
        assert_eq!(config.max_cache_age_hours, 24);
        assert_eq!(config.max_cache_size_mb, 50);
        assert!(config.use_indexed_db);
    }
}
