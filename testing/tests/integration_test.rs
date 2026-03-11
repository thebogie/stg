//! Integration tests against the same docker stack (SurrealDB + Redis).
//! Start stack with: ./deploy/stack.sh start

use anyhow::Result;
use redis::AsyncCommands;
use testing::{test_env_with_prod_data, TestEnvironment, TestEnvironmentBuilder};

#[tokio::test]
async fn test_environment_creation() -> Result<()> {
    let env = TestEnvironment::new().await?;
    assert!(!env.surrealdb_url().is_empty());
    assert!(!env.redis_url().is_empty());
    Ok(())
}

#[tokio::test]
async fn test_redis_connection() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;

    let client = redis::Client::open(env.redis_url())?;
    let mut conn = client.get_async_connection().await?;

    // Test basic Redis operations
    let _: () = conn.set("test_key", "test_value").await?;
    let value: String = conn.get("test_key").await?;
    assert_eq!(value, "test_value");

    // Cleanup
    let _: () = conn.del("test_key").await?;

    Ok(())
}

#[tokio::test]
async fn test_environment_with_data_dump() -> Result<()> {
    // Builder still works (no dump loading with SurrealDB same-stack)
    let env = TestEnvironmentBuilder::new()
        .with_database_name("stg_rd")
        .build()
        .await?;
    env.wait_for_ready().await?;
    assert!(!env.surrealdb_url().is_empty());
    Ok(())
}

#[tokio::test]
async fn test_with_automatic_prod_data_discovery() -> Result<()> {
    // This test demonstrates the convenience helper that automatically
    // finds and loads production data from common locations
    // It gracefully handles missing data dumps (useful for CI)
    let env = test_env_with_prod_data().await?;

    // Test works whether or not production data was loaded
    // Container is still isolated and fresh
    assert!(!env.surrealdb_url().is_empty());
    assert!(!env.redis_url().is_empty());

    Ok(())
}

// Example of table-driven testing - using regular test instead of rstest
// to avoid async_std dependency issue
#[tokio::test]
async fn test_redis_multiple_keys() -> Result<()> {
    let test_cases = vec![
        ("test_key_1", "test_value_1"),
        ("test_key_2", "test_value_2"),
        ("test_key_3", "test_value_3"),
    ];

    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;

    let client = redis::Client::open(env.redis_url())?;
    let mut conn = client.get_async_connection().await?;

    for (key, value) in test_cases {
        // Set and get value
        let _: () = conn.set(key, value).await?;
        let retrieved: String = conn.get(key).await?;
        assert_eq!(retrieved, value);

        // Cleanup
        let _: () = conn.del(key).await?;
    }

    Ok(())
}
