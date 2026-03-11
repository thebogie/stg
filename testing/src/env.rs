//! Test environment: SurrealDB + Redis from env (same docker stack as CI/e2e).
//! Start stack with: ./deploy/stack.sh start

use anyhow::{Context, Result};
use std::time::Duration;

/// SurrealDB and Redis URLs from env (SURREAL_URL, REDIS_URL).
pub struct TestEnvironment {
    pub surrealdb_url: String,
    pub redis_url: String,
    pub surrealdb_ns: String,
    pub surrealdb_db: String,
    pub surrealdb_user: String,
    pub surrealdb_pass: String,
}

impl TestEnvironment {
    /// Create test environment from env vars (same stack as ./deploy/stack.sh start).
    pub async fn new() -> Result<Self> {
        Self::from_env()
    }

    /// Use existing stack via env vars (SURREAL_URL, REDIS_URL, etc.).
    pub fn from_env() -> Result<Self> {
        let surrealdb_url =
            std::env::var("SURREAL_URL").unwrap_or_else(|_| "http://127.0.0.1:50001".to_string());
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        let surrealdb_ns = std::env::var("SURREAL_NS").unwrap_or_else(|_| "stg_rd".to_string());
        let surrealdb_db = std::env::var("SURREAL_DB").unwrap_or_else(|_| "stg_rd".to_string());
        let surrealdb_user = std::env::var("SURREAL_USER").unwrap_or_else(|_| "root".to_string());
        let surrealdb_pass =
            std::env::var("SURREAL_PASSWORD").unwrap_or_else(|_| "root".to_string());
        log::info!("Test env: SurrealDB={}, Redis={}", surrealdb_url, redis_url);
        Ok(Self {
            surrealdb_url,
            redis_url,
            surrealdb_ns,
            surrealdb_db,
            surrealdb_user,
            surrealdb_pass,
        })
    }

    pub fn surrealdb_url(&self) -> &str {
        &self.surrealdb_url
    }
    /// Alias for compatibility (same stack now uses SurrealDB).
    pub fn arangodb_url(&self) -> &str {
        &self.surrealdb_url
    }
    pub fn redis_url(&self) -> &str {
        &self.redis_url
    }

    /// Wait for SurrealDB and Redis to be ready.
    pub async fn wait_for_ready(&self) -> Result<()> {
        use surrealdb::engine::remote::ws::Ws;
        use surrealdb::Surreal;
        let ws_url = self.surrealdb_url.replace("http://", "ws://").replace("https://", "wss://");
        for attempt in 0..60 {
            if let Ok(db) = Surreal::new::<Ws>(&ws_url).await {
                if db
                    .signin(surrealdb::opt::auth::Root {
                        username: &self.surrealdb_user,
                        password: &self.surrealdb_pass,
                    })
                    .await
                    .is_ok()
                    && db
                        .use_ns(&self.surrealdb_ns)
                        .use_db(&self.surrealdb_db)
                        .await
                        .is_ok()
                {
                    log::debug!("SurrealDB ready after {} attempts", attempt + 1);
                    break;
                }
            }
            if attempt == 59 {
                anyhow::bail!("SurrealDB not ready after 60 attempts");
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        let redis_client = redis::Client::open(self.redis_url()).context("Redis client")?;
        for attempt in 0..30 {
            if redis_client.get_async_connection().await.is_ok() {
                log::debug!("Redis ready after {} attempts", attempt + 1);
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        anyhow::bail!("Redis not ready after 30 attempts")
    }
}
