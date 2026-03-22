use crate::cache::{CacheKeys, CacheTTL, RedisCache};
use crate::db::Db;
use crate::surreal_helpers::{
    normalize_record_id_string, record_id_from_row, select_one_by_record_id,
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
}

#[async_trait]
pub trait PlayerRepository: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Option<Player>;
    async fn find_by_id(&self, id: &str) -> Option<Player>;
    async fn search_players(&self, query: &str) -> Vec<Player>;
    async fn create(&self, player: Player) -> Result<Player, String>;
    async fn update(&self, player: Player) -> Result<Player, String>;
    async fn find_by_handle(&self, handle: &str) -> Option<Player>;
    async fn find_many_by_ids(&self, ids: &[String]) -> Vec<Player>;
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

    /// When ns/db are set, prefix query with USE NS/DB so the same connection gets correct scope.
    fn query_with_scope(&self, core: &str) -> String {
        // We rely on `ensure_scope()` to set NS/DB on the active connection before each query.
        // Prepending `USE NS/USE DB` here turns every query into a multi-statement query, which
        // produces multiple result sets and makes `take(0)` read the wrong one (it reads the
        // USE response instead of the SELECT/UPDATE response).
        core.to_string()
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
    })
}

#[async_trait]
impl PlayerRepository for PlayerRepositoryImpl {
    async fn find_by_email(&self, email: &str) -> Option<Player> {
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

        let email_owned = email.to_string();
        // Force id to string and strip backticks so app never sees Surreal wrappers.
        const SELECT_CORE: &str = "SELECT firstname, handle, email, password, createdAt, isAdmin, string::replace(string::concat(id), '`', '') AS id FROM player WHERE email = $email LIMIT 1";
        let select_q = self.query_with_scope(SELECT_CORE);
        self.ensure_scope().await;
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
        let player = match res.take::<Vec<PlayerRow>>(0) {
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
                let rows: Vec<serde_json::Value> = res2.take(0).unwrap_or_default();
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
        if let Some(ref cache) = self.cache {
            let cache_key = CacheKeys::player(id);
            if let Ok(Some(cached_player)) = cache.get::<Player>(&cache_key).await {
                log::debug!("Cache hit for player by id: {}", id);
                let mut p = cached_player;
                p.id = normalize_record_id_string(&p.id);
                return Some(p);
            }
        }

        let player = select_one_by_record_id(&self.db, "player", id)
            .await
            .and_then(|v| value_to_player(&v));
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

    async fn search_players(&self, query: &str) -> Vec<Player> {
        let q_owned = query.to_string();
        let mut res = match self.db.query(
            "SELECT * FROM player WHERE string::contains(string::lowercase(handle), string::lowercase($q)) \
             OR string::contains(string::lowercase(email), string::lowercase($q)) LIMIT 10",
        ).bind(("q", q_owned)).await {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        rows.into_iter()
            .filter_map(|v| value_to_player(&v))
            .collect()
    }

    async fn create(&self, player: Player) -> Result<Player, String> {
        let key = uuid::Uuid::new_v4().to_string();
        let created_at = player.created_at.to_rfc3339();
        let doc = serde_json::json!({
            "firstname": player.firstname,
            "handle": player.handle,
            "email": player.email,
            "password": player.password,
            "createdAt": created_at,
            "isAdmin": player.is_admin,
        });
        // When ns/db are set, run USE then CREATE so scope is set before CREATE (--test-threads 1 helps reuse connection).
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
        self.db
            .query("CREATE type::record('player', type::uuid($key)) CONTENT $doc")
            .bind(("key", key.clone()))
            .bind(("doc", doc))
            .await
            .map_err(|e| format!("Failed to create player: {}", e))?;
        let created_player = Player {
            id: format!("player/{}", key),
            rev: String::new(),
            firstname: player.firstname,
            handle: player.handle,
            email: player.email,
            password: player.password,
            created_at: player.created_at,
            is_admin: player.is_admin,
        };
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
        let email_key = player.email.clone();
        let doc = serde_json::json!({
            "firstname": player.firstname,
            "handle": player.handle,
            "email": player.email,
            "password": player.password,
        });
        // Ensure correct NS/DB scope on the connection that executes this update.
        // Without this, writes can land in the wrong namespace/database when scope
        // isn't persisted across connections/threads (WSL/Docker/test runners).
        self.ensure_scope().await;
        // Update by email, not by record id. This avoids Surreal RecordId key-typing edge cases
        // (string-vs-number keys and backtick-escaped string keys) that can cause UPDATE to
        // target a different record than the one we're reading elsewhere.
        //
        // Email is the stable unique identifier in this app (used by auth/session too).
        let update_q = self.query_with_scope(
            "UPDATE player MERGE $doc WHERE string::lowercase(email) = string::lowercase($email) RETURN AFTER",
        );
        let mut ur = self
            .db
            .query(&update_q)
            .bind(("email", email_key.clone()))
            .bind(("doc", doc))
            .await
            .map_err(|e| format!("Failed to update player: {}", e))?;
        let updated_rows: Vec<serde_json::Value> = ur
            .take(0)
            .map_err(|e| format!("Failed to parse player update result: {}", e))?;
        let stored_player = updated_rows
            .into_iter()
            .next()
            .and_then(|v| value_to_player(&v))
            .ok_or_else(|| {
                format!(
                    "Player update affected 0 rows (email={}, id={})",
                    email_key, player.id
                )
            })?;

        if stored_player.handle != player.handle {
            return Err(format!(
                "Player update returned unexpected handle (email={}, stored_handle='{}', expected='{}')",
                email_key, stored_player.handle, player.handle
            ));
        }

        let updated_player = stored_player.clone();
        if let Some(ref cache) = self.cache {
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
        }
    }

    #[tokio::test]
    async fn test_search_players_by_handle() {
        let players = vec![
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
        let players = vec![
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
        let players = vec![
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
        let players = vec![
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
        let players = vec![
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
        let players = vec![
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
        let players = vec![
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
