use log::{debug, warn};
use redis::Client as RedisClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Generic Redis-backed cache for any serializable type
#[derive(Clone)]
pub struct RedisCache {
    client: Arc<RedisClient>,
    key_prefix: String,
    default_ttl: Duration,
}

impl RedisCache {
    /// Create a new Redis cache with a key prefix
    pub fn new(client: RedisClient, key_prefix: String, default_ttl: Duration) -> Self {
        Self {
            client: Arc::new(client),
            key_prefix,
            default_ttl,
        }
    }

    /// Get a value from cache
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>, String>
    where
        T: for<'de> Deserialize<'de>,
    {
        let full_key = self.full_key(key);
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| format!("Failed to get Redis connection: {}", e))?;

        match redis::cmd("GET")
            .arg(&full_key)
            .query_async::<_, Option<String>>(&mut conn)
            .await
        {
            Ok(Some(value)) => {
                match serde_json::from_str::<T>(&value) {
                    Ok(deserialized) => {
                        debug!("Cache hit for key: {}", full_key);
                        Ok(Some(deserialized))
                    }
                    Err(e) => {
                        warn!(
                            "Failed to deserialize cached value for key {}: {}",
                            full_key, e
                        );
                        // Delete the corrupted cache entry
                        let _ = redis::cmd("DEL")
                            .arg(&full_key)
                            .query_async::<_, ()>(&mut conn)
                            .await;
                        Ok(None)
                    }
                }
            }
            Ok(None) => {
                debug!("Cache miss for key: {}", full_key);
                Ok(None)
            }
            Err(e) => {
                warn!("Redis GET error for key {}: {}", full_key, e);
                // Don't fail the request if cache fails - just return None
                Ok(None)
            }
        }
    }

    /// Set a value in cache with default TTL
    pub async fn set<T>(&self, key: &str, value: &T) -> Result<(), String>
    where
        T: Serialize,
    {
        self.set_with_ttl(key, value, self.default_ttl).await
    }

    /// Set a value in cache with custom TTL
    pub async fn set_with_ttl<T>(&self, key: &str, value: &T, ttl: Duration) -> Result<(), String>
    where
        T: Serialize,
    {
        let full_key = self.full_key(key);
        let serialized = serde_json::to_string(value)
            .map_err(|e| format!("Failed to serialize value for key {}: {}", full_key, e))?;

        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| format!("Failed to get Redis connection: {}", e))?;

        let ttl_seconds = ttl.as_secs() as usize;
        match redis::cmd("SETEX")
            .arg(&full_key)
            .arg(ttl_seconds)
            .arg(&serialized)
            .query_async::<_, ()>(&mut conn)
            .await
        {
            Ok(_) => {
                debug!("Cached value for key: {} (TTL: {}s)", full_key, ttl_seconds);
                Ok(())
            }
            Err(e) => {
                warn!("Redis SETEX error for key {}: {}", full_key, e);
                // Don't fail the request if cache fails
                Ok(())
            }
        }
    }

    /// Delete a value from cache
    pub async fn delete(&self, key: &str) -> Result<(), String> {
        let full_key = self.full_key(key);
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| format!("Failed to get Redis connection: {}", e))?;

        match redis::cmd("DEL")
            .arg(&full_key)
            .query_async::<_, ()>(&mut conn)
            .await
        {
            Ok(_) => {
                debug!("Deleted cache key: {}", full_key);
                Ok(())
            }
            Err(e) => {
                warn!("Redis DEL error for key {}: {}", full_key, e);
                // Don't fail the request if cache fails
                Ok(())
            }
        }
    }

    /// Delete all keys matching a pattern (uses SCAN for safety)
    pub async fn invalidate_pattern(&self, pattern: &str) -> Result<usize, String> {
        let search_pattern = self.full_key(pattern);
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| format!("Failed to get Redis connection: {}", e))?;

        let mut deleted = 0;
        let mut cursor: usize = 0;
        let scan_count = 100; // Scan in batches

        loop {
            let result: (usize, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&format!("{}*", search_pattern))
                .arg("COUNT")
                .arg(scan_count)
                .query_async(&mut conn)
                .await
                .map_err(|e| format!("Redis SCAN error: {}", e))?;

            cursor = result.0;
            let keys = result.1;

            if !keys.is_empty() {
                let deleted_count: usize = redis::cmd("DEL")
                    .arg(&keys)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| format!("Redis DEL error: {}", e))?;
                deleted += deleted_count;
                debug!(
                    "Invalidated {} keys matching pattern: {}",
                    deleted_count, search_pattern
                );
            }

            // SCAN returns 0 when iteration is complete
            if cursor == 0 {
                break;
            }
        }

        Ok(deleted)
    }

    /// Clear all cache entries with this prefix
    pub async fn clear(&self) -> Result<usize, String> {
        self.invalidate_pattern("").await
    }

    /// Build the full cache key with prefix
    fn full_key(&self, key: &str) -> String {
        if key.is_empty() {
            self.key_prefix.clone()
        } else {
            format!("{}:{}", self.key_prefix, key)
        }
    }
}

