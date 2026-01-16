//! Integration testing infrastructure using testcontainers-rs
//!
//! This module provides utilities for spinning up ephemeral Docker containers
//! for ArangoDB and Redis during integration tests.
//!
//! Containers are automatically started when TestEnvironment is created and
//! automatically stopped/removed when it goes out of scope (RAII pattern).

use anyhow::{Context, Result};
use arangors::ClientError;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use testcontainers::{
    core::IntoContainerPort, runners::AsyncRunner, ContainerAsync, GenericImage, ImageExt,
};

/// Test environment with ArangoDB and Redis containers
///
/// Containers are automatically managed - they start when created and
/// stop/remove when dropped (ephemeral containers).
pub struct TestEnvironment {
    arangodb_url: String,
    redis_url: String,
    arangodb_db_name: std::cell::RefCell<String>,
    // Keep containers alive for the lifetime of TestEnvironment
    // When dropped, containers are automatically stopped and removed
    _arangodb: ContainerAsync<GenericImage>,
    _redis: ContainerAsync<GenericImage>,
}

impl TestEnvironment {
    /// Create a new test environment with ArangoDB and Redis containers
    ///
    /// This spins up ephemeral Docker containers that will be automatically
    /// cleaned up when the TestEnvironment is dropped.
    pub async fn new() -> Result<Self> {
        // Check if we should use testcontainers or fall back to env vars
        let use_testcontainers = std::env::var("USE_TESTCONTAINERS")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        if !use_testcontainers {
            // Fallback to environment variables (useful for CI or manual testing)
            return Ok(Self::from_env_vars().await?);
        }

        // Start Docker containers using testcontainers
        // Start ArangoDB container with retry logic for parallel test execution
        // Add small delay before starting to stagger container creation
        tokio::time::sleep(Duration::from_millis(100)).await;

        let arangodb = {
            let mut container_result = None;
            for attempt in 0..5 {
                match GenericImage::new("arangodb", "3.12.5")
                    .with_env_var("ARANGO_ROOT_PASSWORD", "test_password")
                    .start()
                    .await
                {
                    Ok(container) => {
                        // Verify container is actually running before proceeding
                        // Sometimes the container starts but immediately exits
                        tokio::time::sleep(Duration::from_millis(3000)).await;

                        // Try to get the port - if this fails, container might not be ready
                        match container.get_host_port_ipv4(8529.tcp()).await {
                            Ok(port) => {
                                log::debug!(
                                    "ArangoDB container started successfully on attempt {} (port {})",
                                    attempt + 1,
                                    port
                                );
                                container_result = Some(Ok(container));
                                break;
                            }
                            Err(e) => {
                                log::warn!(
                                    "ArangoDB container started but port not available (attempt {}): {:?}",
                                    attempt + 1,
                                    e
                                );
                                if attempt < 4 {
                                    // Retry by continuing the loop with exponential backoff
                                    let backoff_ms = 2000 * (attempt + 1) as u64;
                                    log::debug!(
                                        "Retrying ArangoDB container start in {}ms...",
                                        backoff_ms
                                    );
                                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                                    // Don't set container_result, let it retry
                                } else {
                                    container_result = Some(Err(anyhow::anyhow!(
                                        "ArangoDB container port not available after {} attempts: {:?}",
                                        attempt + 1,
                                        e
                                    )));
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if attempt < 4 {
                            log::warn!(
                                "Failed to start ArangoDB container (attempt {}), retrying...",
                                attempt + 1
                            );
                            tokio::time::sleep(Duration::from_millis(2000 * (attempt + 1) as u64))
                                .await;
                        } else {
                            container_result = Some(Err(anyhow::anyhow!(
                                "Failed to start ArangoDB container after {} attempts: {:?}",
                                attempt + 1,
                                e
                            )));
                        }
                    }
                }
            }
            container_result
                .expect("Should have container result")
                .context("Failed to start ArangoDB container")?
        };

        let arangodb_port = arangodb
            .get_host_port_ipv4(8529.tcp())
            .await
            .context("Failed to get ArangoDB container port")?;
        let arangodb_url = format!("http://localhost:{}", arangodb_port);

        // Start Redis container with retry logic
        // Increased retries and delays for parallel execution
        let redis = {
            let mut container_result = None;
            for attempt in 0..5 {
                match GenericImage::new("redis", "7-alpine").start().await {
                    Ok(container) => {
                        // Give it more time to bind ports and start services
                        // Increased for parallel execution where containers compete for resources
                        tokio::time::sleep(Duration::from_millis(3000)).await;

                        // Verify port is available (similar to ArangoDB check)
                        match container.get_host_port_ipv4(6379.tcp()).await {
                            Ok(_) => {
                                log::debug!(
                                    "Redis container started successfully on attempt {}",
                                    attempt + 1
                                );
                                container_result = Some(container);
                                break;
                            }
                            Err(e) => {
                                log::warn!(
                                    "Redis container started but port not available (attempt {}): {:?}",
                                    attempt + 1,
                                    e
                                );
                                if attempt < 4 {
                                    tokio::time::sleep(Duration::from_millis(
                                        2000 * (attempt + 1) as u64,
                                    ))
                                    .await;
                                } else {
                                    return Err(anyhow::anyhow!(
                                        "Redis container port not available after {} attempts: {:?}",
                                        attempt + 1,
                                        e
                                    ))
                                    .context("Failed to start Redis container");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if attempt < 4 {
                            log::warn!(
                                "Failed to start Redis container (attempt {}), retrying...",
                                attempt + 1
                            );
                            tokio::time::sleep(Duration::from_millis(2000 * (attempt + 1) as u64))
                                .await;
                        } else {
                            return Err(anyhow::anyhow!(
                                "Failed to start Redis container after {} attempts: {:?}",
                                attempt + 1,
                                e
                            ))
                            .context("Failed to start Redis container");
                        }
                    }
                }
            }
            container_result.expect("Should have container after successful start")
        };

        let redis_port = redis
            .get_host_port_ipv4(6379.tcp())
            .await
            .context("Failed to get Redis container port")?;
        let redis_url = format!("redis://localhost:{}/", redis_port);

        log::info!("Started ArangoDB container at {}", arangodb_url);
        log::info!("Started Redis container at {}", redis_url);

        Ok(Self {
            arangodb_url,
            redis_url,
            arangodb_db_name: std::cell::RefCell::new("smacktalk".to_string()),
            _arangodb: arangodb,
            _redis: redis,
        })
    }

    /// Create test environment from environment variables (fallback mode)
    ///
    /// This is useful when you want to use existing containers instead of
    /// spinning up new ones (e.g., in CI or for debugging).
    async fn from_env_vars() -> Result<Self> {
        let arangodb_url =
            std::env::var("ARANGO_URL").unwrap_or_else(|_| "http://localhost:8529".to_string());
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379/".to_string());

        log::info!("Using environment variables for test environment");
        log::info!("ArangoDB: {}", arangodb_url);
        log::info!("Redis: {}", redis_url);

        // In fallback mode, we still need containers for the type system
        // But we'll create minimal ones that won't actually be used
        // This is a limitation - ideally we'd have a separate type for fallback mode
        let dummy_arangodb = GenericImage::new("arangodb", "3.12.5")
            .with_env_var("ARANGO_ROOT_PASSWORD", "test_password")
            .start()
            .await
            .context("Failed to create dummy ArangoDB container (Docker may not be available)")?;
        let dummy_redis = GenericImage::new("redis", "7-alpine")
            .start()
            .await
            .context("Failed to create dummy Redis container (Docker may not be available)")?;

        Ok(Self {
            arangodb_url,
            redis_url,
            arangodb_db_name: std::cell::RefCell::new("smacktalk".to_string()),
            _arangodb: dummy_arangodb,
            _redis: dummy_redis,
        })
    }

    /// Get ArangoDB connection URL
    pub fn arangodb_url(&self) -> &str {
        &self.arangodb_url
    }

    /// Get Redis connection URL
    pub fn redis_url(&self) -> &str {
        &self.redis_url
    }

    /// Wait for services to be ready
    ///
    /// This gives services a moment to fully initialize after containers start.
    /// The WaitFor conditions in the image definitions should handle most of this,
    /// but this provides an additional safety buffer.
    pub async fn wait_for_ready(&self) -> Result<()> {
        // Wait for services to be fully ready with retry logic
        // Optimized for parallel execution with faster initial checks and hard timeout
        let start_time = std::time::Instant::now();
        let max_total_time = Duration::from_secs(120); // Hard timeout of 120 seconds total (increased for slower systems)
        let max_attempts = 120; // More attempts but with shorter waits
        let mut last_error: Option<ClientError> = None;

        for attempt in 0..max_attempts {
            // Check if we've exceeded total time budget
            if start_time.elapsed() > max_total_time {
                let error_msg = last_error
                    .as_ref()
                    .map(|e: &ClientError| e.to_string())
                    .unwrap_or_else(|| "timeout".to_string());
                return Err(anyhow::anyhow!(
                    "ArangoDB failed to become ready within {}s (attempt {}): {}",
                    max_total_time.as_secs(),
                    attempt + 1,
                    error_msg
                ));
            }

            // Try to connect to ArangoDB to verify it's ready
            match arangors::Connection::establish_basic_auth(
                &self.arangodb_url,
                "root",
                "test_password",
            )
            .await
            {
                Ok(_) => {
                    log::debug!(
                        "ArangoDB is ready after {} attempts ({:.2}s)",
                        attempt + 1,
                        start_time.elapsed().as_secs_f64()
                    );
                    last_error = None;
                    break;
                }
                Err(e) if attempt < max_attempts - 1 => {
                    let error_msg = e.to_string();
                    last_error = Some(e);
                    // Faster initial checks, then exponential backoff
                    // Check more frequently at first when containers are likely starting
                    let wait_ms = if attempt < 15 {
                        300 + (attempt * 150) as u64 // 300ms, 450ms, 600ms... up to 2.4s
                    } else if attempt < 40 {
                        2000 + ((attempt - 15) * 300) as u64 // 2s, 2.3s, 2.6s... up to 9.5s
                    } else {
                        3000 // Cap at 3s for later attempts
                    };

                    // Don't wait if we're close to timeout
                    let remaining = max_total_time.saturating_sub(start_time.elapsed());
                    let wait_duration = Duration::from_millis(wait_ms);
                    if wait_duration > remaining {
                        return Err(anyhow::anyhow!(
                            "ArangoDB failed to become ready within {}s (attempt {}): {}",
                            max_total_time.as_secs(),
                            attempt + 1,
                            error_msg
                        ));
                    }

                    log::debug!(
                        "ArangoDB not ready yet (attempt {}, {:.2}s elapsed): {}, waiting {}ms...",
                        attempt + 1,
                        start_time.elapsed().as_secs_f64(),
                        error_msg,
                        wait_ms
                    );
                    tokio::time::sleep(wait_duration).await;
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        if let Some(e) = last_error {
            return Err(anyhow::anyhow!(
                "ArangoDB failed to become ready after {} attempts: {}",
                max_attempts,
                e
            ));
        }

        // Verify Redis is also ready - optimized for parallel execution
        let redis_start_time = std::time::Instant::now();
        let redis_max_time = Duration::from_secs(60); // Increased timeout for parallel execution (was 45s)
        let redis_client = redis::Client::open(self.redis_url())?;
        let redis_max_attempts = 90; // More attempts with shorter waits
        let mut redis_ready = false;

        for attempt in 0..redis_max_attempts {
            // Check if we've exceeded time budget
            if redis_start_time.elapsed() > redis_max_time {
                return Err(anyhow::anyhow!(
                    "Redis failed to become ready within {}s (attempt {})",
                    redis_max_time.as_secs(),
                    attempt + 1
                ));
            }

            match redis_client.get_async_connection().await {
                Ok(mut conn) => {
                    match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
                        Ok(_) => {
                            log::debug!(
                                "Redis is ready after {} attempts ({:.2}s)",
                                attempt + 1,
                                redis_start_time.elapsed().as_secs_f64()
                            );
                            redis_ready = true;
                            break;
                        }
                        Err(e) if attempt < redis_max_attempts - 1 => {
                            // Faster initial checks for Redis
                            let wait_ms = if attempt < 20 {
                                200 + (attempt * 100) as u64 // 200ms, 300ms, 400ms... up to 2.1s
                            } else if attempt < 50 {
                                1500 + ((attempt - 20) * 200) as u64 // 1.5s, 1.7s, 1.9s... up to 7.5s
                            } else {
                                2000 // Cap at 2s
                            };

                            let remaining =
                                redis_max_time.saturating_sub(redis_start_time.elapsed());
                            let wait_duration = Duration::from_millis(wait_ms);
                            if wait_duration > remaining {
                                return Err(anyhow::anyhow!(
                                    "Redis failed to become ready within {}s (attempt {}): {}",
                                    redis_max_time.as_secs(),
                                    attempt + 1,
                                    e
                                ));
                            }

                            log::debug!(
                                "Redis PING failed (attempt {}, {:.2}s elapsed): {}, waiting {}ms...",
                                attempt + 1,
                                redis_start_time.elapsed().as_secs_f64(),
                                e,
                                wait_ms
                            );
                            tokio::time::sleep(wait_duration).await;
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!(
                                "Redis failed to become ready after {} attempts: {}",
                                redis_max_attempts,
                                e
                            ));
                        }
                    }
                }
                Err(e) if attempt < redis_max_attempts - 1 => {
                    // Same wait logic for connection failures
                    let wait_ms = if attempt < 20 {
                        200 + (attempt * 100) as u64
                    } else if attempt < 50 {
                        1500 + ((attempt - 20) * 200) as u64
                    } else {
                        2000
                    };

                    let remaining = redis_max_time.saturating_sub(redis_start_time.elapsed());
                    let wait_duration = Duration::from_millis(wait_ms);
                    if wait_duration > remaining {
                        return Err(anyhow::anyhow!(
                            "Redis failed to become ready within {}s (attempt {}): {}",
                            redis_max_time.as_secs(),
                            attempt + 1,
                            e
                        ));
                    }

                    log::debug!(
                        "Redis connection failed (attempt {}, {:.2}s elapsed): {}, waiting {}ms...",
                        attempt + 1,
                        redis_start_time.elapsed().as_secs_f64(),
                        e,
                        wait_ms
                    );
                    tokio::time::sleep(wait_duration).await;
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Redis failed to become ready after {} attempts: {}",
                        redis_max_attempts,
                        e
                    ));
                }
            }
        }

        if !redis_ready {
            return Err(anyhow::anyhow!(
                "Redis failed to become ready after {} attempts",
                redis_max_attempts
            ));
        }

        // Additional safety buffer for services to fully initialize
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(())
    }

    /// Get the database name
    pub fn arangodb_db_name(&self) -> String {
        self.arangodb_db_name.borrow().clone()
    }

    /// Get the container ID for the ArangoDB container
    fn arangodb_container_id(&self) -> Result<String> {
        // Find the container by filtering for the arangodb image
        // We'll use docker ps to find the running container
        let output = Command::new("docker")
            .args(&[
                "ps",
                "--filter",
                "ancestor=arangodb:3.12.5",
                "--format",
                "{{.ID}}",
            ])
            .output()
            .context("Failed to find ArangoDB container")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to find ArangoDB container"));
        }

        let container_id = String::from_utf8(output.stdout)
            .context("Failed to parse container ID")?
            .trim()
            .to_string();

        if container_id.is_empty() {
            return Err(anyhow::anyhow!("ArangoDB container not found"));
        }

        Ok(container_id)
    }

    /// Load a data dump into ArangoDB
    ///
    /// This method:
    /// 1. Copies the backup file (zip) into the container
    /// 2. Extracts it inside the container
    /// 3. Finds the database directory (handles nested structures)
    /// 4. Uses arangorestore to restore the data
    ///
    /// Supports various dump structures:
    /// - `dump.zip` containing `smacktalk/` directory
    /// - `dump.zip` containing database files directly
    /// - Nested structures like `dump.zip` -> `backup/` -> `smacktalk/`
    pub async fn load_data_dump(&self, dump_path: &str) -> Result<()> {
        let dump_path = Path::new(dump_path);

        if !dump_path.exists() {
            return Err(anyhow::anyhow!(
                "Backup file not found: {}",
                dump_path.display()
            ));
        }

        let container_id = self.arangodb_container_id()?;
        log::info!(
            "Loading data dump from {} into container {}",
            dump_path.display(),
            container_id
        );

        // Step 1: Copy the backup file into the container
        let copy_output = Command::new("docker")
            .args(&[
                "cp",
                dump_path.to_str().unwrap(),
                &format!("{}:/tmp/backup.zip", container_id),
            ])
            .output()
            .context("Failed to copy backup file into container")?;

        if !copy_output.status.success() {
            let error = String::from_utf8_lossy(&copy_output.stderr);
            return Err(anyhow::anyhow!("Failed to copy backup file: {}", error));
        }

        log::info!("Copied backup file into container");

        // Step 2: Extract the zip file inside the container
        let extract_output = Command::new("docker")
            .args(&[
                "exec",
                &container_id,
                "sh",
                "-c",
                "cd /tmp && unzip -q -o backup.zip -d /tmp/dump",
            ])
            .output()
            .context("Failed to extract backup file in container")?;

        if !extract_output.status.success() {
            let error = String::from_utf8_lossy(&extract_output.stderr);
            return Err(anyhow::anyhow!("Failed to extract backup file: {}", error));
        }

        log::info!("Extracted backup file in container");

        // Step 3: Find the database directory (handles nested structures)
        let db_name = self.arangodb_db_name.borrow().clone();
        let dump_dir = self.find_dump_directory(&container_id, &db_name).await?;

        // Step 4: Wait a bit for ArangoDB to be fully ready
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Step 5: Use arangorestore to restore the data
        let restore_output = Command::new("docker")
            .args(&[
                "exec",
                &container_id,
                "arangorestore",
                "--server.endpoint",
                "tcp://127.0.0.1:8529",
                "--server.username",
                "root",
                "--server.password",
                "test_password",
                "--input-directory",
                &dump_dir,
                "--create-database",
                "true",
                "--server.database",
                &db_name,
            ])
            .output()
            .context("Failed to restore backup using arangorestore")?;

        if !restore_output.status.success() {
            let error = String::from_utf8_lossy(&restore_output.stderr);
            let stdout = String::from_utf8_lossy(&restore_output.stdout);
            log::warn!("arangorestore stderr: {}", error);
            log::warn!("arangorestore stdout: {}", stdout);
            return Err(anyhow::anyhow!(
                "Failed to restore backup. Tried directory: {}. Error: {}",
                dump_dir,
                error
            ));
        }

        log::info!("Successfully restored backup into database '{}'", db_name);

        // Cleanup: Remove the backup files from the container
        let _ = Command::new("docker")
            .args(&[
                "exec",
                &container_id,
                "rm",
                "-rf",
                "/tmp/backup.zip",
                "/tmp/dump",
            ])
            .output();

        Ok(())
    }

    /// Find the database directory within the extracted dump
    ///
    /// Handles various dump structures by searching for the database name
    /// in common locations.
    async fn find_dump_directory(&self, container_id: &str, db_name: &str) -> Result<String> {
        // Try common locations in order
        let candidates = vec![
            format!("/tmp/dump/{}", db_name),
            format!("/tmp/dump/backup/{}", db_name),
            format!("/tmp/dump/dump/{}", db_name),
            "/tmp/dump".to_string(), // Direct dump (no database subdirectory)
        ];

        for candidate in &candidates {
            // Check if directory exists and contains ArangoDB files
            let check_cmd = format!(
                "test -d {} && (test -f {}/_graphs || test -f {}/_collections || ls {} | grep -q '\\.json$')",
                candidate, candidate, candidate, candidate
            );

            let check_output = Command::new("docker")
                .args(&["exec", container_id, "sh", "-c", &check_cmd])
                .output()
                .context("Failed to check dump directory")?;

            if check_output.status.success() {
                log::info!("Found database dump at: {}", candidate);
                return Ok(candidate.clone());
            }
        }

        // If no specific directory found, try listing what we have
        let list_output = Command::new("docker")
            .args(&[
                "exec",
                container_id,
                "sh",
                "-c",
                "find /tmp/dump -type d -name '*' | head -10",
            ])
            .output()
            .context("Failed to list dump directories")?;

        let dirs = String::from_utf8_lossy(&list_output.stdout);
        Err(anyhow::anyhow!(
            "Could not find database directory for '{}' in dump. \
             Tried: {}. Found directories: {}",
            db_name,
            candidates.join(", "),
            dirs
        ))
    }
}

/// Helper to create a test environment with sanitized data
pub struct TestEnvironmentBuilder {
    data_dump_path: Option<String>,
    database_name: Option<String>,
    skip_data_load_if_missing: bool,
}

impl TestEnvironmentBuilder {
    pub fn new() -> Self {
        Self {
            data_dump_path: None,
            database_name: None,
            skip_data_load_if_missing: false,
        }
    }

    /// Load sanitized production data dump
    ///
    /// The dump file should be a zip archive containing ArangoDB backup data.
    /// Common locations:
    /// - `../_build/backups/smacktalk.zip` (relative to workspace root)
    /// - `./test_data/smacktalk.zip` (relative to test binary)
    ///
    /// You can also set `TEST_DATA_DUMP_PATH` environment variable for a default path.
    pub fn with_data_dump(mut self, path: &str) -> Self {
        self.data_dump_path = Some(path.to_string());
        self
    }

    /// Load production data dump from environment variable or default location
    ///
    /// Checks in order:
    /// 1. `TEST_DATA_DUMP_PATH` environment variable
    /// 2. `../_build/backups/smacktalk.zip` (relative to workspace root)
    /// 3. `./test_data/smacktalk.zip` (relative to test binary)
    ///
    /// Returns `None` if no dump file is found.
    pub fn with_default_data_dump(mut self) -> Self {
        // Try environment variable first
        if let Ok(path) = std::env::var("TEST_DATA_DUMP_PATH") {
            if Path::new(&path).exists() {
                self.data_dump_path = Some(path);
                return self;
            }
        }

        // Try common locations
        let common_paths = vec![
            "../_build/backups/smacktalk.zip",
            "./test_data/smacktalk.zip",
            "_build/backups/smacktalk.zip",
            "test_data/smacktalk.zip",
        ];

        for path in common_paths {
            if Path::new(path).exists() {
                self.data_dump_path = Some(path.to_string());
                return self;
            }
        }

        // If skip_data_load_if_missing is true, continue without error
        // Otherwise, the build will fail when trying to load
        self
    }

    /// Set the database name (default: "smacktalk")
    pub fn with_database_name(mut self, name: &str) -> Self {
        self.database_name = Some(name.to_string());
        self
    }

    /// If data dump is missing, skip loading instead of failing
    ///
    /// Useful for tests that work with or without production data.
    pub fn skip_data_load_if_missing(mut self) -> Self {
        self.skip_data_load_if_missing = true;
        self
    }

    /// Build the test environment
    pub async fn build(self) -> Result<TestEnvironment> {
        let env = TestEnvironment::new().await?;

        // Set database name if provided
        if let Some(db_name) = self.database_name {
            *env.arangodb_db_name.borrow_mut() = db_name;
        }

        // Wait for services to be ready before loading data
        env.wait_for_ready().await?;

        // If data dump is provided, load it into ArangoDB
        if let Some(dump_path) = self.data_dump_path {
            let dump_path = Path::new(&dump_path);
            if !dump_path.exists() {
                if self.skip_data_load_if_missing {
                    log::warn!(
                        "Data dump not found at {}, skipping data load",
                        dump_path.display()
                    );
                } else {
                    return Err(anyhow::anyhow!(
                        "Data dump file not found: {}. Set TEST_DATA_DUMP_PATH or use skip_data_load_if_missing()",
                        dump_path.display()
                    ));
                }
            } else {
                env.load_data_dump(dump_path.to_str().unwrap()).await?;
            }
        }

        Ok(env)
    }
}

impl Default for TestEnvironmentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a test environment with production data from default location
///
/// This is a convenience function that:
/// 1. Looks for data dump in common locations (env var, `../_build/backups/`, etc.)
/// 2. Loads it into a fresh container
/// 3. Returns the test environment ready to use
///
/// # Example
/// ```rust,no_run
/// #[tokio::test]
/// async fn test_with_prod_data() -> Result<()> {
///     let env = test_env_with_prod_data().await?;
///     // Test against production data structure
///     Ok(())
/// }
/// ```
pub async fn test_env_with_prod_data() -> Result<TestEnvironment> {
    TestEnvironmentBuilder::new()
        .with_default_data_dump()
        .skip_data_load_if_missing()
        .build()
        .await
}

/// Create a test environment with production data from a specific path
///
/// # Example
/// ```rust,no_run
/// #[tokio::test]
/// async fn test_with_specific_dump() -> Result<()> {
///     let env = test_env_with_dump("../_build/backups/smacktalk.zip").await?;
///     // Test against specific production data
///     Ok(())
/// }
/// ```
pub async fn test_env_with_dump(dump_path: &str) -> Result<TestEnvironment> {
    TestEnvironmentBuilder::new()
        .with_data_dump(dump_path)
        .build()
        .await
}

/// Create a test environment with production data, using a custom database name
///
/// Useful when you need multiple isolated databases in the same test suite.
///
/// # Example
/// ```rust,no_run
/// #[tokio::test]
/// async fn test_with_custom_db() -> Result<()> {
///     let env = test_env_with_prod_data_and_db("test_db_1").await?;
///     // Test with isolated database
///     Ok(())
/// }
/// ```
pub async fn test_env_with_prod_data_and_db(db_name: &str) -> Result<TestEnvironment> {
    TestEnvironmentBuilder::new()
        .with_default_data_dump()
        .with_database_name(db_name)
        .skip_data_load_if_missing()
        .build()
        .await
}

// Re-export app setup for convenience
pub mod app_setup;

/// Helper function to create a test environment with timeouts
/// This ensures tests fail fast if containers don't start in time
/// Use this in all integration tests for consistent timeout behavior
pub async fn create_test_env_with_timeout() -> Result<TestEnvironment> {
    use std::time::Duration;
    let env = tokio::time::timeout(Duration::from_secs(90), TestEnvironment::new())
        .await
        .map_err(|_| anyhow::anyhow!("Test environment setup timed out after 90s"))??;

    // wait_for_ready now has its own internal timeouts (120s ArangoDB + 45s Redis)
    // Add a small buffer timeout here (total should be ~180s max)
    tokio::time::timeout(Duration::from_secs(180), env.wait_for_ready())
        .await
        .map_err(|_| anyhow::anyhow!("wait_for_ready timed out after 180s"))??;

    Ok(env)
}

/// Macro to create an authenticated user and return session token
/// Usage: let session_id = create_authenticated_user!(app, "email@example.com", "username").await?;
#[macro_export]
macro_rules! create_authenticated_user {
    ($app:expr, $email:expr, $username:expr) => {{
        // First register the user
        let register_req = actix_web::test::TestRequest::post()
            .uri("/api/players/register")
            .set_json(&serde_json::json!({
                "username": $username,
                "email": $email,
                "password": "password123"
            }))
            .to_request();

        let register_resp = actix_web::test::call_service(&$app, register_req).await;
        assert!(
            register_resp.status().is_success(),
            "User registration should succeed"
        );

        // Then login to get session
        let login_req = actix_web::test::TestRequest::post()
            .uri("/api/players/login")
            .set_json(&serde_json::json!({
                "email": $email,
                "password": "password123"
            }))
            .to_request();

        let login_resp = actix_web::test::call_service(&$app, login_req).await;
        assert!(
            login_resp.status().is_success(),
            "User login should succeed"
        );

        let login_body: shared::dto::player::LoginResponse = actix_web::test::read_body_json(login_resp).await;
        login_body.session_id
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_environment_creation() {
        let env = TestEnvironment::new().await.unwrap();
        assert!(!env.arangodb_url().is_empty());
        assert!(!env.redis_url().is_empty());
    }
}
