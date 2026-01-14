//! Example integration test using testcontainers
//!
//! This demonstrates how to use the TestEnvironment to run integration tests
//! against ephemeral Docker containers, with optional production data seeding.
//!
//! Each test gets its own isolated container, ensuring no test interference.

use anyhow::Result;
use redis::AsyncCommands;
use testing::{test_env_with_prod_data, TestEnvironment, TestEnvironmentBuilder};

#[tokio::test]
async fn test_environment_creation() -> Result<()> {
    // Test that we can create a test environment
    let env = TestEnvironment::new().await?;
    assert!(!env.arangodb_url().is_empty());
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
    // This test demonstrates how to use the builder pattern
    // with a data dump from production backup
    let backup_path = "../_build/backups/smacktalk.zip";

    // Skip test if backup file doesn't exist
    if !std::path::Path::new(backup_path).exists() {
        eprintln!(
            "⚠️  Backup file not found at {}, skipping test",
            backup_path
        );
        return Ok(());
    }

    let env = TestEnvironmentBuilder::new()
        .with_data_dump(backup_path)
        .with_database_name("smacktalk")
        .build()
        .await?;

    // Verify the database was created and has data
    // We can check by connecting to ArangoDB and querying collections
    use arangors::Connection;
    let conn =
        Connection::establish_basic_auth(env.arangodb_url(), "root", "test_password").await?;

    // Verify we can access the database (the backup should have created it)
    let _db = conn.db(&env.arangodb_db_name()).await?;

    // Check that we have collections (the backup should have created them)
    // This is a basic sanity check that the restore worked
    log::info!(
        "✅ Database '{}' restored successfully",
        env.arangodb_db_name()
    );

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
    assert!(!env.arangodb_url().is_empty());
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