/// Cache key generators for different entity types
pub struct CacheKeys;

impl CacheKeys {
    /// Game cache keys
    pub fn game(id: &str) -> String {
        format!("game:{}", id)
    }

    pub fn game_list() -> String {
        "games:list".to_string()
    }

    pub fn game_search(query: &str) -> String {
        format!("games:search:{}", query.to_lowercase())
    }

    /// Venue cache keys
    pub fn venue(id: &str) -> String {
        format!("venue:{}", id)
    }

    pub fn venue_search(query: &str) -> String {
        format!("venues:search:{}", query.to_lowercase())
    }

    /// Player cache keys
    pub fn player(id: &str) -> String {
        format!("player:{}", id)
    }

    pub fn player_by_email(email: &str) -> String {
        format!("player:email:{}", email.to_lowercase())
    }

    pub fn player_by_handle(handle: &str) -> String {
        format!("player:handle:{}", handle.to_lowercase())
    }
}

/// Cache TTL configuration
pub struct CacheTTL;

impl CacheTTL {
    /// Game cache TTLs
    pub fn game() -> Duration {
        Duration::from_secs(60 * 60) // 1 hour - games change rarely
    }

    pub fn game_list() -> Duration {
        Duration::from_secs(10 * 60) // 10 minutes
    }

    pub fn game_search() -> Duration {
        Duration::from_secs(5 * 60) // 5 minutes
    }

    /// Venue cache TTLs
    pub fn venue() -> Duration {
        Duration::from_secs(60 * 60) // 1 hour - venues change rarely
    }

    pub fn venue_search() -> Duration {
        Duration::from_secs(5 * 60) // 5 minutes
    }

    pub fn google_places() -> Duration {
        Duration::from_secs(24 * 60 * 60) // 24 hours - Google Places data changes rarely
    }

    /// Player cache TTLs
    pub fn player() -> Duration {
        Duration::from_secs(15 * 60) // 15 minutes - players update more frequently
    }

    pub fn player_profile() -> Duration {
        Duration::from_secs(15 * 60) // 15 minutes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        assert_eq!(CacheKeys::game("123"), "game:123");
        assert_eq!(CacheKeys::game_list(), "games:list");
        assert_eq!(CacheKeys::venue("venue/456"), "venue:venue/456");
        assert_eq!(CacheKeys::player("player/789"), "player:player/789");
        assert_eq!(
            CacheKeys::player_by_email("test@example.com"),
            "player:email:test@example.com"
        );
        assert_eq!(
            CacheKeys::player_by_handle("player1"),
            "player:handle:player1"
        );
    }

    #[test]
    fn test_ttl_configuration() {
        assert_eq!(CacheTTL::game(), Duration::from_secs(3600));
        assert_eq!(CacheTTL::game_list(), Duration::from_secs(600));
        assert_eq!(CacheTTL::player(), Duration::from_secs(900));
        assert_eq!(CacheTTL::venue(), Duration::from_secs(3600));
        assert_eq!(CacheTTL::google_places(), Duration::from_secs(86400));
    }

    #[test]
    fn test_cache_key_prefix() {
        let key = CacheKeys::game("test-id");
        assert!(key.starts_with("game:"));
    }

    // Integration tests with real Redis (requires Redis running)
    mod integration_tests {
        use super::*;

        // Helper to create a test Redis client
        fn create_test_redis_client() -> redis::Client {
            let redis_url = std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
            redis::Client::open(redis_url).expect("Failed to create Redis client")
        }

