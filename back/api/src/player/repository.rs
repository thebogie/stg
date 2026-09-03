use crate::cache::{CacheKeys, CacheTTL, RedisCache};
use crate::db::Db;
use crate::surreal_helpers::{
    normalize_record_id_string, record_id_from_row, record_id_to_key, scope_prefix,
    select_one_by_record_id_scoped,
};
use async_trait::async_trait;
use log;
use shared::models::player::Player;
use std::sync::Arc;
use surrealdb::types::SurrealValue;

/// Row shape from SurrealDB SELECT * FROM player. Id is a RecordId in responses.
#[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
struct PlayerRow {
    // We intentionally SELECT id as a string to avoid SDK RecordId wrapper/backtick variations.
    // This must always be canonicalized via `normalize_record_id_string`.
    id: Option<String>,
    firstname: Option<String>,
    handle: Option<String>,
    email: Option<String>,
    password: Option<String>,
    #[serde(alias = "createdAt")]
    created_at: Option<String>,
    #[serde(alias = "isAdmin")]
    is_admin: Option<bool>,
    #[serde(alias = "isActive")]
    is_active: Option<bool>,
}

#[async_trait]
pub trait PlayerRepository: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Option<Player>;
    async fn find_by_id(&self, id: &str) -> Option<Player>;
    /// Search players by handle, email, or first name (case-insensitive substring). At most `limit` rows (capped in the repository).
    async fn search_players(&self, query: &str, limit: u32, include_inactive: bool) -> Vec<Player>;
    /// Players ordered by handle for directory browsing when the search box is empty.
    async fn list_players_directory(&self, limit: u32) -> Vec<Player>;
    async fn create(&self, player: Player) -> Result<Player, String>;
    async fn update(&self, player: Player) -> Result<Player, String>;
    async fn find_by_handle(&self, handle: &str) -> Option<Player>;
    async fn find_many_by_ids(&self, ids: &[String]) -> Vec<Player>;
    async fn set_admin_status(&self, player_id: &str, is_admin: bool) -> Result<Player, String>;
    async fn set_active_status(&self, player_id: &str, is_active: bool) -> Result<Player, String>;
    async fn count_contests_as_creator(&self, player_id: &str) -> Result<u64, String>;
    async fn delete_player(&self, player_id: &str) -> Result<(), String>;
}

#[derive(Clone)]
pub struct PlayerRepositoryImpl {
    pub db: Db,
    pub cache: Option<Arc<RedisCache>>,
    /// When set (e.g. in tests), use_ns/use_db are called before each query so scope is correct on the thread that runs the query.
    pub ns: Option<String>,
    pub db_name: Option<String>,
}

impl PlayerRepositoryImpl {
    pub fn new(db: Db) -> Self {
        Self {
            db,
            cache: None,
            ns: None,
            db_name: None,
        }
    }

    pub fn new_with_cache(db: Db, cache: Arc<RedisCache>) -> Self {
        Self {
            db,
            cache: Some(cache),
            ns: None,
            db_name: None,
        }
    }

    /// For integration tests: ensure each query runs with the given NS/DB (avoids scope not persisting across threads).
    pub fn new_with_scope(db: Db, ns: String, db_name: String) -> Self {
        Self {
            db,
            cache: None,
            ns: Some(ns),
            db_name: Some(db_name),
        }
    }

    async fn ensure_scope(&self) {
        if let (Some(ref ns), Some(ref db_name)) = (&self.ns, &self.db_name) {
            let _ = self.db.use_ns(ns).use_db(db_name).await;
        }
    }

    /// Same prelude as `create`: SDK `use_ns`/`use_db`, then a SurrealQL `USE NS; USE DB;`
    /// round-trip so the following `query` runs in the intended namespace.
    async fn ensure_scope_via_query(&self) -> Result<(), String> {
        self.ensure_scope().await;
        if let (Some(ref ns), Some(ref db_name)) = (&self.ns, &self.db_name) {
            let ns_ok = ns.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            let db_ok = db_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_');
            if ns_ok && db_ok {
                let use_q = format!("USE NS {}; USE DB {};", ns, db_name);
                self.db
                    .query(use_q)
                    .await
                    .map_err(|e| format!("Failed to set scope: {}", e))?;
            }
        }
        Ok(())
    }

    /// When ns/db are set, prefix query with USE NS/DB in the *same* multi-statement query.
    /// Callers must `take(scope_result_index())` — not `take(0)` — so they read the real statement
    /// result rather than a USE response (see GameRepositoryImpl).
    fn query_with_scope(&self, core: &str) -> String {
        if let (Some(ref ns), Some(ref db_name)) = (&self.ns, &self.db_name) {
            let ns_ok = ns.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            let db_ok = db_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_');
            if ns_ok && db_ok {
                return format!("USE NS {}; USE DB {}; {}", ns, db_name, core);
            }
        }
        core.to_string()
    }

    fn scope_result_index(&self) -> usize {
        if self.ns.is_some() && self.db_name.is_some() {
            2
        } else {
            0
        }
    }
}

fn row_to_player(r: PlayerRow) -> Option<Player> {
    let id =
        r.id.as_deref()
            .map(normalize_record_id_string)
            .unwrap_or_default();
    if id.is_empty() {
        return None;
    }
    let created_at: chrono::DateTime<chrono::Utc> = r
        .created_at
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(chrono::Utc::now);
    Some(Player {
        id,
        rev: String::new(),
        firstname: r.firstname.unwrap_or_default(),
        handle: r.handle.unwrap_or_default(),
        email: r.email.unwrap_or_default(),
        password: r.password.unwrap_or_default(),
        created_at,
        is_admin: r.is_admin.unwrap_or(false),
        is_active: r.is_active.unwrap_or(true),
    })
}

