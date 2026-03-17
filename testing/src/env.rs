//! Test environment: SurrealDB + Redis from env (same docker stack as CI/e2e).
//! Start stack with: ./deploy/stack.sh start

use anyhow::{Context, Result};
use std::net::SocketAddr;
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

    /// Use existing stack via env vars. Always connects to 127.0.0.1 (ports from SURREALDB_PORT, REDIS_PORT).
    /// If you see "name resolution" failure, run tests from a system terminal (not the IDE) so the process has network.
    pub fn from_env() -> Result<Self> {
        let surrealdb_port = std::env::var("SURREALDB_PORT").unwrap_or_else(|_| "50001".to_string());
        let redis_port = std::env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string());
        let surrealdb_url = format!("http://127.0.0.1:{}", surrealdb_port);
        let redis_url = format!("redis://127.0.0.1:{}/", redis_port);
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

    /// Parse URL as IP:port and return SocketAddr to avoid DNS lookup (same as backend main.rs).
    pub fn surreal_socket_addr(url: &str) -> Option<SocketAddr> {
        let host_port = url
            .trim_start_matches("http://")
            .trim_start_matches("https://");
        let (host, port_str) = host_port.split_once(':')?;
        let port: u16 = port_str.parse().ok()?;
        let ip = host.parse::<std::net::IpAddr>().ok()?;
        Some(SocketAddr::new(ip, port))
    }

    /// Wait for SurrealDB and Redis to be ready.
    /// Uses SocketAddr when URL host is an IP to avoid DNS lookup (fixes "name resolution" in WSL2/Docker).
    pub async fn wait_for_ready(&self) -> Result<()> {
        use surrealdb::engine::remote::ws::Ws;
        use surrealdb::Surreal;
        let ws_url = self.surrealdb_url.replace("http://", "ws://").replace("https://", "wss://");
        let mut last_connect_err = None::<String>;
        let mut last_auth_err = None::<String>;
        let attempt_timeout = Duration::from_secs(15);
        const MAX_ATTEMPTS: u32 = 8;
        for attempt in 0..MAX_ATTEMPTS {
            let connect_fut = async {
                let db = match TestEnvironment::surreal_socket_addr(&self.surrealdb_url) {
                    Some(addr) => Surreal::new::<Ws>(addr).await?,
                    None => Surreal::new::<Ws>(&ws_url).await?,
                };
                db.signin(surrealdb::opt::auth::Root {
                    username: self.surrealdb_user.clone(),
                    password: self.surrealdb_pass.clone(),
                })
                .await?;
                db.use_ns(&self.surrealdb_ns)
                    .use_db(&self.surrealdb_db)
                    .await?;
                Ok::<(), anyhow::Error>(())
            };
            match tokio::time::timeout(attempt_timeout, connect_fut).await {
                Ok(Ok(())) => {
                    log::debug!("SurrealDB ready after {} attempts", attempt + 1);
                    break;
                }
                Ok(Err(e)) => last_auth_err = Some(e.to_string()),
                Err(_) => last_connect_err = Some(format!("connect/signin timed out ({}s)", attempt_timeout.as_secs())),
            }
            if attempt == MAX_ATTEMPTS - 1 {
                let msg = match (last_connect_err.as_deref(), last_auth_err.as_deref()) {
                    (Some(c), Some(a)) => format!("SurrealDB not ready after {} attempts. Last connect err: {}. Last auth err: {}", MAX_ATTEMPTS, c, a),
                    (Some(c), None) => format!("SurrealDB not ready after {} attempts (connect failed: {}). Use SURREAL_URL=http://127.0.0.1:50001 when running tests on host.", MAX_ATTEMPTS, c),
                    (None, Some(a)) => format!("SurrealDB not ready after {} attempts (auth/ns: {})", MAX_ATTEMPTS, a),
                    (None, None) => format!("SurrealDB not ready after {} attempts. Use SURREAL_URL=http://127.0.0.1:50001 when running tests on host.", MAX_ATTEMPTS),
                };
                anyhow::bail!("{}", msg);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        let redis_client = redis::Client::open(self.redis_url()).context("Redis client")?;
        for attempt in 0..20 {
            if redis_client.get_async_connection().await.is_ok() {
                log::debug!("Redis ready after {} attempts", attempt + 1);
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        anyhow::bail!("Redis not ready after 20 attempts")
    }
}
