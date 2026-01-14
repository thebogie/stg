use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use web_sys::console;

/// Cache entry with expiration
#[derive(Clone)]
pub struct CacheEntry<T: Clone> {
    data: T,
    expires_at: Instant,
}

impl<T: Clone> CacheEntry<T> {
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

/// Request cache for deduplicating API calls
pub struct RequestCache {
    cache: Arc<Mutex<HashMap<String, CacheEntry<String>>>>,
    ttl: Duration,
}

impl RequestCache {
    /// Creates a new request cache with default TTL
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            ttl,
        }
    }

    /// Creates a new request cache with 5 minute default TTL
    pub fn new_default() -> Self {
        Self::new(Duration::from_secs(300)) // 5 minutes
    }

    /// Gets a cached response or fetches it if not cached
    pub async fn get_or_fetch<F, Fut>(&self, key: &str, fetcher: F) -> Result<String, String>
    where
        F: FnOnce() -> Fut + 'static,
        Fut: Future<Output = Result<String, String>> + 'static,
    {
        // Check cache first
        if let Some(entry) = self.get(key) {
            if !entry.is_expired() {
                console::log_1(&format!("Cache hit for key: {}", key).into());
                return Ok(entry.data);
            }
        }

        // Fetch and cache
        console::log_1(&format!("Cache miss for key: {}, fetching...", key).into());
        let result = fetcher().await?;
        self.set(key.to_string(), result.clone());
        Ok(result)
    }

    /// Gets a value from cache
    pub fn get(&self, key: &str) -> Option<CacheEntry<String>> {
        let cache = self.cache.lock().unwrap();
        cache.get(key).cloned()
    }

    /// Sets a value in cache with default TTL
    pub fn set(&self, key: String, value: String) {
        let entry = CacheEntry::new(value, self.ttl);
        let mut cache = self.cache.lock().unwrap();
        cache.insert(key, entry);
    }

    /// Sets a value in cache with custom TTL
    pub fn set_with_ttl(&self, key: String, value: String, ttl: Duration) {
        let entry = CacheEntry::new(value, ttl);
        let mut cache = self.cache.lock().unwrap();
        cache.insert(key, entry);
    }

    /// Removes a value from cache
    pub fn remove(&self, key: &str) {
        let mut cache = self.cache.lock().unwrap();
        cache.remove(key);
    }

    /// Clears all expired entries
    pub fn cleanup(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.retain(|_, entry| !entry.is_expired());
    }

    /// Gets cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.lock().unwrap();
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
    pub fn invalidate_pattern(&self, pattern: &str) {
        let mut cache = self.cache.lock().unwrap();
        cache.retain(|key, _| !key.contains(pattern));
    }

    /// Invalidate all cache entries
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub expired_entries: usize,
}

pub static REQUEST_CACHE: LazyLock<RequestCache> = LazyLock::new(|| RequestCache::new_default());

/// Helper function to get or fetch data with caching
pub async fn cached_request<F, Fut>(key: &str, fetcher: F) -> Result<String, String>
where
    F: FnOnce() -> Fut + 'static,
    Fut: Future<Output = Result<String, String>> + 'static,
{
    REQUEST_CACHE.get_or_fetch(key, fetcher).await
}

/// Helper function to invalidate cache for a specific pattern
pub fn invalidate_cache_pattern(pattern: &str) {
    REQUEST_CACHE.invalidate_pattern(pattern);
}

/// Helper function to get cache statistics
pub fn get_cache_stats() -> CacheStats {
    REQUEST_CACHE.stats()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry::new("test_value".to_string(), Duration::from_millis(100));
        assert!(!entry.is_expired());

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));
        assert!(entry.is_expired());
    }

    #[test]
    fn test_cache_basic_operations() {
        let cache = RequestCache::new(Duration::from_secs(60));

        // Test set and get
        cache.set("test_key".to_string(), "test_value".to_string());
        let entry = cache.get("test_key").unwrap();
        assert_eq!(entry.data, "test_value");
        assert!(!entry.is_expired());

        // Test remove
        cache.remove("test_key");
        assert!(cache.get("test_key").is_none());
    }
}
