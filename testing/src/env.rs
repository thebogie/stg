//! Test environment: SurrealDB + Redis from env (same docker stack as CI/e2e).
//! Start stack with: ./deploy/stack.sh start
//!
//! `wait_for_ready` uses a process-wide gate so only one test pays the full Surreal WebSocket +
//! signin cost; other tests verify Redis with a short ping. Without this, `cargo test` defaults to
//! many threads and each test can sit silently for up to 15s per Surreal attempt (looks “frozen”).

use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Mutex;

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
        let surrealdb_port =
            std::env::var("SURREALDB_PORT").unwrap_or_else(|_| "50001".to_string());
        let redis_port = std::env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string());
        let surrealdb_url = format!("http://127.0.0.1:{}", surrealdb_port);
        let redis_url = format!("redis://127.0.0.1:{}/", redis_port);
        let surrealdb_ns = std::env::var("SURREAL_NS").unwrap_or_else(|_| "stg_rd".to_string());
        let surrealdb_db = std::env::var("SURREAL_DB").unwrap_or_else(|_| "stg_rd".to_string());
        let surrealdb_user = std::env::var("SURREAL_USER").unwrap_or_else(|_| "root".to_string());
        let surrealdb_pass =
            std::env::var("SURREAL_PASSWORD").unwrap_or_else(|_| "root".to_string());
        log::info!("Test env: SurrealDB={}, Redis={}", surrealdb_url, redis_url);
        eprintln!(
            "Integration tests: SurrealDB {} user={} (SURREAL_USER / SURREAL_PASSWORD; must match `docker compose` --user/--pass and any existing volume).",
            surrealdb_url, surrealdb_user
        );
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
        if STACK_READY.load(Ordering::Acquire) {
            if self.verify_stack_quick().await.is_ok() {
                return Ok(());
            }
            STACK_READY.store(false, Ordering::Release);
        }
        let _guard = lock_stack_wait_or_timeout("wait_for_ready").await?;
        if STACK_READY.load(Ordering::Acquire) {
            if self.verify_stack_quick().await.is_ok() {
                return Ok(());
            }
            STACK_READY.store(false, Ordering::Release);
        }
        let result = self.wait_surreal_and_redis_full().await;
        if result.is_ok() {
            STACK_READY.store(true, Ordering::Release);
        }
        result
    }

    /// Fast path after another test already completed a full wait in this process.
    async fn verify_stack_quick(&self) -> Result<()> {
        eprintln!("Stack ready earlier in this process — quick Redis check…");
        let redis_client = redis::Client::open(self.redis_url()).context("Redis client")?;
        for attempt in 1..=8 {
            let conn_ok = tokio::time::timeout(
                Duration::from_millis(500),
                redis_client.get_async_connection(),
            )
            .await
            .ok()
            .and_then(|r| r.ok())
            .is_some();
            if conn_ok {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
            if attempt == 4 || attempt == 8 {
                eprintln!("  …still checking Redis (attempt {}/8)", attempt);
            }
        }
        anyhow::bail!("Redis quick check failed (stack may have stopped)")
    }

    async fn wait_surreal_and_redis_full(&self) -> Result<()> {
        use surrealdb::engine::remote::ws::Ws;
        use surrealdb::Surreal;
        let ws_url = self
            .surrealdb_url
            .replace("http://", "ws://")
            .replace("https://", "wss://");
        let mut last_connect_err = None::<String>;
        let mut last_auth_err = None::<String>;
        // Shorter slices + more attempts → more frequent eprintln progress (less “silent freeze”).
        let attempt_timeout = Duration::from_secs(5);
        const MAX_ATTEMPTS: u32 = 15;
        for attempt in 0..MAX_ATTEMPTS {
            eprintln!(
                "Waiting for SurrealDB (attempt {}/{}); connect/signin may take up to {}s with no further output until this attempt finishes.",
                attempt + 1,
                MAX_ATTEMPTS,
                attempt_timeout.as_secs()
            );
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
                Err(_) => {
                    last_connect_err = Some(format!(
                        "connect/signin timed out ({}s)",
                        attempt_timeout.as_secs()
                    ))
                }
            }
            if attempt == MAX_ATTEMPTS - 1 {
                let auth_hint = last_auth_err.as_deref().map_or(String::new(), |a| {
                    if a.to_lowercase().contains("auth") {
                        "\n\n\
                            Hint (authentication failed): `SURREAL_USER` / `SURREAL_PASSWORD` must match the running SurrealDB instance. \
                            `deploy/docker-compose.yml` passes them as `surreal start --user/--pass`. \
                            If the DB volume was created with different credentials, sign-in will fail until you use the original password or wipe `surrealdb_data` and restart. \
                            Before `cargo test -p testing`, run: `source scripts/load-env.sh prod` (or `dev`) so env matches the stack."
                            .to_string()
                    } else {
                        String::new()
                    }
                });
                let msg = match (last_connect_err.as_deref(), last_auth_err.as_deref()) {
                    (Some(c), Some(a)) => format!("SurrealDB not ready after {} attempts. Last connect err: {}. Last auth err: {}{}", MAX_ATTEMPTS, c, a, auth_hint),
                    (Some(c), None) => format!("SurrealDB not ready after {} attempts (connect failed: {}). Use SURREAL_URL=http://127.0.0.1:50001 when running tests on host.", MAX_ATTEMPTS, c),
                    (None, Some(a)) => format!("SurrealDB not ready after {} attempts (auth/ns: {}){}", MAX_ATTEMPTS, a, auth_hint),
                    (None, None) => format!("SurrealDB not ready after {} attempts. Use SURREAL_URL=http://127.0.0.1:50001 when running tests on host.", MAX_ATTEMPTS),
                };
                anyhow::bail!("{}", msg);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        let redis_client = redis::Client::open(self.redis_url()).context("Redis client")?;
        for attempt in 0..20 {
            eprintln!("Waiting for Redis (attempt {}/20).", attempt + 1);
            // Redis connect can hang (e.g. network/DNS issues in some environments),
            // so bound it with a short timeout per attempt to avoid "stuck" tests.
            let conn_ok =
                tokio::time::timeout(Duration::from_secs(2), redis_client.get_async_connection())
                    .await
                    .ok()
                    .and_then(|r| r.ok())
                    .is_some();
            if conn_ok {
                log::debug!("Redis ready after {} attempts", attempt + 1);
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        anyhow::bail!("Redis not ready after 20 attempts")
    }
}

/// One test in the process runs the full Surreal handshake; others reuse [`STACK_READY`] + Redis ping.
static STACK_READY: AtomicBool = AtomicBool::new(false);

static STACK_WAIT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Max time to wait for the per-process stack mutex before failing (another test may be stuck on Surreal).
const STACK_LOCK_WAIT_SECS: u64 = 180;

/// Serializes full stack checks and per-test Surreal `connect` in [`crate::app_setup::setup_test_app_data`]
/// so parallel integration tests do not open many WebSocket clients at once (avoids long stalls).
pub(crate) fn stack_wait_lock() -> &'static Mutex<()> {
    STACK_WAIT_LOCK.get_or_init(|| Mutex::new(()))
}

/// Bounded wait for [`stack_wait_lock`] so one stuck test cannot block the suite forever.
pub(crate) async fn lock_stack_wait_or_timeout(
    context: &'static str,
) -> Result<tokio::sync::MutexGuard<'static, ()>, anyhow::Error> {
    tokio::time::timeout(
        Duration::from_secs(STACK_LOCK_WAIT_SECS),
        stack_wait_lock().lock(),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "Timed out after {}s waiting for integration stack lock ({}). Another test may be stuck on Surreal or Redis. Prefer: cargo test -p testing -- --test-threads=1 --nocapture (or cargo test-integration)",
            STACK_LOCK_WAIT_SECS,
            context
        )
    })
}