        #[tokio::test]
        #[ignore] // Requires Redis - run with integration tests
        async fn test_cache_set_and_get() {
            let client = create_test_redis_client();
            let cache = RedisCache::new(
                client,
                "test:cache:basic".to_string(),
                Duration::from_secs(60),
            );

            #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
            struct TestData {
                id: String,
                value: i32,
            }

            let test_data = TestData {
                id: "123".to_string(),
                value: 42,
            };

            // Test set
            cache.set("test_key", &test_data).await.unwrap();

            // Test get
            let retrieved = cache.get::<TestData>("test_key").await.unwrap();
            assert_eq!(retrieved, Some(test_data));

            // Cleanup
            cache.delete("test_key").await.unwrap();
        }

        #[tokio::test]
        #[ignore] // Requires Redis - run with integration tests
        async fn test_cache_miss() {
            let client = create_test_redis_client();
            let cache = RedisCache::new(
                client,
                "test:cache:miss".to_string(),
                Duration::from_secs(60),
            );

            let result = cache.get::<String>("nonexistent_key").await.unwrap();
            assert_eq!(result, None);
        }

        #[tokio::test]
        #[ignore] // Requires Redis - run with integration tests
        async fn test_cache_ttl_expiration() {
            let client = create_test_redis_client();
            let cache =
                RedisCache::new(client, "test:cache:ttl".to_string(), Duration::from_secs(1));

            let test_value = "expires_soon".to_string();
            cache
                .set_with_ttl("ttl_key", &test_value, Duration::from_secs(1))
                .await
                .unwrap();

            // Should be cached immediately
            let result = cache.get::<String>("ttl_key").await.unwrap();
            assert_eq!(result, Some(test_value.clone()));

            // Wait for expiration - Redis expiration is not guaranteed to be exact
            // Use a longer wait time to account for Redis's lazy expiration mechanism
            // Use 3000ms wait for 1000ms TTL to account for Redis's lazy expiration
            tokio::time::sleep(Duration::from_millis(3000)).await;

            // Should be expired - retry a few times if needed (Redis lazy expiration)
            let mut result = cache.get::<String>("ttl_key").await.unwrap();
            let mut retries = 0;
            while result.is_some() && retries < 3 {
                tokio::time::sleep(Duration::from_millis(500)).await;
                result = cache.get::<String>("ttl_key").await.unwrap();
                retries += 1;
            }

            assert_eq!(
                result, None,
                "Value should be expired after TTL (waited 3000ms+ for 1000ms TTL, retried {} times)",
                retries
            );

            // Cleanup
            cache.delete("ttl_key").await.unwrap();
        }

        #[tokio::test]
        #[ignore] // Requires Redis - run with integration tests
        async fn test_cache_delete() {
            let client = create_test_redis_client();
            let cache = RedisCache::new(
                client,
                "test:cache:delete".to_string(),
                Duration::from_secs(60),
            );

            let test_value = "to_be_deleted".to_string();
            cache.set("delete_key", &test_value).await.unwrap();

            // Verify it exists
            let result = cache.get::<String>("delete_key").await.unwrap();
            assert_eq!(result, Some(test_value));

            // Delete it
            cache.delete("delete_key").await.unwrap();

            // Verify it's gone
            let result = cache.get::<String>("delete_key").await.unwrap();
            assert_eq!(result, None);
        }

        #[tokio::test]
        #[ignore] // Requires Redis - run with integration tests
        async fn test_cache_pattern_invalidation() {
            let client = create_test_redis_client();
            let cache = RedisCache::new(
                client,
                "test:cache:pattern".to_string(),
                Duration::from_secs(60),
            );

            // Set multiple keys with a pattern
            cache
                .set("player:123", &"player1".to_string())
                .await
                .unwrap();
            cache
                .set("player:456", &"player2".to_string())
                .await
                .unwrap();
            cache.set("game:789", &"game1".to_string()).await.unwrap();

            // Verify all exist
            assert_eq!(
                cache.get::<String>("player:123").await.unwrap(),
                Some("player1".to_string())
            );
            assert_eq!(
                cache.get::<String>("player:456").await.unwrap(),
                Some("player2".to_string())
            );
            assert_eq!(
                cache.get::<String>("game:789").await.unwrap(),
                Some("game1".to_string())
            );

            // Invalidate pattern
            let deleted = cache.invalidate_pattern("player:").await.unwrap();
            assert_eq!(deleted, 2);

            // Player keys should be gone
            assert_eq!(cache.get::<String>("player:123").await.unwrap(), None);
            assert_eq!(cache.get::<String>("player:456").await.unwrap(), None);

            // Game key should still exist
            assert_eq!(
                cache.get::<String>("game:789").await.unwrap(),
                Some("game1".to_string())
            );

            // Cleanup
            cache.delete("game:789").await.unwrap();
        }

