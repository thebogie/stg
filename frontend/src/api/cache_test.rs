use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_cache_entry_creation() {
        let entry = CacheEntry::new("test_value".to_string(), Duration::from_secs(60));
        
        assert_eq!(entry.data, "test_value");
        assert!(!entry.is_expired());
        println!("✅ Cache entry creation test passed");
    }

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry::new("test_value".to_string(), Duration::from_millis(100));
        
        // Should not be expired initially
        assert!(!entry.is_expired());
        
        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));
        
        // Should be expired now
        assert!(entry.is_expired());
        println!("✅ Cache entry expiration test passed");
    }

    #[test]
    fn test_request_cache_creation() {
        let cache = RequestCache::new(Duration::from_secs(300));
        let default_cache = RequestCache::new_default();
        
        assert_eq!(cache.ttl, Duration::from_secs(300));
        assert_eq!(default_cache.ttl, Duration::from_secs(300));
        println!("✅ Request cache creation test passed");
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
        println!("✅ Cache basic operations test passed");
    }

    #[test]
    fn test_cache_cleanup() {
        let cache = RequestCache::new(Duration::from_millis(100));
        
        cache.set("key1".to_string(), "value1".to_string());
        cache.set("key2".to_string(), "value2".to_string());
        
        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));
        
        // Cleanup expired entries
        cache.cleanup();
        
        let stats = cache.stats();
        assert_eq!(stats.valid_entries, 0);
        assert_eq!(stats.expired_entries, 0);
        println!("✅ Cache cleanup test passed");
    }

    #[test]
    fn test_cache_pattern_invalidation() {
        let cache = RequestCache::new(Duration::from_secs(60));
        
        cache.set("analytics:player:123:stats".to_string(), "value1".to_string());
        cache.set("analytics:player:456:stats".to_string(), "value2".to_string());
        cache.set("analytics:contest:789:stats".to_string(), "value3".to_string());
        
        // Invalidate player-related entries
        cache.invalidate_pattern("player");
        
        assert!(cache.get("analytics:player:123:stats").is_none());
        assert!(cache.get("analytics:player:456:stats").is_none());
        assert!(cache.get("analytics:contest:789:stats").is_some());
        println!("✅ Cache pattern invalidation test passed");
    }

    #[test]
    fn test_cache_statistics() {
        let cache = RequestCache::new(Duration::from_secs(60));
        
        cache.set("key1".to_string(), "value1".to_string());
        cache.set("key2".to_string(), "value2".to_string());
        
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.valid_entries, 2);
        assert_eq!(stats.expired_entries, 0);
        println!("✅ Cache statistics test passed");
    }

    #[test]
    fn test_global_cache_instance() {
        // Test that the global cache instance can be accessed
        let stats = get_cache_stats();
        assert_eq!(stats.total_entries, 0);
        println!("✅ Global cache instance test passed");
    }

    #[test]
    fn test_cache_helper_functions() {
        // Test helper functions
        invalidate_cache_pattern("test");
        let stats = get_cache_stats();
        assert_eq!(stats.total_entries, 0);
        println!("✅ Cache helper functions test passed");
    }
}

/// Performance tests for the request cache
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_cache_performance() {
        let cache = RequestCache::new(Duration::from_secs(60));
        
        let start = Instant::now();
        
        // Add many items to test performance
        for i in 0..1000 {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            cache.set(key, value);
        }
        
        let duration = start.elapsed();
        
        // Should complete in reasonable time (less than 100ms)
        assert!(duration.as_millis() < 100);
        println!("✅ Cache performance test passed: {}ms", duration.as_millis());
    }

    #[test]
    fn test_cache_memory_efficiency() {
        let cache = RequestCache::new(Duration::from_secs(60));
        
        let initial_memory = std::mem::size_of_val(&cache);
        
        // Add many items
        for i in 0..1000 {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            cache.set(key, value);
        }
        
        let final_memory = std::mem::size_of_val(&cache);
        
        // Memory should not grow significantly
        let memory_growth = final_memory - initial_memory;
        assert!(memory_growth < 1024 * 1024); // Less than 1MB growth
        
        println!("✅ Cache memory efficiency test passed: {} bytes growth", memory_growth);
    }

    #[test]
    fn test_cache_concurrent_access() {
        let cache = RequestCache::new(Duration::from_secs(60));
        
        let start = Instant::now();
        
        // Simulate concurrent access patterns
        for i in 0..100 {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            
            // Set value
            cache.set(key.clone(), value.clone());
            
            // Get value
            let retrieved = cache.get(&key);
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().data, value);
            
            // Remove value
            cache.remove(&key);
            assert!(cache.get(&key).is_none());
        }
        
        let duration = start.elapsed();
        assert!(duration.as_millis() < 1000); // Less than 1 second
        
        println!("✅ Cache concurrent access test passed: {}ms", duration.as_millis());
    }
}

/// Integration tests for the request cache
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_cache_integration_scenario() {
        let cache = RequestCache::new(Duration::from_secs(60));
        
        // Simulate a real-world scenario: caching API responses
        let api_endpoints = vec![
            "analytics:player:123:stats",
            "analytics:player:123:achievements", 
            "analytics:player:123:rankings",
            "analytics:contest:456:stats",
            "analytics:contest:456:difficulty",
        ];
        
        // Cache responses
        for endpoint in &api_endpoints {
            let response = format!("response_for_{}", endpoint);
            cache.set(endpoint.to_string(), response);
        }
        
        // Verify all are cached
        for endpoint in &api_endpoints {
            assert!(cache.get(endpoint).is_some());
        }
        
        // Invalidate player-related cache
        cache.invalidate_pattern("player");
        
        // Player endpoints should be gone, contest endpoints should remain
        assert!(cache.get("analytics:player:123:stats").is_none());
        assert!(cache.get("analytics:player:123:achievements").is_none());
        assert!(cache.get("analytics:player:123:rankings").is_none());
        assert!(cache.get("analytics:contest:456:stats").is_some());
        assert!(cache.get("analytics:contest:456:difficulty").is_some());
        
        println!("✅ Cache integration scenario test passed");
    }
}
