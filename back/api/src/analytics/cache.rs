use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use log::warn;

/// Cache entry with expiration
#[derive(Clone)]
struct CacheEntry<T> {
    data: T,
    expires_at: Instant,
}

impl<T> CacheEntry<T> {
    fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            expires_at: Instant::now() + ttl,
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// In-memory cache for analytics data
#[derive(Clone)]
pub struct AnalyticsCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry<String>>>>,
    default_ttl: Duration,
}

impl AnalyticsCache {
    /// Creates a new analytics cache with default TTL
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
        }
    }

    /// Creates a new analytics cache with 5 minute default TTL
    pub fn new_default() -> Self {
        Self::new(Duration::from_secs(300)) // 5 minutes
    }

    /// Get a value from cache
    pub async fn get(&self, key: &str) -> Option<String> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(key) {
            if !entry.is_expired() {
                return Some(entry.data.clone());
            }
        }
        None
    }

    /// Set a value in cache with default TTL
    pub async fn set(&self, key: String, value: String) {
        let entry = CacheEntry::new(value, self.default_ttl);
        let mut cache = self.cache.write().await;
        cache.insert(key, entry);
    }

    /// Set a value in cache with custom TTL
    pub async fn set_with_ttl(&self, key: String, value: String, ttl: Duration) {
        let entry = CacheEntry::new(value, ttl);
        let mut cache = self.cache.write().await;
        cache.insert(key, entry);
    }

    /// Remove a value from cache
    pub async fn remove(&self, key: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(key);
    }

    /// Clear all expired entries
    pub async fn cleanup(&self) {
        let mut cache = self.cache.write().await;
        cache.retain(|_, entry| !entry.is_expired());
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let total_entries = cache.len();
        let expired_entries = cache.values().filter(|entry| entry.is_expired()).count();
        let valid_entries = total_entries - expired_entries;

        CacheStats {
            total_entries,
            valid_entries,
            expired_entries,
        }
    }

    /// Invalidate cache entries matching a pattern
    pub async fn invalidate_pattern(&self, pattern: &str) {
        let mut cache = self.cache.write().await;
        cache.retain(|key, _| !key.contains(pattern));
    }

    /// Invalidate all cache entries
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

/// Redis-backed analytics cache (same API as AnalyticsCache; survives restarts, shared across instances).
#[derive(Clone)]
pub struct RedisAnalyticsCache {
    client: Arc<redis::Client>,
    key_prefix: String,
}

impl RedisAnalyticsCache {
    pub fn new(client: redis::Client) -> Self {
        Self {
            client: Arc::new(client),
            key_prefix: "analytics:".to_string(),
        }
    }

    fn full_key(&self, key: &str) -> String {
        if key.starts_with(&self.key_prefix) {
            key.to_string()
        } else {
            format!("{}{}", self.key_prefix, key)
        }
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let full_key = self.full_key(key);
        let mut conn = match self.client.get_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                warn!("Redis analytics cache get connection: {}", e);
                return None;
            }
        };
        match redis::cmd("GET")
            .arg(&full_key)
            .query_async::<_, Option<String>>(&mut conn)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                warn!("Redis analytics cache GET {}: {}", full_key, e);
                None
            }
        }
    }

    pub async fn set(&self, key: String, value: String) {
        self.set_with_ttl(key, value, Duration::from_secs(300)).await;
    }

    pub async fn set_with_ttl(&self, key: String, value: String, ttl: Duration) {
        let full_key = self.full_key(&key);
        let mut conn = match self.client.get_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                warn!("Redis analytics cache set connection: {}", e);
                return;
            }
        };
        let ttl_secs = ttl.as_secs() as usize;
        if let Err(e) = redis::cmd("SETEX")
            .arg(&full_key)
            .arg(ttl_secs)
            .arg(&value)
            .query_async::<_, ()>(&mut conn)
            .await
        {
            warn!("Redis analytics cache SETEX {}: {}", full_key, e);
        }
    }

    pub async fn remove(&self, key: &str) {
        let full_key = self.full_key(key);
        let mut conn = match self.client.get_async_connection().await {
            Ok(c) => c,
            Err(_) => return,
        };
        let _ = redis::cmd("DEL").arg(&full_key).query_async::<_, ()>(&mut conn).await;
    }

    pub async fn cleanup(&self) {
        // Redis TTL handles expiry; no-op
    }

    pub async fn stats(&self) -> CacheStats {
        // Avoid expensive SCAN; return placeholder
        CacheStats {
            total_entries: 0,
            valid_entries: 0,
            expired_entries: 0,
        }
    }

    pub async fn invalidate_pattern(&self, pattern: &str) {
        let match_pattern = if pattern.is_empty() {
            format!("{}*", self.key_prefix)
        } else {
            format!("{}*{}*", self.key_prefix, pattern)
        };
        let mut cursor: usize = 0;
        loop {
            let mut conn = match self.client.get_async_connection().await {
                Ok(c) => c,
                Err(_) => break,
            };
            let result: (usize, Vec<String>) = match redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&match_pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
            {
                Ok(r) => r,
                Err(_) => break,
            };
            cursor = result.0;
            if !result.1.is_empty() {
                let _ = redis::cmd("DEL").arg(&result.1).query_async::<_, ()>(&mut conn).await;
            }
            if cursor == 0 {
                break;
            }
        }
    }

    pub async fn clear(&self) {
        self.invalidate_pattern("").await;
    }
}

