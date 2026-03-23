//! Integration tests for the **`bgg_catalog`** search tier (`search_bgg_catalog` → used by game name search).
//!
//! Requires SurrealDB with `bgg_catalog` table defined (e.g. `docs/surreal-schema-minimal-tests.surql` or migrations).

use anyhow::Result;
use std::collections::HashSet;
use testing::{app_setup, TestEnvironment};

use backend::bgg_catalog::repository::search_bgg_catalog;

async fn ensure_db_scope(db: &backend::db::Db, ns: &str, db_name: &str) -> Result<()> {
    db.use_ns(ns)
        .use_db(db_name)
        .await
        .map_err(|e| anyhow::anyhow!("use_ns/use_db: {}", e))?;
    Ok(())
}

/// Insert one catalog row with a unique name token so we do not collide with real data.
async fn seed_bgg_catalog_row(
    db: &backend::db::Db,
    ns: &str,
    db_name: &str,
    bgg_id: i32,
    name: &str,
) -> Result<()> {
    ensure_db_scope(db, ns, db_name).await?;
    let key = bgg_id.to_string();
    // Datetime fields must be Surreal `datetime`, not JSON strings (schema coercion fails on CONTENT $doc).
    let sql = format!(
        "USE NS {}; USE DB {}; UPSERT type::record('bgg_catalog', $key) CONTENT {{ \
         bgg_id: $bgg_id, \
         name: $name, \
         year_published: $year, \
         rank: $rank, \
         imported_at: time::now(), \
         import_batch: 'integration-test' \
         }};",
        ns, db_name
    );
    let mut res = db
        .query(&sql)
        .bind(("key", key))
        .bind(("bgg_id", bgg_id))
        .bind(("name", name.to_string()))
        .bind(("year", 2021i32))
        .bind(("rank", 42i32))
        .await
        .map_err(|e| anyhow::anyhow!("seed bgg_catalog: {}", e))?;
    let _: Vec<serde_json::Value> = res.take(2).map_err(|e| anyhow::anyhow!("take: {}", e))?;
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn search_bgg_catalog_returns_seeded_row() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;

    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_millis()
        % 900_000;
    let bgg_id = 8_100_000 + suffix as i32;
    let token = format!("ZxBggCatToken{}", suffix);
    let name = format!("{} Unique Boardgame Title", token);

    let db = app_data.db.as_ref();
    seed_bgg_catalog_row(db, &env.surrealdb_ns, &env.surrealdb_db, bgg_id, &name).await?;

    ensure_db_scope(db, &env.surrealdb_ns, &env.surrealdb_db).await?;

    let q = token.as_str();
    assert!(q.len() >= 2, "search_bgg_catalog requires query len >= 2");

    let games = search_bgg_catalog(db, q, 10, &HashSet::new()).await;
    let hit = games.iter().find(|g| g.bgg_id == Some(bgg_id));
    assert!(
        hit.is_some(),
        "expected bgg_catalog row for bgg_id={} name={:?}, got {:?}",
        bgg_id,
        name,
        games
    );
    assert_eq!(hit.expect("checked").name, name);

    Ok(())
}
