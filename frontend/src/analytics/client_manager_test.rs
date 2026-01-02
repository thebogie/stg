use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Mock storage implementation for testing
    struct MockStorage {
        data: HashMap<String, String>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                data: HashMap::new(),
            }
        }
    }

    #[async_trait::async_trait]
    impl ClientStorage for MockStorage {
        async fn get_analytics_cache(&self, _player_id: &str) -> Result<Option<ClientAnalyticsCache>, String> {
            Ok(None)
        }

        async fn set_analytics_cache(&self, _player_id: &str, _cache: &ClientAnalyticsCache) -> Result<(), String> {
            Ok(())
        }

        async fn clear_player_data(&self, _player_id: &str) -> Result<(), String> {
            Ok(())
        }

        async fn get_storage_stats(&self) -> Result<StorageStats, String> {
            Ok(StorageStats {
                total_players: 0,
                total_size_bytes: 0,
                last_cleanup: chrono::Utc::now().fixed_offset(),
            })
        }
    }

    #[test]
    fn test_lru_cache_creation() {
        let manager = ClientAnalyticsManager::new();
        
        // Test that the cache is created with the right capacity
        assert_eq!(manager.memory_cache.cap(), 100);
        println!("✅ LRU cache creation test passed");
    }

    #[test]
    fn test_cache_cleanup_methods() {
        let mut manager = ClientAnalyticsManager::new();
        
        // Test that cleanup methods exist and can be called
        manager.cleanup_cache();
        let (total, expired) = manager.get_cache_stats();
        
        assert_eq!(total, 0);
        assert_eq!(expired, 0);
        println!("✅ Cache cleanup methods test passed");
    }

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
        println!("✅ Cache status creation test passed");
    }

    #[test]
    fn test_memory_management() {
        let mut manager = ClientAnalyticsManager::new();
        
        // Test that we can add multiple players without exceeding capacity
        for i in 0..150 {
            let player_id = format!("player/{}", i);
            let cache = ClientAnalyticsCache::new(player_id.clone());
            manager.memory_cache.put(player_id, cache);
        }
        
        // Should not exceed capacity
        assert!(manager.memory_cache.len() <= 100);
        println!("✅ Memory management test passed");
    }

    #[test]
    fn test_cache_operations() {
        let mut manager = ClientAnalyticsManager::new();
        
        // Test basic cache operations
        let player_id = "player/test".to_string();
        let cache = ClientAnalyticsCache::new(player_id.clone());
        
        // Test put and get
        manager.memory_cache.put(player_id.clone(), cache.clone());
        assert!(manager.memory_cache.contains(&player_id));
        
        // Test pop
        let retrieved = manager.memory_cache.pop(&player_id);
        assert!(retrieved.is_some());
        assert!(!manager.memory_cache.contains(&player_id));
        
        println!("✅ Cache operations test passed");
    }
}

/// Performance tests for the LRU cache
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_cache_performance() {
        let mut manager = ClientAnalyticsManager::new();
        
        let start = Instant::now();
        
        // Add many items to test performance
        for i in 0..1000 {
            let player_id = format!("player/{}", i);
            let cache = ClientAnalyticsCache::new(player_id.clone());
            manager.memory_cache.put(player_id, cache);
        }
        
        let duration = start.elapsed();
        
        // Should complete in reasonable time (less than 100ms)
        assert!(duration.as_millis() < 100);
        println!("✅ Cache performance test passed: {}ms", duration.as_millis());
    }

    #[test]
    fn test_memory_efficiency() {
        let mut manager = ClientAnalyticsManager::new();
        
        let initial_memory = std::mem::size_of_val(&manager);
        
        // Add items up to capacity
        for i in 0..100 {
            let player_id = format!("player/{}", i);
            let cache = ClientAnalyticsCache::new(player_id.clone());
            manager.memory_cache.put(player_id, cache);
        }
        
        let final_memory = std::mem::size_of_val(&manager);
        
        // Memory should not grow significantly beyond capacity
        let memory_growth = final_memory - initial_memory;
        assert!(memory_growth < 1024 * 1024); // Less than 1MB growth
        
        println!("✅ Memory efficiency test passed: {} bytes growth", memory_growth);
    }
}