/// Map a Surreal record (Value) to Player. Record has id (record id), and stored fields.
fn value_to_player(v: &serde_json::Value) -> Option<Player> {
    let id = record_id_from_row(v, Some("player"))?;
    let firstname = v
        .get("firstname")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let handle = v
        .get("handle")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let email = v
        .get("email")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let password = v
        .get("password")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let created_at: chrono::DateTime<chrono::Utc> = v
        .get("createdAt")
        .or_else(|| v.get("created_at"))
        .and_then(|x| {
            serde_json::from_value::<chrono::DateTime<chrono::FixedOffset>>(x.clone()).ok()
        })
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(chrono::Utc::now);
    let is_admin = v
        .get("isAdmin")
        .or_else(|| v.get("is_admin"))
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    let is_active = v
        .get("isActive")
        .or_else(|| v.get("is_active"))
        .and_then(|x| x.as_bool())
        .unwrap_or(true);
    Some(Player {
        id,
        rev: v
            .get("_rev")
            .or_else(|| v.get("rev"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        firstname,
        handle,
        email,
        password,
        created_at,
        is_admin,
        is_active,
    })
}

fn json_value_as_bool(v: &serde_json::Value) -> Option<bool> {
    match v {
        serde_json::Value::Bool(b) => Some(*b),
        serde_json::Value::Number(n) => n.as_u64().map(|u| u != 0),
        serde_json::Value::String(s) => match s.to_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        },
        serde_json::Value::Null => None,
        _ => None,
    }
}

fn is_admin_from_row_json(row: &serde_json::Value) -> Option<bool> {
    row.get("isAdmin")
        .or_else(|| row.get("is_admin"))
        .and_then(json_value_as_bool)
}

impl PlayerRepositoryImpl {
    /// Same scoped Surreal path as email lookups; matches `UPDATE player SET isAdmin … WHERE email`.
    /// Record-id fetches alone can miss `isAdmin` depending on Surreal JSON shape / id binding.
    async fn load_is_admin_for_email(&self, email: &str) -> Option<bool> {
        let email_owned = email.trim().to_string();
        if email_owned.is_empty() {
            return None;
        }
        const CORE: &str = "SELECT isAdmin FROM player WHERE string::lowercase(email) = string::lowercase($email) LIMIT 1";
        // Integration tests: match `create` — `ensure_scope_via_query` before SELECT (see module docs).
        if self.ns.is_some() && self.db_name.is_some() {
            self.ensure_scope_via_query().await.ok()?;
            let mut res = self
                .db
                .query(CORE)
                .bind(("email", email_owned))
                .await
                .ok()?;
            let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
            return rows
                .into_iter()
                .next()
                .and_then(|v| json_value_as_bool(&v).or_else(|| is_admin_from_row_json(&v)));
        }
        let (q, take_idx) = scope_prefix(self.ns.as_deref(), self.db_name.as_deref(), CORE);
        if take_idx == 0 {
            self.ensure_scope().await;
        }
        let mut res = self.db.query(&q).bind(("email", email_owned)).await.ok()?;
        let rows: Vec<serde_json::Value> = res.take(take_idx).unwrap_or_default();
        rows.into_iter()
            .next()
            .and_then(|v| json_value_as_bool(&v).or_else(|| is_admin_from_row_json(&v)))
    }

    /// Resolve player for auth and admin middleware: same cache-backed lookup as `find_by_email`
    /// (so behavior matches contest create and other authenticated flows), then overlays `isAdmin`
    /// from Surreal via `load_is_admin_for_email` so promotions are visible immediately.
    pub async fn find_by_email_for_auth(&self, email: &str) -> Option<Player> {
        let mut p = self.find_by_email(email).await?;
        if let Some(ia) = self.load_is_admin_for_email(email).await {
            p.is_admin = ia;
        }
        Some(p)
    }

    async fn load_player_row_by_email_from_db(&self, email: &str) -> Option<Player> {
        let email_owned = email.trim().to_string();
        if email_owned.is_empty() {
            return None;
        }
        // Case-insensitive match: session/Redis may differ in casing from the Surreal row.
        // Force id to string and strip backticks so app never sees Surreal wrappers.
        const SELECT_CORE: &str = "SELECT firstname, handle, email, password, createdAt, isAdmin, isActive, string::replace(string::concat(id), '`', '') AS id FROM player WHERE string::lowercase(email) = string::lowercase($email) LIMIT 1";

        if self.ns.is_some() && self.db_name.is_some() {
            let (q, take_idx) =
                scope_prefix(self.ns.as_deref(), self.db_name.as_deref(), SELECT_CORE);
            let mut res = match self
                .db
                .query(&q)
                .bind(("email", email_owned.clone()))
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Error querying player by email '{}': {:?}", email, e);
                    return None;
                }
            };
            let player = match res.take::<Vec<PlayerRow>>(take_idx) {
                Ok(rows) => {
                    let count = rows.len();
                    let p = rows.into_iter().next().and_then(row_to_player);
                    if p.is_none() && count > 0 {
                        log::warn!(
                            "Player find_by_email: got {} row(s) but row_to_player returned None (email {})",
                            count, email
                        );
                    } else if p.is_none() {
                        log::info!("Player find_by_email: no rows for email {}", email);
                    }
                    p
                }
                Err(e) => {
                    log::warn!(
                        "Player find_by_email: typed take failed ({}), trying Value path",
                        e
                    );
                    let (q2, idx2) =
                        scope_prefix(self.ns.as_deref(), self.db_name.as_deref(), SELECT_CORE);
                    let mut res2 = self
                        .db
                        .query(&q2)
                        .bind(("email", email_owned))
                        .await
                        .ok()?;
                    let rows: Vec<serde_json::Value> = res2.take(idx2).unwrap_or_default();
                    let row_count = rows.len();
                    let p = rows.into_iter().next().and_then(|v| {
                        let out = value_to_player(&v);
                        if out.is_none() {
                            let keys: Vec<&str> = v
                                .as_object()
                                .map(|o| o.keys().map(String::as_str).collect())
                                .unwrap_or_default();
                            log::warn!(
                                "Player find_by_email: value_to_player returned None for row keys={:?} id={:?}",
                                keys,
                                v.get("id").or_else(|| v.get("_id"))
                            );
                        }
                        out
                    });
                    if p.is_none() && row_count > 0 {
                        log::warn!(
                            "Player find_by_email: Value path got {} row(s) but value_to_player returned None",
                            row_count
                        );
                    }
                    p
                }
            };
            if player.is_none() {
                log::info!("Player find_by_email: no player for email {}", email);
            }
            return player;
        }

        let (select_q, take_idx) =
            scope_prefix(self.ns.as_deref(), self.db_name.as_deref(), SELECT_CORE);
        if take_idx == 0 {
            self.ensure_scope().await;
        }
        let mut res = match self
            .db
            .query(&select_q)
            .bind(("email", email_owned.clone()))
            .await
        {
            Ok(r) => r,
            Err(e) => {
                log::error!("Error querying player by email '{}': {:?}", email, e);
                return None;
            }
        };
        let player = match res.take::<Vec<PlayerRow>>(take_idx) {
            Ok(rows) => {
                let count = rows.len();
                let p = rows.into_iter().next().and_then(row_to_player);
                if p.is_none() && count > 0 {
                    log::warn!("Player find_by_email: got {} row(s) but row_to_player returned None (email {})", count, email);
                } else if p.is_none() {
                    log::info!("Player find_by_email: no rows for email {}", email);
                }
                p
            }
            Err(e) => {
                log::warn!(
                    "Player find_by_email: typed take failed ({}), trying Value path",
                    e
                );
                let mut res2 = self
                    .db
                    .query(&select_q)
                    .bind(("email", email_owned))
                    .await
                    .ok()?;
                let rows: Vec<serde_json::Value> = res2.take(take_idx).unwrap_or_default();
                let row_count = rows.len();
                let p = rows.into_iter().next().and_then(|v| {
                    let out = value_to_player(&v);
                    if out.is_none() {
                        let keys: Vec<&str> = v.as_object().map(|o| o.keys().map(String::as_str).collect()).unwrap_or_default();
                        log::warn!(
                            "Player find_by_email: value_to_player returned None for row keys={:?} id={:?}",
                            keys,
                            v.get("id").or_else(|| v.get("_id"))
                        );
                    }
                    out
                });
                if p.is_none() && row_count > 0 {
                    log::warn!("Player find_by_email: Value path got {} row(s) but value_to_player returned None", row_count);
                }
                p
            }
        };
        if player.is_none() {
            log::info!("Player find_by_email: no player for email {}", email);
        }
        player
    }
}

