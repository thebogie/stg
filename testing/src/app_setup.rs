//! Set up backend app for integration tests using SurrealDB (same stack as CI).

use actix_web::web;
use anyhow::{Context, Result};
use backend::cache::{CacheTTL, RedisCache};
use backend::db::Db;
use backend::player::session::RedisSessionStore;
use std::sync::Arc;
use std::time::Duration;

use super::env::lock_stack_wait_or_timeout;
use super::TestEnvironment;

#[derive(Clone)]
pub struct TestAppData {
    pub redis_data: web::Data<redis::Client>,
    pub db: web::Data<Db>,
    /// Same `Arc` as inside `player_repo` Data — use for `AdminAuthMiddleware` (same DB + scope as handlers).
    pub player_repo_arc: Arc<backend::player::repository::PlayerRepositoryImpl>,
    pub player_repo: web::Data<backend::player::repository::PlayerRepositoryImpl>,
    pub venue_repo: web::Data<backend::venue::repository::VenueRepositoryImpl>,
    pub game_repo: web::Data<backend::game::repository::GameRepositoryImpl>,
    pub contest_repo: web::Data<backend::contest::repository::ContestRepositoryImpl>,
    pub session_store: web::Data<RedisSessionStore>,
    pub redis_arc: Arc<redis::Client>,
}

/// Set up test app data connected to the same stack (SurrealDB + Redis from env).
/// **Call [`TestEnvironment::wait_for_ready`] once before this** (tests already do).
/// Connect/signin/use_ns are wrapped in a 15s timeout so we never hang.
pub async fn setup_test_app_data(env: &TestEnvironment) -> Result<TestAppData> {
    // Serialize with `wait_for_ready`'s full handshake: parallel tests otherwise each open a new WS
    // to Surreal at the same time and can stall the server or the client pool.
    let _setup_guard = lock_stack_wait_or_timeout("setup_test_app_data").await?;
    eprintln!("Opening SurrealDB client for integration test app (up to 15s)…");

    let ws_url = env
        .surrealdb_url
        .replace("http://", "ws://")
        .replace("https://", "wss://");
    let connect = async {
        let db: Db = match TestEnvironment::surreal_socket_addr(&env.surrealdb_url) {
            Some(addr) => surrealdb::Surreal::new::<surrealdb::engine::remote::ws::Ws>(addr)
                .await
                .context("Connect to SurrealDB")?,
            None => surrealdb::Surreal::new::<surrealdb::engine::remote::ws::Ws>(&ws_url)
                .await
                .context("Connect to SurrealDB")?,
        };
        db.signin(surrealdb::opt::auth::Root {
            username: env.surrealdb_user.clone(),
            password: env.surrealdb_pass.clone(),
        })
        .await
        .context("SurrealDB signin")?;
        db.use_ns(&env.surrealdb_ns)
            .use_db(&env.surrealdb_db)
            .await
            .context("SurrealDB use_ns/use_db")?;
        backend::db::ensure_contest_moderation_schema(&db)
            .await
            .context("contest moderation schema bootstrap")?;
        backend::db::ensure_sell_listing_schema(&db)
            .await
            .context("sell listing schema bootstrap")?;
        Ok::<Db, anyhow::Error>(db)
    };
    let db: Db = tokio::time::timeout(Duration::from_secs(15), connect)
        .await
        .map_err(|_| anyhow::anyhow!("SurrealDB connect/signin timed out after 15s (check SURREAL_URL is host-accessible, e.g. http://127.0.0.1:50001)"))??;

    let redis_client = redis::Client::open(env.redis_url()).context("Redis client")?;
    let redis_data = web::Data::new(redis_client.clone());
    let db_data = web::Data::new(db.clone());
    let session_store = web::Data::new(RedisSessionStore {
        client: redis_client.clone(),
    });

    // Use the same Redis-backed cache strategy as production to make read-after-write stable
    // for register→login flows in integration tests.
    let player_cache = Arc::new(RedisCache::new(
        redis_client.clone(),
        "stg:test:cache:player".to_string(),
        CacheTTL::player(),
    ));
    let mut player_repo_impl = backend::player::repository::PlayerRepositoryImpl::new_with_cache(
        db.clone(),
        player_cache.clone(),
    );
    // Scope can be lost across WS connections; set it explicitly per-query in integration tests.
    player_repo_impl.ns = Some(env.surrealdb_ns.clone());
    player_repo_impl.db_name = Some(env.surrealdb_db.clone());
    let player_repo_arc = Arc::new(player_repo_impl.clone());
    let player_repo = web::Data::from(player_repo_arc.clone());
    let venue_repo = web::Data::new(
        backend::venue::repository::VenueRepositoryImpl::new_with_scope(
            db.clone(),
            None,
            env.surrealdb_ns.clone(),
            env.surrealdb_db.clone(),
        ),
    );
    let game_repo = web::Data::new(
        backend::game::repository::GameRepositoryImpl::new_with_scope(
            db.clone(),
            env.surrealdb_ns.clone(),
            env.surrealdb_db.clone(),
        ),
    );
    let contest_repo = web::Data::new(
        backend::contest::repository::ContestRepositoryImpl::new_with_google_config_and_player_repo(
            db.clone(),
            None,
            Some(player_repo_impl.clone()),
            Some(env.surrealdb_ns.clone()),
            Some(env.surrealdb_db.clone()),
        ),
    );

    Ok(TestAppData {
        redis_data,
        db: db_data,
        player_repo_arc,
        player_repo,
        venue_repo,
        game_repo,
        contest_repo,
        session_store,
        redis_arc: Arc::new(redis_client),
    })
}