/// Analytics cache: in-memory (default) or Redis-backed.
#[derive(Clone)]
pub enum AnalyticsCacheKind {
    Memory(AnalyticsCache),
    Redis(RedisAnalyticsCache),
}

impl AnalyticsCacheKind {
    pub async fn get(&self, key: &str) -> Option<String> {
        match self {
            AnalyticsCacheKind::Memory(c) => c.get(key).await,
            AnalyticsCacheKind::Redis(c) => c.get(key).await,
        }
    }

    pub async fn set(&self, key: String, value: String) {
        match self {
            AnalyticsCacheKind::Memory(c) => c.set(key, value).await,
            AnalyticsCacheKind::Redis(c) => c.set(key, value).await,
        }
    }

    pub async fn set_with_ttl(&self, key: String, value: String, ttl: Duration) {
        match self {
            AnalyticsCacheKind::Memory(c) => c.set_with_ttl(key, value, ttl).await,
            AnalyticsCacheKind::Redis(c) => c.set_with_ttl(key, value, ttl).await,
        }
    }

    pub async fn remove(&self, key: &str) {
        match self {
            AnalyticsCacheKind::Memory(c) => c.remove(key).await,
            AnalyticsCacheKind::Redis(c) => c.remove(key).await,
        }
    }

    pub async fn cleanup(&self) {
        match self {
            AnalyticsCacheKind::Memory(c) => c.cleanup().await,
            AnalyticsCacheKind::Redis(c) => c.cleanup().await,
        }
    }

    pub async fn stats(&self) -> CacheStats {
        match self {
            AnalyticsCacheKind::Memory(c) => c.stats().await,
            AnalyticsCacheKind::Redis(c) => c.stats().await,
        }
    }

    pub async fn invalidate_pattern(&self, pattern: &str) {
        match self {
            AnalyticsCacheKind::Memory(c) => c.invalidate_pattern(pattern).await,
            AnalyticsCacheKind::Redis(c) => c.invalidate_pattern(pattern).await,
        }
    }