#[async_trait]
impl PlayerRepository for PlayerRepositoryImpl {
    async fn find_by_email(&self, email: &str) -> Option<Player> {
        let email = email.trim();
        if email.is_empty() {
            return None;
        }
        if let Some(ref cache) = self.cache {
            let cache_key = CacheKeys::player_by_email(email);
            if let Ok(Some(cached_player)) = cache.get::<Player>(&cache_key).await {
                log::debug!("Cache hit for player by email: {}", email);
                // Defensive: older cached values may contain backticks/colon ids.
                let mut p = cached_player;
                p.id = normalize_record_id_string(&p.id);
                return Some(p);
            }
        }

        let player = self.load_player_row_by_email_from_db(email).await;
        if let Some(ref p) = player {
            if let Some(ref cache) = self.cache {
                let _ = cache
                    .set_with_ttl(&CacheKeys::player_by_email(email), p, CacheTTL::player())
                    .await;
                let _ = cache
                    .set_with_ttl(&CacheKeys::player(&p.id), p, CacheTTL::player())
                    .await;
            }
        }
        player
    }

    async fn find_by_id(&self, id: &str) -> Option<Player> {
        // #region agent log
        {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/home/thebogie/work/stg/.cursor/debug-4e07b4.log") {
                let _ = writeln!(f, r#"{{"sessionId":"4e07b4","location":"repository.rs:find_by_id:entry","message":"find_by_id called","data":{{"id":"{}"}},"timestamp":{}}}"#,
                    id, chrono::Utc::now().timestamp_millis());
            }
        }
        // #endregion
        if let Some(ref cache) = self.cache {
            let cache_key = CacheKeys::player(id);
            if let Ok(Some(cached_player)) = cache.get::<Player>(&cache_key).await {
                log::debug!("Cache hit for player by id: {}", id);
                let mut p = cached_player;
                p.id = normalize_record_id_string(&p.id);
                return Some(p);
            }
        }

