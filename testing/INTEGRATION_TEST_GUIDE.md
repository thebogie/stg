# Integration Testing Guide: Production Data Seeding

This guide explains how to use production data in integration tests with testcontainers.

## Quick Start

### Basic Test (No Production Data)

```rust
use testing::TestEnvironment;

#[tokio::test]
async fn test_basic() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    
    // Test with empty database
    Ok(())
}
```

### Test with Production Data (Recommended)

```rust
use testing::test_env_with_prod_data;

#[tokio::test]
async fn test_with_production_data() -> Result<()> {
    // Automatically finds and loads production data dump
    let env = test_env_with_prod_data().await?;
    
    // Test against real production data structure
    // Container is automatically cleaned up after test
    Ok(())
}
```

## Production Data Seeding Options

### Option 1: Automatic Discovery (Easiest)

The `test_env_with_prod_data()` helper automatically looks for data dumps in:

1. `TEST_DATA_DUMP_PATH` environment variable
2. `../_build/backups/smacktalk.zip` (relative to workspace root)
3. `./test_data/smacktalk.zip` (relative to test binary)
4. `_build/backups/smacktalk.zip`
5. `test_data/smacktalk.zip`

```rust
use testing::test_env_with_prod_data;

#[tokio::test]
async fn test_automatic() -> Result<()> {
    let env = test_env_with_prod_data().await?;
    // If dump not found, test continues without error
    // (useful for CI where dumps might not be available)
    Ok(())
}
```

### Option 2: Explicit Path

```rust
use testing::test_env_with_dump;

#[tokio::test]
async fn test_explicit_path() -> Result<()> {
    let env = test_env_with_dump("../_build/backups/smacktalk.zip").await?;
    // Test will fail if dump file doesn't exist
    Ok(())
}
```

### Option 3: Builder Pattern (Most Flexible)

```rust
use testing::TestEnvironmentBuilder;

#[tokio::test]
async fn test_builder_pattern() -> Result<()> {
    let env = TestEnvironmentBuilder::new()
        .with_data_dump("../_build/backups/smacktalk.zip")
        .with_database_name("my_test_db")
        .skip_data_load_if_missing()  // Don't fail if dump missing
        .build()
        .await?;
    
    Ok(())
}
```

## Environment Variables

Set these environment variables to customize behavior:

- `TEST_DATA_DUMP_PATH`: Default path to production data dump
- `USE_TESTCONTAINERS`: Set to `false` to use existing containers (fallback mode)
- `ARANGO_URL`: ArangoDB URL (when `USE_TESTCONTAINERS=false`)
- `REDIS_URL`: Redis URL (when `USE_TESTCONTAINERS=false`)

## Full Example: API Test with Production Data

```rust
use anyhow::Result;
use actix_web::{test, web, App};
use testing::{test_env_with_prod_data, app_setup};

#[tokio::test]
async fn test_player_search_with_prod_data() -> Result<()> {
    // Start container with production data
    let env = test_env_with_prod_data().await?;
    
    // Set up test app with real repositories
    let app_data = app_setup::setup_test_app_data(&env).await?;
    
    let app = test::init_service(
        App::new()
            .app_data(app_data.player_repo.clone())
            .app_data(app_data.session_store.clone())
            .service(backend::player::controller::search_players_handler)
    ).await;

    // Test against real production data structure
    let req = test::TestRequest::get()
        .uri("/api/players/search?q=test")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    Ok(())
}
```

## Data Dump Structure

The data dump loader supports various ArangoDB backup structures:

- **Standard**: `dump.zip` → `smacktalk/` → database files
- **Nested**: `dump.zip` → `backup/` → `smacktalk/` → database files
- **Direct**: `dump.zip` → database files (no subdirectory)

The loader automatically detects the structure and finds the database directory.

## Best Practices

### 1. Use Per-Test Containers

Each test gets its own isolated container:

```rust
#[tokio::test]
async fn test_1() -> Result<()> {
    let env = test_env_with_prod_data().await?;
    // Isolated container, no interference
}

#[tokio::test]
async fn test_2() -> Result<()> {
    let env = test_env_with_prod_data().await?;
    // Separate container, completely isolated
}
```

### 2. Handle Missing Data Gracefully

```rust
#[tokio::test]
async fn test_works_with_or_without_data() -> Result<()> {
    // This won't fail if dump is missing
    let env = TestEnvironmentBuilder::new()
        .with_default_data_dump()
        .skip_data_load_if_missing()
        .build()
        .await?;
    
    // Test logic that works with or without production data
    Ok(())
}
```

### 3. Use Custom Database Names for Isolation

```rust
#[tokio::test]
async fn test_with_isolated_db() -> Result<()> {
    let env = test_env_with_prod_data_and_db("test_db_1").await?;
    // Uses separate database, even if sharing container
    Ok(())
}
```

### 4. Combine with App Setup

```rust
use testing::{test_env_with_prod_data, app_setup};

#[tokio::test]
async fn test_full_stack() -> Result<()> {
    let env = test_env_with_prod_data().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;
    
    // Full backend app with production data structure
    let app = test::init_service(/* ... */).await;
    
    // Test complete workflows
    Ok(())
}
```

## Performance Considerations

- **Container startup**: ~2-5 seconds per test
- **Data loading**: ~5-15 seconds for typical production dumps
- **Total per test**: ~7-20 seconds

To optimize:
1. Use `skip_data_load_if_missing()` for tests that don't need production data
2. Group related tests that can share setup
3. Use smaller, focused data dumps for specific test scenarios

## Troubleshooting

### Data Dump Not Found

```
Error: Data dump file not found: ../_build/backups/smacktalk.zip
```

**Solutions:**
- Set `TEST_DATA_DUMP_PATH` environment variable
- Use `skip_data_load_if_missing()` if data is optional
- Check that dump file exists at expected location

### Database Directory Not Found

```
Error: Could not find database directory for 'smacktalk' in dump
```

**Solutions:**
- Verify dump structure matches expected format
- Check that database name matches dump directory name
- Use `with_database_name()` to match your dump structure

### Container Startup Issues

```
Error: Failed to start ArangoDB container
```

**Solutions:**
- Ensure Docker is running: `docker ps`
- Check Docker permissions
- Try fallback mode: `USE_TESTCONTAINERS=false`

## Migration from Old Pattern

**Old way:**
```rust
let env = TestEnvironment::new().await?;
env.wait_for_ready().await?;
// Empty database
```

**New way (with production data):**
```rust
let env = test_env_with_prod_data().await?;
// Database pre-loaded with production data
```

**Old way (manual dump loading):**
```rust
let env = TestEnvironmentBuilder::new()
    .with_data_dump("../_build/backups/smacktalk.zip")
    .build()
    .await?;
```

**New way (automatic discovery):**
```rust
let env = test_env_with_prod_data().await?;
// Automatically finds dump file
```