    pub async fn clear(&self) {
        match self {
            AnalyticsCacheKind::Memory(c) => c.clear().await,
            AnalyticsCacheKind::Redis(c) => c.clear().await,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub expired_entries: usize,
}

/// Cache keys for analytics data
pub struct CacheKeys;

impl CacheKeys {
    pub fn platform_stats() -> String {
        // Bump this when platform stats calculation changes (prevents stale cached totals).
        "analytics:platform:stats:v2".to_string()
    }

    pub fn leaderboard(category: &str, time_period: &str, limit: i32, offset: i32) -> String {
        format!("analytics:leaderboard:{}:{}:{}:{}", category, time_period, limit, offset)
    }

    pub fn player_stats(player_id: &str) -> String {
        format!("analytics:player:{}:stats", player_id)
    }

    pub fn player_achievements(player_id: &str) -> String {
        format!("analytics:player:{}:achievements", player_id)
    }

    pub fn player_rankings(player_id: &str) -> String {
        format!("analytics:player:{}:rankings", player_id)
    }

    /// Generate cache key for players who beat me
    pub fn players_who_beat_me(player_id: &str) -> String {
        format!("players_who_beat_me:{}", player_id)
    }

    /// Generate cache key for players I beat
    pub fn players_i_beat(player_id: &str) -> String {
        format!("players_i_beat:{}", player_id)
    }

    /// Generate cache key for my game performance
    pub fn my_game_performance(player_id: &str) -> String {
        format!("my_game_performance:{}", player_id)
    }

    /// Generate cache key for head-to-head record
    pub fn head_to_head_record(player_id: &str, opponent_id: &str) -> String {
        format!("head_to_head:{}:vs:{}", player_id, opponent_id)
    }

    /// Generate cache key for my performance trends
    pub fn my_performance_trends(player_id: &str) -> String {
        format!("my_performance_trends:{}", player_id)
    }

    pub fn contest_stats(contest_id: &str) -> String {
        format!("analytics:contest:{}:stats", contest_id)
    }

    pub fn contest_trends(months: i32) -> String {
        format!("analytics:contest:trends:{}", months)
    }

    pub fn recent_contests(limit: i32) -> String {
        format!("analytics:contest:recent:{}", limit)
    }

    /// Profile bundle (all tab data) per player; short TTL for fast repeat loads.
    pub fn profile_bundle(player_id: &str) -> String {
        // Version suffix invalidates old bundle cache entries (e.g. after DB re-import / schema changes).
        format!("analytics:profile_bundle:v2:{}", player_id)
    }

    /// Profile summary (display + stats only) for fast first paint.
    /// Version suffix invalidates old cache entries (e.g. after DB re-import / stats query changes).
    pub fn profile_summary(player_id: &str) -> String {
        format!("analytics:profile_summary:v3:{}", player_id)
    }

    /// Profile opponents (Nemesis + Owned) for fast tab paint; also primed when bundle is built.
    pub fn profile_opponents(player_id: &str) -> String {
        format!("analytics:profile_opponents:{}", player_id)
    }
}

/// Cache TTL configuration
#[derive(Debug, Clone)]
pub struct CacheTTL {
    pub platform_stats: Duration,
    pub leaderboard: Duration,
    pub player_stats: Duration,
    pub player_achievements: Duration,
    pub player_rankings: Duration,
    pub contest_stats: Duration,
    pub contest_trends: Duration,
    pub recent_contests: Duration,
    pub player_opponents: Duration,
    pub head_to_head: Duration,
    pub player_trends: Duration,
    pub profile_bundle: Duration,
}

impl CacheTTL {
    pub fn new() -> Self {
        Self {
            platform_stats: Duration::from_secs(5 * 60), // 5 minutes
            leaderboard: Duration::from_secs(10 * 60),   // 10 minutes
            player_stats: Duration::from_secs(15 * 60),  // 15 minutes
            player_achievements: Duration::from_secs(30 * 60), // 30 minutes
            player_rankings: Duration::from_secs(20 * 60), // 20 minutes
            contest_stats: Duration::from_secs(30 * 60), // 30 minutes
            contest_trends: Duration::from_secs(60 * 60), // 1 hour
            recent_contests: Duration::from_secs(5 * 60), // 5 minutes
            player_opponents: Duration::from_secs(15 * 60), // 15 minutes
            head_to_head: Duration::from_secs(10 * 60),  // 10 minutes
            player_trends: Duration::from_secs(30 * 60), // 30 minutes
            profile_bundle: Duration::from_secs(60),    // 1 minute
        }
    }

    // Convenience methods for accessing TTLs
    pub fn platform_stats() -> Duration {
        Duration::from_secs(5 * 60)
    }
    pub fn leaderboard() -> Duration {
        Duration::from_secs(10 * 60)
    }
    pub fn player_stats() -> Duration {
        Duration::from_secs(15 * 60)
    }
    pub fn player_achievements() -> Duration {
        Duration::from_secs(30 * 60)
    }
    pub fn player_rankings() -> Duration {
        Duration::from_secs(20 * 60)
    }
    pub fn contest_stats() -> Duration {
        Duration::from_secs(30 * 60)
    }
    pub fn contest_trends() -> Duration {
        Duration::from_secs(60 * 60)
    }
    pub fn recent_contests() -> Duration {
        Duration::from_secs(5 * 60)
    }
    pub fn player_opponents() -> Duration {
        Duration::from_secs(15 * 60)
    }
    pub fn head_to_head() -> Duration {
        Duration::from_secs(10 * 60)
    }
    pub fn player_trends() -> Duration {
        Duration::from_secs(30 * 60)
    }
    pub fn profile_bundle() -> Duration {
        Duration::from_secs(60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = AnalyticsCache::new(Duration::from_secs(1));

        // Test set and get
        cache
            .set("test_key".to_string(), "test_value".to_string())
            .await;
        assert_eq!(cache.get("test_key").await, Some("test_value".to_string()));

        // Test expiration
        tokio::time::sleep(Duration::from_millis(1100)).await;
        assert_eq!(cache.get("test_key").await, None);
    }

    #[tokio::test]
    async fn test_cache_cleanup() {
        let cache = AnalyticsCache::new(Duration::from_millis(100));

        cache.set("key1".to_string(), "value1".to_string()).await;
        cache.set("key2".to_string(), "value2".to_string()).await;

        tokio::time::sleep(Duration::from_millis(150)).await;
        cache.cleanup().await;

        let stats = cache.stats().await;
        assert_eq!(stats.valid_entries, 0);
        assert_eq!(stats.expired_entries, 0);
    }

    #[tokio::test]
    async fn test_cache_pattern_invalidation() {
        let cache = AnalyticsCache::new(Duration::from_secs(60));

        cache
            .set(
                "analytics:player:123:stats".to_string(),
                "value1".to_string(),
            )
            .await;
        cache
            .set(
                "analytics:player:456:stats".to_string(),
                "value2".to_string(),
            )
            .await;
        cache
            .set(
                "analytics:contest:789:stats".to_string(),
                "value3".to_string(),
            )
            .await;

        cache.invalidate_pattern("player").await;

        assert_eq!(cache.get("analytics:player:123:stats").await, None);
        assert_eq!(cache.get("analytics:player:456:stats").await, None);
        assert_eq!(
            cache.get("analytics:contest:789:stats").await,
            Some("value3".to_string())
        );
    }
}