        #[tokio::test]
        #[ignore] // Requires Redis - run with integration tests
        async fn test_cache_clear() {
            let client = create_test_redis_client();
            let cache = RedisCache::new(
                client,
                "test:cache:clear".to_string(),
                Duration::from_secs(60),
            );

            // Set multiple keys
            cache.set("key1", &"value1".to_string()).await.unwrap();
            cache.set("key2", &"value2".to_string()).await.unwrap();
            cache.set("key3", &"value3".to_string()).await.unwrap();

            // Clear all
            let deleted = cache.clear().await.unwrap();
            assert!(deleted >= 3); // At least 3, might be more if tests ran before

            // Verify all are gone
            assert_eq!(cache.get::<String>("key1").await.unwrap(), None);
            assert_eq!(cache.get::<String>("key2").await.unwrap(), None);
            assert_eq!(cache.get::<String>("key3").await.unwrap(), None);
        }

        #[tokio::test]
        #[ignore] // Requires Redis - run with integration tests
        async fn test_cache_serialization_types() {
            let client = create_test_redis_client();
            let cache = RedisCache::new(
                client,
                "test:cache:types".to_string(),
                Duration::from_secs(60),
            );

            // Test with different types
            cache
                .set("string_key", &"test_string".to_string())
                .await
                .unwrap();
            cache.set("int_key", &42i32).await.unwrap();
            cache.set("vec_key", &vec![1, 2, 3]).await.unwrap();

            #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
            struct ComplexType {
                name: String,
                values: Vec<i32>,
            }
            let complex = ComplexType {
                name: "test".to_string(),
                values: vec![1, 2, 3],
            };
            cache.set("complex_key", &complex).await.unwrap();

            // Retrieve and verify
            assert_eq!(
                cache.get::<String>("string_key").await.unwrap(),
                Some("test_string".to_string())
            );
            assert_eq!(cache.get::<i32>("int_key").await.unwrap(), Some(42));
            assert_eq!(
                cache.get::<Vec<i32>>("vec_key").await.unwrap(),
                Some(vec![1, 2, 3])
            );
            assert_eq!(
                cache.get::<ComplexType>("complex_key").await.unwrap(),
                Some(complex)
            );

            // Cleanup
            cache.delete("string_key").await.unwrap();
            cache.delete("int_key").await.unwrap();
            cache.delete("vec_key").await.unwrap();
            cache.delete("complex_key").await.unwrap();
        }

        #[tokio::test]
        #[ignore] // Requires Redis - run with integration tests
        async fn test_cache_key_prefix_isolation() {
            let client = create_test_redis_client();
            let cache1 = RedisCache::new(
                client.clone(),
                "test:cache:prefix1".to_string(),
                Duration::from_secs(60),
            );
            let cache2 = RedisCache::new(
                client,
                "test:cache:prefix2".to_string(),
                Duration::from_secs(60),
            );

            // Set same key in both caches
            cache1.set("same_key", &"value1".to_string()).await.unwrap();
            cache2.set("same_key", &"value2".to_string()).await.unwrap();

            // They should be isolated
            assert_eq!(
                cache1.get::<String>("same_key").await.unwrap(),
                Some("value1".to_string())
            );
            assert_eq!(
                cache2.get::<String>("same_key").await.unwrap(),
                Some("value2".to_string())
            );

            // Cleanup
            cache1.delete("same_key").await.unwrap();
            cache2.delete("same_key").await.unwrap();
        }

        #[tokio::test]
        #[ignore] // Requires Redis - run with integration tests
        async fn test_cache_deserialization_error_handling() {
            let client = create_test_redis_client();
            let cache = RedisCache::new(
                client,
                "test:cache:error".to_string(),
                Duration::from_secs(60),
            );

            // Manually insert invalid JSON into Redis (bypassing cache.set)
            let mut conn = cache.client.get_async_connection().await.unwrap();
            let full_key = format!("test:cache:error:{}", "corrupted_key");
            redis::cmd("SETEX")
                .arg(&full_key)
                .arg(60)
                .arg("invalid json{")
                .query_async::<_, ()>(&mut conn)
                .await
                .unwrap();

            // Get should handle deserialization error gracefully
            let result = cache.get::<String>("corrupted_key").await.unwrap();
            assert_eq!(result, None); // Should return None, not panic

            // Cleanup
            cache.delete("corrupted_key").await.unwrap();
        }
    }
}
