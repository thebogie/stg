//! Set up backend app for integration tests using SurrealDB (same stack as CI).

use actix_web::web;
use anyhow::{Context, Result};
use backend::db::Db;
use backend::player::session::RedisSessionStore;
use std::sync::Arc;

use super::TestEnvironment;

#[derive(Clone)]
pub struct TestAppData {
    pub redis_data: web::Data<redis::Client>,
    pub player_repo: web::Data<backend::player::repository::PlayerRepositoryImpl>,
    pub venue_repo: web::Data<backend::venue::repository::VenueRepositoryImpl>,
    pub game_repo: web::Data<backend::game::repository::GameRepositoryImpl>,
    pub contest_repo: web::Data<backend::contest::repository::ContestRepositoryImpl>,
    pub session_store: web::Data<RedisSessionStore>,
    pub redis_arc: Arc<redis::Client>,
}

/// Set up test app data connected to the same stack (SurrealDB + Redis from env).
pub async fn setup_test_app_data(env: &TestEnvironment) -> Result<TestAppData> {
    env.wait_for_ready().await?;

    let ws_url = env
        .surrealdb_url
        .replace("http://", "ws://")
        .replace("https://", "wss://");
    let db: Db = surrealdb::Surreal::new::<surrealdb::engine::remote::ws::Ws>(&ws_url)
        .await
        .context("Connect to SurrealDB")?;
    db.signin(surrealdb::opt::auth::Root {
        username: &env.surrealdb_user,
        password: &env.surrealdb_pass,
    })
    .await
    .context("SurrealDB signin")?;
    db.use_ns(&env.surrealdb_ns)
        .use_db(&env.surrealdb_db)
        .await
        .context("SurrealDB use_ns/use_db")?;

    let redis_client =
        redis::Client::open(env.redis_url()).context("Redis client")?;
    let redis_data = web::Data::new(redis_client.clone());
    let session_store = web::Data::new(RedisSessionStore {
        client: redis_client.clone(),
    });

    let player_repo = web::Data::new(backend::player::repository::PlayerRepositoryImpl::new(
        db.clone(),
    ));
    let venue_repo = web::Data::new(backend::venue::repository::VenueRepositoryImpl::new(
        db.clone(),
        None,
    ));
    let game_repo = web::Data::new(backend::game::repository::GameRepositoryImpl::new(
        db.clone(),
    ));
    let contest_repo = web::Data::new(
        backend::contest::repository::ContestRepositoryImpl::new_with_google_config(
            db.clone(),
            None,
        ),
    );

    Ok(TestAppData {
        redis_data,
        player_repo,
        venue_repo,
        game_repo,
        contest_repo,
        session_store,
        redis_arc: Arc::new(redis_client),
    })
}
