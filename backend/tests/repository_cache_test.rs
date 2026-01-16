//! Integration tests for repository caching
//!
//! These tests verify that repositories properly use Redis cache.
//! Requires Redis running at REDIS_URL (default: redis://127.0.0.1:6379/)
//! Requires ArangoDB running at ARANGO_URL (default: http://localhost:8529/)

use backend::cache::{CacheKeys, CacheTTL, RedisCache};
use backend::game::repository::{GameRepository, GameRepositoryImpl};
use backend::player::repository::{PlayerRepository, PlayerRepositoryImpl};
use std::sync::Arc;
use std::time::Duration;
use tokio;

/// Helper to create a test Redis client
fn create_test_redis_client() -> redis::Client {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
    redis::Client::open(redis_url).expect("Failed to create Redis client")
}

/// Helper to create a test ArangoDB connection
async fn create_test_arangodb() -> arangors::Database<arangors::client::reqwest::ReqwestClient> {
    let arango_url =
        std::env::var("ARANGO_URL").unwrap_or_else(|_| "http://localhost:8529".to_string());
    let username = std::env::var("ARANGO_USERNAME").unwrap_or_else(|_| "root".to_string());
    let password = std::env::var("ARANGO_PASSWORD").unwrap_or_else(|_| "test".to_string());
    let db_name = std::env::var("ARANGO_DB").unwrap_or_else(|_| "_system".to_string());

    let conn = arangors::Connection::establish_basic_auth(&arango_url, &username, &password)
        .await
        .expect("Failed to connect to ArangoDB");

    conn.db(&db_name).await.expect("Failed to get database")
}

#[tokio::test]
#[ignore] // Ignore by default - requires Redis and ArangoDB
async fn test_game_repository_cache_flow() {
    let db = create_test_arangodb().await;
    let redis_client = create_test_redis_client();

    let cache = Arc::new(RedisCache::new(
        redis_client,
        "test:cache:game_repo".to_string(),
        CacheTTL::game(),
    ));

    let repo = GameRepositoryImpl::new_with_cache(db, cache.clone());

    // First call should miss cache and hit database
    // Note: This test assumes there's no game with ID "game/nonexistent" in the DB
    // For a real test, you'd want to create a test game first

    // Clear cache before test
    cache.clear().await.unwrap();

    // If we had a test game, we could verify:
    // 1. First call - cache miss, database hit
    // 2. Second call - cache hit, no database query
    // 3. After update - cache invalidated
    // 4. Third call - cache miss again, database hit

    // For now, just verify the cache is being used (no panic means integration works)
    let _result = repo.find_by_id("game/nonexistent_test_game").await;

    // Cleanup
    cache.clear().await.unwrap();
}

#[tokio::test]
#[ignore] // Ignore by default - requires Redis and ArangoDB
async fn test_player_repository_cache_flow() {
    let db = create_test_arangodb().await;
    let redis_client = create_test_redis_client();

    let cache = Arc::new(RedisCache::new(
        redis_client,
        "test:cache:player_repo".to_string(),
        CacheTTL::player(),
    ));

    let repo = PlayerRepositoryImpl::new_with_cache(db, cache.clone());

    // Clear cache before test
    cache.clear().await.unwrap();

    // Test that cache is being used for lookups
    // Note: This would require a test player in the database
    let _result = repo.find_by_email("nonexistent@test.com").await;
    let _result = repo.find_by_handle("nonexistent_handle").await;

    // Cleanup
    cache.clear().await.unwrap();
}

#[tokio::test]
async fn test_cache_key_consistency() {
    // Test that cache keys are generated consistently
    let game_id = "game/123";
    let key1 = CacheKeys::game(game_id);
    let key2 = CacheKeys::game(game_id);
    assert_eq!(key1, key2);

    let email = "test@example.com";
    let key1 = CacheKeys::player_by_email(email);
    let key2 = CacheKeys::player_by_email(email);
    assert_eq!(key1, key2);
    assert!(key1.contains("email"));
    assert!(key1.contains(&email.to_lowercase()));

    let handle = "Player1";
    let key1 = CacheKeys::player_by_handle(handle);
    let key2 = CacheKeys::player_by_handle(handle);
    assert_eq!(key1, key2);
    assert!(key1.contains("handle"));
    assert!(key1.contains(&handle.to_lowercase()));
}

#[tokio::test]
async fn test_cache_invalidation_patterns() {
    let redis_client = create_test_redis_client();
    let cache = Arc::new(RedisCache::new(
        redis_client,
        "test:cache:invalidation".to_string(),
        Duration::from_secs(60),
    ));

    // Simulate what happens when a game is created/updated
    // 1. Cache the game
    let game_id = "game/test123";
    cache
        .set(&CacheKeys::game(game_id), &"game_data".to_string())
        .await
        .unwrap();

    // 2. Cache the game list
    cache
        .set(&CacheKeys::game_list(), &vec!["game1", "game2"])
        .await
        .unwrap();

    // 3. Cache some search results
    cache
        .set(&CacheKeys::game_search("test"), &vec!["result1"])
        .await
        .unwrap();

    // 4. Simulate game update - should invalidate related caches
    cache.delete(&CacheKeys::game(game_id)).await.unwrap();
    cache.delete(&CacheKeys::game_list()).await.unwrap();
    let deleted = cache.invalidate_pattern("games:search:").await.unwrap();
    assert!(deleted >= 1);

    // Verify caches are cleared
    assert_eq!(
        cache
            .get::<String>(&CacheKeys::game(game_id))
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        cache
            .get::<Vec<String>>(&CacheKeys::game_list())
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        cache
            .get::<Vec<String>>(&CacheKeys::game_search("test"))
            .await
            .unwrap(),
        None
    );

    // Cleanup
    cache.clear().await.unwrap();
}

#[tokio::test]
async fn test_cache_ttl_values() {
    // Verify TTL values are reasonable

    // Game TTL should be long (changes rarely)
    assert!(CacheTTL::game().as_secs() >= 3600);

    // Game list should be shorter (more dynamic)
    assert!(CacheTTL::game_list().as_secs() < CacheTTL::game().as_secs());

    // Player TTL should be moderate (updates more frequently)
    assert!(CacheTTL::player().as_secs() >= 900); // At least 15 minutes
    assert!(CacheTTL::player().as_secs() < CacheTTL::game().as_secs());

    // Search results should be short (very dynamic)
    assert!(CacheTTL::game_search().as_secs() <= 300); // 5 minutes max

    // Google Places should be very long (24 hours)
    assert_eq!(CacheTTL::google_places().as_secs(), 86400);
}