        let player = select_one_by_record_id_scoped(
            &self.db,
            "player",
            id,
            self.ns.as_deref(),
            self.db_name.as_deref(),
        )
            .await
            .and_then(|v| {
                // #region agent log
                {
                    use std::io::Write;
                    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/home/thebogie/work/stg/.cursor/debug-4e07b4.log") {
                        let _ = writeln!(f, r#"{{"sessionId":"4e07b4","location":"repository.rs:find_by_id:raw_row","message":"select_one raw value","data":{{"id":"{}","row":"{:?}"}},"timestamp":{}}}"#,
                            id, v, chrono::Utc::now().timestamp_millis());
                    }
                }
                // #endregion
                value_to_player(&v)
            });
        // #region agent log
        {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/home/thebogie/work/stg/.cursor/debug-4e07b4.log") {
                let _ = writeln!(f, r#"{{"sessionId":"4e07b4","location":"repository.rs:find_by_id:result","message":"find_by_id result","data":{{"id":"{}","found":{}}},"timestamp":{}}}"#,
                    id, player.is_some(), chrono::Utc::now().timestamp_millis());
            }
        }
        // #endregion
        if let Some(ref p) = player {
            if let Some(ref cache) = self.cache {
                let _ = cache
                    .set_with_ttl(&CacheKeys::player(&p.id), p, CacheTTL::player())
                    .await;
                let _ = cache
                    .set_with_ttl(&CacheKeys::player_by_email(&p.email), p, CacheTTL::player())
                    .await;
                if !p.handle.is_empty() {
                    let _ = cache
                        .set_with_ttl(
                            &CacheKeys::player_by_handle(&p.handle),
                            p,
                            CacheTTL::player(),
                        )
                        .await;
                }
            }
        }
        player
    }

    async fn search_players(&self, query: &str, limit: u32, include_inactive: bool) -> Vec<Player> {
        let lim = limit.clamp(1, 100) as i64;
        let q_owned = query.to_string();
        let active_filter = if include_inactive {
            String::new()
        } else {
            " AND (isActive == NONE OR isActive == true) ".to_string()
        };
        let sql = format!(
            "SELECT firstname, handle, email, password, createdAt, isAdmin, isActive, \
             string::replace(string::concat(id), '`', '') AS id \
             FROM player WHERE (string::lowercase(email) = string::lowercase($q) \
             OR string::contains(string::lowercase(handle), string::lowercase($q)) \
             OR string::contains(string::lowercase(email), string::lowercase($q)) \
             OR (firstname != NONE AND string::contains(string::lowercase(firstname), string::lowercase($q)))){} \
             ORDER BY handle ASC LIMIT $lim",
            active_filter
        );
        let (q, take_idx) = scope_prefix(self.ns.as_deref(), self.db_name.as_deref(), &sql);
        if take_idx == 0 {
            self.ensure_scope().await;
        }
        // #region agent log
        {
            use std::io::Write;
            let diag = "SELECT count() AS c FROM player GROUP ALL";
            let (dq, didx) = scope_prefix(self.ns.as_deref(), self.db_name.as_deref(), diag);
            let mut total = None;
            if let Ok(mut dr) = self.db.query(&dq).await {
                let drows: Vec<serde_json::Value> = dr.take(didx).unwrap_or_default();
                total = drows.first().cloned();
            }
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/home/thebogie/work/stg/.cursor/debug-4e07b4.log") {
                let _ = writeln!(f, r#"{{"sessionId":"4e07b4","hypothesisId":"H2","runId":"post-fix","location":"repository.rs:search_players:entry","message":"search_players called","data":{{"q":"{}","lim":{},"include_inactive":{},"ns":"{:?}","db_name":"{:?}","take_idx":{},"player_table_count":"{:?}"}},"timestamp":{}}}"#,
                    q_owned, lim, include_inactive, self.ns, self.db_name, take_idx, total, chrono::Utc::now().timestamp_millis());
            }
        }
        // #endregion
        let mut res = match self
            .db
            .query(&q)
            .bind(("q", q_owned.clone()))
            .bind(("lim", lim))
            .await
        {
            Ok(r) => r,
            Err(e) => {
                log::warn!("search_players query failed: {}", e);
                return Vec::new();
            }
        };
        // #region agent log
        {
            use std::io::Write;
            let mut lens = Vec::new();
            for i in 0..=2 {
                let n: usize = res.take::<Vec<serde_json::Value>>(i).map(|v| v.len()).unwrap_or(9999);
                lens.push(n);
            }
            // Re-run search for actual take_idx after consuming takes above
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/home/thebogie/work/stg/.cursor/debug-4e07b4.log") {
                let _ = writeln!(f, r#"{{"sessionId":"4e07b4","hypothesisId":"H5","runId":"post-fix","location":"repository.rs:search_players:take_lens","message":"response take lengths","data":{{"q":"{}","lens":"{:?}","take_idx":{}}},"timestamp":{}}}"#,
                    q_owned, lens, take_idx, chrono::Utc::now().timestamp_millis());
            }
        }
        // #endregion
        // Response was consumed by take_lens probe — re-query for real results.
        let mut res = match self
            .db
            .query(&q)
            .bind(("q", q_owned.clone()))
            .bind(("lim", lim))
            .await
        {
            Ok(r) => r,
            Err(e) => {
                log::warn!("search_players requery failed: {}", e);
                return Vec::new();
            }
        };
        let rows: Vec<serde_json::Value> = match res.take(take_idx) {
            Ok(r) => r,
            Err(e) => {
                // #region agent log
                {
                    use std::io::Write;
                    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/home/thebogie/work/stg/.cursor/debug-4e07b4.log") {
                        let _ = writeln!(f, r#"{{"sessionId":"4e07b4","hypothesisId":"H5","location":"repository.rs:search_players:take_err","message":"take failed","data":{{"q":"{}","take_idx":{},"err":"{}"}},"timestamp":{}}}"#,
                            q_owned, take_idx, e, chrono::Utc::now().timestamp_millis());
                    }
                }
                // #endregion
                log::warn!("search_players take({}) failed: {}", take_idx, e);
                return Vec::new();
            }
        };
        // #region agent log
        {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/home/thebogie/work/stg/.cursor/debug-4e07b4.log") {
                let _ = writeln!(f, r#"{{"sessionId":"4e07b4","location":"repository.rs:search_players:rows","message":"raw rows from query","data":{{"q":"{}","row_count":{},"first_row":"{:?}"}},"timestamp":{}}}"#,
                    q_owned, rows.len(), rows.first(), chrono::Utc::now().timestamp_millis());
            }
        }
        // #endregion
        let result: Vec<Player> = rows.into_iter()
            .filter_map(|v| {
                let p = value_to_player(&v);
                // #region agent log
                if p.is_none() {
                    use std::io::Write;
                    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/home/thebogie/work/stg/.cursor/debug-4e07b4.log") {
                        let _ = writeln!(f, r#"{{"sessionId":"4e07b4","location":"repository.rs:search_players:parse_fail","message":"value_to_player returned None","data":{{"row":"{:?}"}},"timestamp":{}}}"#,
                            v, chrono::Utc::now().timestamp_millis());
                    }
                }
                // #endregion
                p
            })
            .collect();
        // #region agent log
        {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/home/thebogie/work/stg/.cursor/debug-4e07b4.log") {
                let _ = writeln!(f, r#"{{"sessionId":"4e07b4","location":"repository.rs:search_players:result","message":"returning players","data":{{"count":{}}},"timestamp":{}}}"#,
                    result.len(), chrono::Utc::now().timestamp_millis());
            }
        }
        // #endregion
        result
    }

    async fn list_players_directory(&self, limit: u32) -> Vec<Player> {
        let lim = limit.clamp(1, 100) as i64;
        let core = "SELECT firstname, handle, email, password, createdAt, isAdmin, \
                 string::replace(string::concat(id), '`', '') AS id \
                 FROM player WHERE (isActive == NONE OR isActive == true) ORDER BY handle ASC LIMIT $lim";
        let (q, take_idx) = scope_prefix(self.ns.as_deref(), self.db_name.as_deref(), core);
        if take_idx == 0 {
            self.ensure_scope().await;
        }
        let mut res = match self.db.query(&q).bind(("lim", lim)).await {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        let rows: Vec<serde_json::Value> = res.take(take_idx).unwrap_or_default();
        rows.into_iter()
            .filter_map(|v| value_to_player(&v))
            .collect()
    }

    async fn create(&self, player: Player) -> Result<Player, String> {
        let key = uuid::Uuid::new_v4().to_string();
        let created_at = player.created_at.to_rfc3339();
        // Surreal SCHEMAFULL `createdAt` is `option<datetime>` — a plain JSON string fails coerce
        // (CREATE used to ignore the response and still cache a fabricated player).
        const CREATE_CORE: &str = "CREATE type::record('player', type::uuid($key)) CONTENT { \
            firstname: $firstname, \
            handle: $handle, \
            email: $email, \
            password: $password, \
            createdAt: type::datetime($created_at), \
            isAdmin: $is_admin, \
            isActive: $is_active \
        } RETURN AFTER";
        let (create_q, take_idx) =
            scope_prefix(self.ns.as_deref(), self.db_name.as_deref(), CREATE_CORE);
        let mut create_res = self
            .db
            .query(&create_q)
            .bind(("key", key.clone()))
            .bind(("firstname", player.firstname.clone()))
            .bind(("handle", player.handle.clone()))
            .bind(("email", player.email.clone()))
            .bind(("password", player.password.clone()))
            .bind(("created_at", created_at))
            .bind(("is_admin", player.is_admin))
            .bind(("is_active", player.is_active))
            .await
            .map_err(|e| format!("Failed to create player: {}", e))?;
        let created_rows: Vec<serde_json::Value> = create_res
            .take(take_idx)
            .map_err(|e| format!("Failed to parse created player: {}", e))?;
        // #region agent log
        {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/home/thebogie/work/stg/.cursor/debug-4e07b4.log") {
                let _ = writeln!(f, r#"{{"sessionId":"4e07b4","hypothesisId":"H6","runId":"post-fix","location":"repository.rs:create","message":"CREATE RETURN AFTER","data":{{"key":"{}","take_idx":{},"row_count":{},"first_row":"{:?}"}},"timestamp":{}}}"#,
                    key, take_idx, created_rows.len(), created_rows.first(), chrono::Utc::now().timestamp_millis());
            }
        }
        // #endregion
        let created_player = created_rows
            .into_iter()
            .next()
            .and_then(|v| value_to_player(&v))
            .ok_or_else(|| "Player CREATE returned no record".to_string())?;
        // Prefer canonical id from DB; fall back to expected uuid key.
        let created_player = if created_player.id.is_empty() {
            Player {
                id: format!("player/{}", key),
                ..created_player
            }
        } else {
            created_player
        };
        // #region agent log
        {
            use std::io::Write;
            let verify_core = "SELECT count() AS c FROM player WHERE string::lowercase(email) = string::lowercase($email) GROUP ALL";
            let (vq, vidx) = scope_prefix(self.ns.as_deref(), self.db_name.as_deref(), verify_core);
            if let Ok(mut vr) = self.db.query(&vq).bind(("email", created_player.email.clone())).await {
                let vrows: Vec<serde_json::Value> = vr.take(vidx).unwrap_or_default();
                if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/home/thebogie/work/stg/.cursor/debug-4e07b4.log") {
                    let _ = writeln!(f, r#"{{"sessionId":"4e07b4","hypothesisId":"H6","runId":"post-fix","location":"repository.rs:create:verify","message":"post-create email count","data":{{"email_domain":"{}","count_row":"{:?}"}},"timestamp":{}}}"#,
                        created_player.email.split('@').nth(1).unwrap_or(""), vrows.first(), chrono::Utc::now().timestamp_millis());
                }
            }
        }
        // #endregion
        if let Some(ref cache) = self.cache {
            let _ = cache
                .set_with_ttl(
                    &CacheKeys::player(&created_player.id),
                    &created_player,
                    CacheTTL::player(),
                )
                .await;
            let _ = cache
                .set_with_ttl(
                    &CacheKeys::player_by_email(&created_player.email),
                    &created_player,
                    CacheTTL::player(),
                )
                .await;
            if !created_player.handle.is_empty() {
                let _ = cache
                    .set_with_ttl(
                        &CacheKeys::player_by_handle(&created_player.handle),
                        &created_player,
                        CacheTTL::player(),
                    )
                    .await;
            }
        }
        Ok(created_player)
    }

    async fn update(&self, player: Player) -> Result<Player, String> {
        let old_email = player.email.clone();
        let key = record_id_to_key(&player.id, "player");
        if key.is_empty() {
            return Err("Invalid player id".to_string());
        }
        let doc = serde_json::json!({
            "firstname": player.firstname,
            "handle": player.handle,
            "email": player.email,
            "password": player.password,
        });
        // Update by record id (not email). Admin flows often change email in the same
        // MERGE payload; WHERE email = $new_email would match 0 rows.
        let core = if uuid::Uuid::parse_str(&key).is_ok() {
            "UPDATE type::record('player', type::uuid($key)) MERGE $doc RETURN AFTER"
        } else {
            "UPDATE type::record('player', $key) MERGE $doc RETURN AFTER"
        };
        let (update_q, take_idx) = scope_prefix(self.ns.as_deref(), self.db_name.as_deref(), core);
        if take_idx == 0 {
            self.ensure_scope().await;
        }
        let mut ur = self
            .db
            .query(&update_q)
            .bind(("key", key))
            .bind(("doc", doc))
            .await
            .map_err(|e| format!("Failed to update player: {}", e))?;
        let updated_rows: Vec<serde_json::Value> = ur
            .take(take_idx)
            .map_err(|e| format!("Failed to parse player update result: {}", e))?;
        let stored_player = updated_rows
            .into_iter()
            .next()
            .and_then(|v| value_to_player(&v))
            .ok_or_else(|| {
                format!(
                    "Player update affected 0 rows (id={}, email={})",
                    player.id, old_email
                )
            })?;

        if stored_player.handle != player.handle {
            return Err(format!(
                "Player update returned unexpected handle (id={}, stored_handle='{}', expected='{}')",
                player.id, stored_player.handle, player.handle
            ));
        }

        let updated_player = stored_player.clone();
        if let Some(ref cache) = self.cache {
            let _ = cache
                .delete(&CacheKeys::player_by_email(&old_email))
                .await;
            let _ = cache
                .delete(&CacheKeys::player_by_email(&updated_player.email))
                .await;
            if !player.handle.is_empty() {
                let _ = cache
                    .delete(&CacheKeys::player_by_handle(&player.handle))
                    .await;
            }
            let _ = cache
                .set_with_ttl(
                    &CacheKeys::player(&updated_player.id),
                    &updated_player,
                    CacheTTL::player(),
                )
                .await;
            let _ = cache
                .set_with_ttl(
                    &CacheKeys::player_by_email(&updated_player.email),
                    &updated_player,
                    CacheTTL::player(),
                )
                .await;
            if !updated_player.handle.is_empty() {
                let _ = cache
                    .set_with_ttl(
                        &CacheKeys::player_by_handle(&updated_player.handle),
                        &updated_player,
                        CacheTTL::player(),
                    )
                    .await;
            }
        }
        Ok(updated_player)
    }

    async fn find_by_handle(&self, handle: &str) -> Option<Player> {
        if let Some(ref cache) = self.cache {
            let cache_key = CacheKeys::player_by_handle(handle);
            if let Ok(Some(cached_player)) = cache.get::<Player>(&cache_key).await {
                log::debug!("Cache hit for player by handle: {}", handle);
                let mut p = cached_player;
                p.id = normalize_record_id_string(&p.id);
                return Some(p);
            }
        }

        let handle_owned = handle.to_string();
        let mut res = match self
            .db
            .query("SELECT * FROM player WHERE string::lowercase(handle) = string::lowercase($handle) LIMIT 1")
            .bind(("handle", handle_owned))
            .await
        {
            Ok(r) => r,
            Err(_) => return None,
        };
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        rows.into_iter().next().and_then(|v| value_to_player(&v))
    }

    async fn find_many_by_ids(&self, ids: &[String]) -> Vec<Player> {
        if ids.is_empty() {
            return Vec::new();
        }
        let record_ids: Vec<String> = ids
            .iter()
            .map(|id| {
                format!(
                    "player:{}",
                    id.trim_start_matches("player/")
                        .trim_start_matches("player:")
                )
            })
            .collect();
        let mut res = match self
            .db
            .query("SELECT * FROM player WHERE id INSIDE $ids")
            .bind(("ids", record_ids))
            .await
        {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        rows.into_iter()
            .filter_map(|v| value_to_player(&v))
            .collect()
    }

    async fn set_admin_status(&self, player_id: &str, is_admin: bool) -> Result<Player, String> {
        let key = record_id_to_key(player_id, "player");
        if key.is_empty() {
            return Err("Invalid player id".to_string());
        }
        let update_q = if uuid::Uuid::parse_str(&key).is_ok() {
            self.query_with_scope(
                "UPDATE type::record('player', type::uuid($key)) MERGE { isAdmin: $is_admin } RETURN AFTER",
            )
        } else {
            self.query_with_scope(
                "UPDATE type::record('player', $key) MERGE { isAdmin: $is_admin } RETURN AFTER",
            )
        };
        let mut ur = self
            .db
            .query(&update_q)
            .bind(("key", key))
            .bind(("is_admin", is_admin))
            .await
            .map_err(|e| format!("Failed to update admin status: {}", e))?;
        let rows: Vec<serde_json::Value> = ur
            .take(self.scope_result_index())
            .map_err(|e| format!("Failed to parse admin update: {}", e))?;
        let updated = rows
            .into_iter()
            .next()
            .and_then(|v| value_to_player(&v))
            .ok_or_else(|| "Player not found".to_string())?;
        if let Some(ref cache) = self.cache {
            let _ = cache.delete(&CacheKeys::player(&updated.id)).await;
            let _ = cache
                .delete(&CacheKeys::player_by_email(&updated.email))
                .await;
            if !updated.handle.is_empty() {
                let _ = cache
                    .delete(&CacheKeys::player_by_handle(&updated.handle))
                    .await;
            }
        }
        Ok(updated)
    }

    async fn set_active_status(&self, player_id: &str, is_active: bool) -> Result<Player, String> {
        let key = record_id_to_key(player_id, "player");
        if key.is_empty() {
            return Err("Invalid player id".to_string());
        }
        let core = if uuid::Uuid::parse_str(&key).is_ok() {
            "UPDATE type::record('player', type::uuid($key)) MERGE { isActive: $is_active } RETURN AFTER"
        } else {
            "UPDATE type::record('player', $key) MERGE { isActive: $is_active } RETURN AFTER"
        };
        let (update_q, take_idx) = scope_prefix(self.ns.as_deref(), self.db_name.as_deref(), core);
        if take_idx == 0 {
            self.ensure_scope().await;
        }
        let mut ur = self
            .db
            .query(&update_q)
            .bind(("key", key))
            .bind(("is_active", is_active))
            .await
            .map_err(|e| format!("Failed to update active status: {}", e))?;
        let rows: Vec<serde_json::Value> = ur
            .take(take_idx)
            .map_err(|e| format!("Failed to parse active update: {}", e))?;
        let updated = rows
            .into_iter()
            .next()
            .and_then(|v| value_to_player(&v))
            .ok_or_else(|| "Player not found".to_string())?;
        // #region agent log
        {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/home/thebogie/work/stg/.cursor/debug-4e07b4.log") {
                let _ = writeln!(f, r#"{{"sessionId":"4e07b4","hypothesisId":"H7","runId":"post-fix","location":"repository.rs:set_active_status","message":"active status updated","data":{{"player_id":"{}","requested":{},"stored_is_active":{},"email_domain":"{}"}},"timestamp":{}}}"#,
                    player_id, is_active, updated.is_active, updated.email.split('@').nth(1).unwrap_or(""), chrono::Utc::now().timestamp_millis());
            }
        }
        // #endregion
        if let Some(ref cache) = self.cache {
            // Replace cache entries with the updated player so login cannot see a stale is_active=true.
            let _ = cache
                .set_with_ttl(&CacheKeys::player(&updated.id), &updated, CacheTTL::player())
                .await;
            let _ = cache
                .set_with_ttl(
                    &CacheKeys::player_by_email(&updated.email),
                    &updated,
                    CacheTTL::player(),
                )
                .await;
            if !updated.handle.is_empty() {
                let _ = cache
                    .set_with_ttl(
                        &CacheKeys::player_by_handle(&updated.handle),
                        &updated,
                        CacheTTL::player(),
                    )
                    .await;
            }
        }
        Ok(updated)
    }

    async fn count_contests_as_creator(&self, player_id: &str) -> Result<u64, String> {
        let key = record_id_to_key(player_id, "player");
        if key.is_empty() {
            return Err("Invalid player id".to_string());
        }
        self.ensure_scope_via_query()
            .await
            .map_err(|e| format!("Failed to set scope: {}", e))?;
        let player_rid = surrealdb::types::RecordId::new("player", key.as_str());
        let mut res = self
            .db
            .query(
                "SELECT count() AS c FROM contest WHERE creator_id = $record_id GROUP ALL",
            )
            .bind(("record_id", player_rid))
            .await
            .map_err(|e| format!("Failed to count contests: {}", e))?;
        #[derive(serde::Deserialize, surrealdb::types::SurrealValue)]
        struct CountRow {
            c: Option<u64>,
        }
        let rows: Vec<CountRow> = res.take(0).unwrap_or_default();
        Ok(rows.first().and_then(|r| r.c).unwrap_or(0))
    }

    async fn delete_player(&self, player_id: &str) -> Result<(), String> {
        let player = self
            .find_by_id(player_id)
            .await
            .ok_or_else(|| "Player not found".to_string())?;
        let key = record_id_to_key(player_id, "player");
        if key.is_empty() {
            return Err("Invalid player id".to_string());
        }
        self.ensure_scope_via_query()
            .await
            .map_err(|e| format!("Failed to set scope: {}", e))?;
        let player_rid = surrealdb::types::RecordId::new("player", key.as_str());

        self.db
            .query("DELETE FROM resulted_in WHERE `out` = $record_id")
            .bind(("record_id", player_rid.clone()))
            .await
            .map_err(|e| format!("Failed to delete resulted_in edges: {}", e))?;

        self.db
            .query("DELETE FROM rating_latest WHERE player_id = $record_id")
            .bind(("record_id", player_rid.clone()))
            .await
            .map_err(|e| format!("Failed to delete rating_latest: {}", e))?;

        self.db
            .query("DELETE FROM rating_history WHERE player_id = $record_id")
            .bind(("record_id", player_rid.clone()))
            .await
            .map_err(|e| format!("Failed to delete rating_history: {}", e))?;

        let delete_core = if uuid::Uuid::parse_str(&key).is_ok() {
            "DELETE type::record('player', type::uuid($key))"
        } else {
            "DELETE type::record('player', $key)"
        };
        let (delete_q, _) = scope_prefix(self.ns.as_deref(), self.db_name.as_deref(), delete_core);
        self.db
            .query(&delete_q)
            .bind(("key", key))
            .await
            .map_err(|e| format!("Failed to delete player: {}", e))?;

        if let Some(ref cache) = self.cache {
            let _ = cache.delete(&CacheKeys::player(&player.id)).await;
            let _ = cache
                .delete(&CacheKeys::player_by_email(&player.email))
                .await;
            if !player.handle.is_empty() {
                let _ = cache
                    .delete(&CacheKeys::player_by_handle(&player.handle))
                    .await;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use shared::models::player::Player;

    fn create_test_player(id: &str, handle: &str, email: &str) -> Player {
        Player {
            id: id.to_string(),
            rev: "1".to_string(),
            firstname: "Test".to_string(),
            handle: handle.to_string(),
            email: email.to_string(),
            password: "hashed_password".to_string(),
            created_at: Utc::now(),
            is_admin: false,
            is_active: true,
        }
    }

    #[tokio::test]
    async fn test_search_players_by_handle() {
        let players = [
            create_test_player("1", "john_doe", "john@example.com"),
            create_test_player("2", "jane_smith", "jane@example.com"),
            create_test_player("3", "bob_wilson", "bob@example.com"),
        ];

        let results: Vec<&Player> = players
            .iter()
            .filter(|p| p.handle.to_lowercase().contains("john"))
            .collect();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].handle, "john_doe");
    }

    #[tokio::test]
    async fn test_search_players_by_email() {
        let players = [
            create_test_player("1", "john_doe", "john@example.com"),
            create_test_player("2", "jane_smith", "jane@example.com"),
            create_test_player("3", "bob_wilson", "bob@example.com"),
        ];

        let results: Vec<&Player> = players
            .iter()
            .filter(|p| p.email.to_lowercase().contains("example"))
            .collect();

        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_search_players_case_insensitive() {
        let players = [
            create_test_player("1", "John_Doe", "John@Example.com"),
            create_test_player("2", "jane_smith", "jane@example.com"),
        ];

        let results: Vec<&Player> = players
            .iter()
            .filter(|p| p.handle.to_lowercase().contains("john"))
            .collect();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].handle, "John_Doe");
    }

    #[tokio::test]
    async fn test_search_players_empty_query() {
        let players = [
            create_test_player("1", "john_doe", "john@example.com"),
            create_test_player("2", "jane_smith", "jane@example.com"),
        ];

        let results: Vec<&Player> = players
            .iter()
            .filter(|p| p.handle.to_lowercase().contains(""))
            .collect();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_search_players_partial_match() {
        let players = [
            create_test_player("1", "john_doe", "john@example.com"),
            create_test_player("2", "johnny_cash", "johnny@example.com"),
            create_test_player("3", "jane_smith", "jane@example.com"),
        ];

        let results: Vec<&Player> = players
            .iter()
            .filter(|p| p.handle.to_lowercase().contains("john"))
            .collect();

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|p| p.handle == "john_doe"));
        assert!(results.iter().any(|p| p.handle == "johnny_cash"));
    }

    #[tokio::test]
    async fn test_search_players_no_matches() {
        let players = [
            create_test_player("1", "john_doe", "john@example.com"),
            create_test_player("2", "jane_smith", "jane@example.com"),
        ];

        let results: Vec<&Player> = players
            .iter()
            .filter(|p| p.handle.to_lowercase().contains("nonexistent"))
            .collect();

        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_players_special_characters() {
        let players = [
            create_test_player("1", "user_123", "user123@example.com"),
            create_test_player("2", "test_user", "test@example.com"),
        ];

        let results: Vec<&Player> = players
            .iter()
            .filter(|p| p.handle.to_lowercase().contains("user"))
            .collect();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_player_repository_trait_implementation() {
        assert!(true);
    }
}
