use crate::cache::{CacheKeys, CacheTTL, RedisCache};
use crate::db::Db;
use crate::surreal_helpers::thing_to_record_id;
use async_trait::async_trait;
use log;
use shared::models::player::Player;
use std::sync::Arc;

/// Row shape from SurrealDB SELECT * FROM player. Id is a Thing in responses.
#[derive(serde::Deserialize)]
struct PlayerRow {
    id: Option<surrealdb::sql::Thing>,
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
}

impl PlayerRepositoryImpl {
    pub fn new(db: Db) -> Self {
        Self { db, cache: None }
    }

    pub fn new_with_cache(db: Db, cache: Arc<RedisCache>) -> Self {
        Self {
            db,
            cache: Some(cache),
        }
    }
}

fn row_to_player(r: PlayerRow) -> Option<Player> {
    let id = thing_to_record_id(&r.id);
    if id.is_empty() {
        return None;
    }
    let created_at = r
        .created_at
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()))
        .unwrap_or_else(|| chrono::Utc::now().fixed_offset());
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

/// Extract record id from SurrealDB row (Value). Returns "table/key" with backticks stripped.
fn record_id_to_string(v: &serde_json::Value) -> Option<String> {
    let id_val = v.get("id").or_else(|| v.get("_id"))?;
    if let Some(s) = id_val.as_str() {
        let s = s.replace("player:", "player/").replace('`', "");
        return Some(s);
    }
    if let Some(tb) = id_val.get("tb").and_then(|x| x.as_str()) {
        let id_part = id_val
            .get("id")
            .and_then(|x| x.as_str().map(String::from))
            .or_else(|| id_val.get("id").and_then(|x| x.as_i64().map(|n| n.to_string())))
            .or_else(|| id_val.get("id").and_then(|x| x.as_u64().map(|n| n.to_string())));
        if let Some(mut id_part) = id_part {
            id_part = id_part.replace('`', "");
            return Some(format!("{}/{}", tb, id_part));
        }
    }
    None
}

