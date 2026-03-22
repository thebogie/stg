//! Shared SurrealDB client type for the backend.
//! Use this type everywhere instead of the concrete engine type.
//!
//! **Query convention:** When deserializing into `Vec<serde_json::Value>` (or plain `Value`),
//! record IDs must be selected as strings or deserialization can fail with "invalid type: enum".
//! Use `string::concat(id) AS id` (or equivalent for other record refs) in SELECT. See
//! `docs/SURREALDB_QUERY_CONVENTIONS.md`.

use anyhow::Context;
use surrealdb::engine::remote::ws::Client;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::Surreal;

use crate::config::DatabaseConfig;

/// SurrealDB client (WebSocket). Use for all repository and health code.
pub type Db = Surreal<Client>;

/// Root sign-in + `USE NS` / `USE DB` — same as the `backend` server binary.
pub async fn connect_surreal(database: &DatabaseConfig) -> anyhow::Result<Db> {
    let ws_url = database
        .url
        .replace("http://", "ws://")
        .replace("https://", "wss://");
    log::info!("Connecting to SurrealDB at {}", ws_url);

    let db: Db = match url::Url::parse(&ws_url).ok().and_then(|u| {
        let host = u.host_str()?;
        let ip: std::net::IpAddr = host.parse().ok()?;
        let port = u.port().unwrap_or(50001);
        Some(std::net::SocketAddr::new(ip, port))
    }) {
        Some(addr) => Surreal::new::<Ws>(addr)
            .await
            .with_context(|| format!("connect SurrealDB at {}", addr))?,
        None => Surreal::new::<Ws>(ws_url.as_str())
            .await
            .context("connect SurrealDB")?,
    };

    db.signin(surrealdb::opt::auth::Root {
        username: database.root_username.clone(),
        password: database.root_password.clone(),
    })
    .await
    .context("SurrealDB root signin")?;

    db.use_ns(&database.ns)
        .use_db(&database.name)
        .await
        .context("SurrealDB USE NS/DB")?;

    Ok(db)
}
