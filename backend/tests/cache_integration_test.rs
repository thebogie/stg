//! Integration tests for Redis cache with repositories
//!
//! These tests require a running Redis instance.
//! Set REDIS_URL environment variable to override default (redis://127.0.0.1:6379/)

use backend::cache::{CacheKeys, CacheTTL, RedisCache};
use std::sync::Arc;
use std::time::Duration;
use tokio;

#[tokio::test]
async fn test_game_cache_integration() {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
    let client = redis::Client::open(redis_url).expect("Failed to create Redis client");

    let cache = Arc::new(RedisCache::new(
        client,
        "test:cache:game".to_string(),
        CacheTTL::game(),
    ));

    // Test game cache keys
    let game_id = "game/123";
    let cache_key = CacheKeys::game(game_id);

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
    struct TestGame {
        id: String,
        name: String,
    }

    let test_game = TestGame {
        id: game_id.to_string(),
        name: "Test Game".to_string(),
    };

    // Cache should be empty initially
    let result = cache.get::<TestGame>(&cache_key).await.unwrap();
    assert_eq!(result, None);

    // Set in cache
    cache
        .set_with_ttl(&cache_key, &test_game, CacheTTL::game())
        .await
        .unwrap();

    // Retrieve from cache
    let cached = cache.get::<TestGame>(&cache_key).await.unwrap();
    assert_eq!(cached, Some(test_game.clone()));

    // Test cache invalidation
    cache.delete(&cache_key).await.unwrap();
    let result = cache.get::<TestGame>(&cache_key).await.unwrap();
    assert_eq!(result, None);

    // Cleanup
    cache.clear().await.unwrap();
}

#[tokio::test]
async fn test_venue_cache_integration() {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
    let client = redis::Client::open(redis_url).expect("Failed to create Redis client");

    let cache = Arc::new(RedisCache::new(
        client,
        "test:cache:venue".to_string(),
        CacheTTL::venue(),
    ));

    let venue_id = "venue/456";
    let cache_key = CacheKeys::venue(venue_id);

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
    struct TestVenue {
        id: String,
        name: String,
    }

    let test_venue = TestVenue {
        id: venue_id.to_string(),
        name: "Test Venue".to_string(),
    };

    // Test venue caching
    cache
        .set_with_ttl(&cache_key, &test_venue, CacheTTL::venue())
        .await
        .unwrap();
    let cached = cache.get::<TestVenue>(&cache_key).await.unwrap();
    assert_eq!(cached, Some(test_venue.clone()));

    // Test search cache invalidation
    let search_key = CacheKeys::venue_search("test query");
    cache
        .set_with_ttl(
            &search_key,
            &vec![test_venue.clone()],
            CacheTTL::venue_search(),
        )
        .await
        .unwrap();

    // Invalidate search pattern
    let deleted = cache.invalidate_pattern("venues:search:").await.unwrap();
    assert!(deleted >= 1);

    // Search cache should be gone
    assert_eq!(
        cache.get::<Vec<TestVenue>>(&search_key).await.unwrap(),
        None
    );

    // Cleanup
    cache.clear().await.unwrap();
}

#[tokio::test]
async fn test_player_cache_integration() {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
    let client = redis::Client::open(redis_url).expect("Failed to create Redis client");

    let cache = Arc::new(RedisCache::new(
        client,
        "test:cache:player".to_string(),
        CacheTTL::player(),
    ));

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
    struct TestPlayer {
        id: String,
        email: String,
        handle: String,
    }

    let test_player = TestPlayer {
        id: "player/789".to_string(),
        email: "test@example.com".to_string(),
        handle: "player1".to_string(),
    };

    // Test caching by different keys
    cache
        .set_with_ttl(
            &CacheKeys::player(&test_player.id),
            &test_player,
            CacheTTL::player(),
        )
        .await
        .unwrap();

    cache
        .set_with_ttl(
            &CacheKeys::player_by_email(&test_player.email),
            &test_player,
            CacheTTL::player(),
        )
        .await
        .unwrap();

    cache
        .set_with_ttl(
            &CacheKeys::player_by_handle(&test_player.handle),
            &test_player,
            CacheTTL::player(),
        )
        .await
        .unwrap();

    // Verify all lookups work
    assert_eq!(
        cache
            .get::<TestPlayer>(&CacheKeys::player(&test_player.id))
            .await
            .unwrap(),
        Some(test_player.clone())
    );
    assert_eq!(
        cache
            .get::<TestPlayer>(&CacheKeys::player_by_email(&test_player.email))
            .await
            .unwrap(),
        Some(test_player.clone())
    );
    assert_eq!(
        cache
            .get::<TestPlayer>(&CacheKeys::player_by_handle(&test_player.handle))
            .await
            .unwrap(),
        Some(test_player.clone())
    );

    // Test cache invalidation on update - should invalidate all related keys
    cache
        .delete(&CacheKeys::player(&test_player.id))
        .await
        .unwrap();
    cache
        .delete(&CacheKeys::player_by_email(&test_player.email))
        .await
        .unwrap();
    cache
        .delete(&CacheKeys::player_by_handle(&test_player.handle))
        .await
        .unwrap();

    // All should be gone
    assert_eq!(
        cache
            .get::<TestPlayer>(&CacheKeys::player(&test_player.id))
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        cache
            .get::<TestPlayer>(&CacheKeys::player_by_email(&test_player.email))
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        cache
            .get::<TestPlayer>(&CacheKeys::player_by_handle(&test_player.handle))
            .await
            .unwrap(),
        None
    );

    // Cleanup
    cache.clear().await.unwrap();
}