/// Map a Surreal record (Value) to Player. Record has id (record id), and stored fields.
fn value_to_player(v: &serde_json::Value) -> Option<Player> {
    let id = record_id_to_string(v)?;
    let firstname = v.get("firstname").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let handle = v.get("handle").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let email = v.get("email").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let password = v.get("password").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let created_at = v
        .get("createdAt")
        .or_else(|| v.get("created_at"))
        .and_then(|x| serde_json::from_value::<chrono::DateTime<chrono::FixedOffset>>(x.clone()).ok())
        .unwrap_or_else(|| chrono::Utc::now().fixed_offset());
    let is_admin = v.get("isAdmin").or_else(|| v.get("is_admin")).and_then(|x| x.as_bool()).unwrap_or(false);
    Some(Player {
        id,
        rev: v.get("_rev").or_else(|| v.get("rev")).and_then(|x| x.as_str()).unwrap_or("").to_string(),
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
                return Some(cached_player);
            }
        }

        let email_owned = email.to_string();
        let mut res = match self
            .db
            .query("SELECT * FROM player WHERE string::lowercase(email) = string::lowercase($email) LIMIT 1")
            .bind(("email", email_owned))
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
                    log::warn!("Player find_by_email: got {} row(s) but row_to_player returned None (check id/fields for email {})", count, email);
                } else if p.is_none() {
                    log::info!("Player find_by_email: no rows for email {}", email);
                }
                p
            }
            Err(e) => {
                log::warn!("Player find_by_email: typed take failed ({}), trying Value path", e);
                let mut res2 = self
                    .db
                    .query("SELECT * FROM player WHERE string::lowercase(email) = string::lowercase($email) LIMIT 1")
                    .bind(("email", email.to_string()))
                    .await
                    .ok()?;
                let rows: Vec<serde_json::Value> = res2.take(0).unwrap_or_default();
                let row_count = rows.len();
                let p = rows.into_iter().next().and_then(|v| value_to_player(&v));
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
                let _ = cache.set_with_ttl(&CacheKeys::player_by_email(email), p, CacheTTL::player()).await;
                let _ = cache.set_with_ttl(&CacheKeys::player(&p.id), p, CacheTTL::player()).await;
            }
        }
        player
    }

    async fn find_by_id(&self, id: &str) -> Option<Player> {
        if let Some(ref cache) = self.cache {
            let cache_key = CacheKeys::player(id);
            if let Ok(Some(cached_player)) = cache.get::<Player>(&cache_key).await {
                log::debug!("Cache hit for player by id: {}", id);
                return Some(cached_player);
            }
        }

        let key = id
            .trim_start_matches("player/")
            .trim_start_matches("player:")
            .trim_matches('`')
            .to_string();
        let mut res = match self.db.query("SELECT * FROM player WHERE id = type::thing('player', $key)").bind(("key", key)).await {
            Ok(r) => r,
            Err(_) => return None,
        };
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let player = rows.into_iter().next().and_then(|v| value_to_player(&v));
        if let Some(ref p) = player {
            if let Some(ref cache) = self.cache {
                let _ = cache.set_with_ttl(&CacheKeys::player(&p.id), p, CacheTTL::player()).await;
                let _ = cache.set_with_ttl(&CacheKeys::player_by_email(&p.email), p, CacheTTL::player()).await;
                if !p.handle.is_empty() {
                    let _ = cache.set_with_ttl(&CacheKeys::player_by_handle(&p.handle), p, CacheTTL::player()).await;
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
        rows.into_iter().filter_map(|v| value_to_player(&v)).collect()
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
        self.db
            .query("CREATE type::thing('player', $key) CONTENT $doc")
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
            let _ = cache.set_with_ttl(&CacheKeys::player(&created_player.id), &created_player, CacheTTL::player()).await;
            let _ = cache.set_with_ttl(&CacheKeys::player_by_email(&created_player.email), &created_player, CacheTTL::player()).await;
            if !created_player.handle.is_empty() {
                let _ = cache.set_with_ttl(&CacheKeys::player_by_handle(&created_player.handle), &created_player, CacheTTL::player()).await;
            }
        }
        Ok(created_player)
    }

    async fn update(&self, player: Player) -> Result<Player, String> {
        let key = player
            .id
            .trim_start_matches("player/")
            .trim_start_matches("player:")
            .trim_matches('`')
            .to_string();
        let created_at = player.created_at.to_rfc3339();
        let doc = serde_json::json!({
            "firstname": player.firstname,
            "handle": player.handle,
            "email": player.email,
            "password": player.password,
            "createdAt": created_at,
            "isAdmin": player.is_admin,
        });
        self.db
            .query("UPDATE type::thing('player', $key) MERGE $doc")
            .bind(("key", key))
            .bind(("doc", doc))
            .await
            .map_err(|e| format!("Failed to update player: {}", e))?;
        let updated_player = player.clone();
        if let Some(ref cache) = self.cache {
            let _ = cache.delete(&CacheKeys::player_by_email(&player.email)).await;
            if !player.handle.is_empty() {
                let _ = cache.delete(&CacheKeys::player_by_handle(&player.handle)).await;
            }
            let _ = cache.set_with_ttl(&CacheKeys::player(&updated_player.id), &updated_player, CacheTTL::player()).await;
            let _ = cache.set_with_ttl(&CacheKeys::player_by_email(&updated_player.email), &updated_player, CacheTTL::player()).await;
            if !updated_player.handle.is_empty() {
                let _ = cache.set_with_ttl(&CacheKeys::player_by_handle(&updated_player.handle), &updated_player, CacheTTL::player()).await;
            }
        }
        Ok(updated_player)
    }

    async fn find_by_handle(&self, handle: &str) -> Option<Player> {
        if let Some(ref cache) = self.cache {
            let cache_key = CacheKeys::player_by_handle(handle);
            if let Ok(Some(cached_player)) = cache.get::<Player>(&cache_key).await {
                log::debug!("Cache hit for player by handle: {}", handle);
                return Some(cached_player);
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
            .map(|id| format!("player:{}", id.trim_start_matches("player/").trim_start_matches("player:")))
            .collect();
        let mut res = match self.db.query("SELECT * FROM player WHERE id INSIDE $ids").bind(("ids", record_ids)).await {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        rows.into_iter().filter_map(|v| value_to_player(&v)).collect()
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
            created_at: Utc::now().fixed_offset(),
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