#[tokio::test]
async fn test_cache_ttl_respect() {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
    let client = redis::Client::open(redis_url).expect("Failed to create Redis client");

    let cache = Arc::new(RedisCache::new(
        client,
        "test:cache:ttl_test".to_string(),
        Duration::from_secs(5),
    ));

    let test_value = "short_ttl".to_string();

    // Set with short TTL (1 second for more reliable testing)
    cache
        .set_with_ttl("ttl_test", &test_value, Duration::from_secs(1))
        .await
        .unwrap();

    // Should be available immediately
    assert_eq!(
        cache.get::<String>("ttl_test").await.unwrap(),
        Some(test_value.clone()),
        "Value should be cached immediately after setting"
    );

    // Wait for expiration (add buffer for timing variations)
    // Redis TTL expiration can be slightly delayed, so wait longer than the TTL
    // Use 3000ms wait for 1000ms TTL to account for Redis's lazy expiration
    tokio::time::sleep(Duration::from_millis(3000)).await;

    // Should be expired - retry a few times if needed (Redis lazy expiration)
    let mut result = cache.get::<String>("ttl_test").await.unwrap();
    let mut retries = 0;
    while result.is_some() && retries < 3 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        result = cache.get::<String>("ttl_test").await.unwrap();
        retries += 1;
    }
    
    assert_eq!(
        result, None,
        "Value should be expired after TTL (waited 3000ms+ for 1000ms TTL, retried {} times)",
        retries
    );

    // Cleanup
    cache.clear().await.unwrap();
}

#[tokio::test]
async fn test_cache_pattern_invalidation_complex() {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
    let client = redis::Client::open(redis_url).expect("Failed to create Redis client");

    let cache = Arc::new(RedisCache::new(
        client,
        "test:cache:pattern_complex".to_string(),
        Duration::from_secs(60),
    ));

    // Create a scenario like game search results
    for i in 1..=5 {
        let key = format!("games:search:query{}", i);
        cache.set(&key, &format!("result_{}", i)).await.unwrap();
    }

    // Create other keys that shouldn't be affected
    cache
        .set("game:123", &"game_data".to_string())
        .await
        .unwrap();
    cache
        .set("games:list", &"list_data".to_string())
        .await
        .unwrap();

    // Verify all exist
    for i in 1..=5 {
        let key = format!("games:search:query{}", i);
        assert!(cache.get::<String>(&key).await.unwrap().is_some());
    }
    assert!(cache.get::<String>("game:123").await.unwrap().is_some());
    assert!(cache.get::<String>("games:list").await.unwrap().is_some());

    // Invalidate only search pattern
    let deleted = cache.invalidate_pattern("games:search:").await.unwrap();
    assert_eq!(deleted, 5);

    // Search keys should be gone
    for i in 1..=5 {
        let key = format!("games:search:query{}", i);
        assert_eq!(cache.get::<String>(&key).await.unwrap(), None);
    }

    // Other keys should still exist
    assert_eq!(
        cache.get::<String>("game:123").await.unwrap(),
        Some("game_data".to_string())
    );
    assert_eq!(
        cache.get::<String>("games:list").await.unwrap(),
        Some("list_data".to_string())
    );

    // Cleanup
    cache.clear().await.unwrap();
}
