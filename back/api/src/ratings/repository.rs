use crate::db::Db;
use serde_json::Value;
use shared::{Result, SharedError};
use surrealdb::types::SurrealValue;

#[derive(Clone)]
pub struct RatingsRepository {
    pub db: Db,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, surrealdb::types::SurrealValue)]
pub struct RatingsRebuildRun {
    pub id: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub running: bool,
    pub current_period: Option<String>,
    pub processed_periods: u32,
    pub total_periods: u32,
    pub last_error: Option<String>,
}

impl RatingsRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn get_contests_in_period(
        &self,
        start: &str,
        end: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let start = start.to_string();
        let end = end.to_string();
        let mut res = self
            .db
            .query(
                "SELECT * FROM contest WHERE start >= type::datetime($start) AND start < type::datetime($end)",
            )
            .bind(("start", start))
            .bind(("end", end))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to fetch contests: {}", e)))?;
        let rows: Vec<Value> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("Failed to take contests: {}", e)))?;
        // Normalize Surreal record id (contest:key Thing) to canonical string id.
        let out: Vec<Value> = rows
            .into_iter()
            .map(|mut v| {
                // Surreal returns `id` as a Thing; ensure both `id` and `_id` are strings like "contest/<key>".
                let cid = crate::surreal_helpers::record_id_from_field(&v, "id");
                if let (Some(obj), Some(cid)) = (v.as_object_mut(), cid) {
                    obj.insert("id".into(), Value::String(cid.clone()));
                    obj.insert("_id".into(), Value::String(cid));
                }
                v
            })
            .collect();
        Ok(out)
    }

    pub async fn get_contest_results(
        &self,
        contest_id: &str,
    ) -> Result<Vec<(String, Option<i32>)>> {
        let key = crate::surreal_helpers::record_id_to_key(contest_id, "contest");
        if key.is_empty() {
            return Ok(Vec::new());
        }
        let record_id = surrealdb::types::RecordId::new("contest", key.as_str());
        let mut res = self
            .db
            .query("SELECT `out` AS player_id, place FROM resulted_in WHERE `in` = $record_id")
            .bind(("record_id", record_id))
            .await
            .map_err(|e| {
                SharedError::Database(format!("Failed to fetch contest results: {}", e))
            })?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct Row {
            player_id: Option<surrealdb::types::RecordId>,
            place: Option<i64>,
        }
        let rows: Vec<Row> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("Failed to take contest results: {}", e)))?;
        let mut out = Vec::new();
        for r in rows {
            let pid = r
                .player_id
                .as_ref()
                .map(crate::surreal_helpers::record_id_to_canonical)
                .unwrap_or_default()
                .replace("player:", "player/");
            let place = r.place.map(|n| n as i32);
            out.push((pid, place));
        }
        Ok(out)
    }

    pub async fn get_contest_players(&self, contest_id: &str) -> Result<Vec<String>> {
        let key = crate::surreal_helpers::record_id_to_key(contest_id, "contest");
        if key.is_empty() {
            return Ok(Vec::new());
        }
        let record_id = surrealdb::types::RecordId::new("contest", key.as_str());
        let mut res = self
            .db
            .query("SELECT `out` AS player_id FROM resulted_in WHERE `in` = $record_id")
            .bind(("record_id", record_id))
            .await
            .map_err(|e| {
                SharedError::Database(format!("Failed to fetch contest players: {}", e))
            })?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct Row {
            player_id: Option<surrealdb::types::RecordId>,
        }
        let rows: Vec<Row> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("Failed to take contest players: {}", e)))?;
        let out: Vec<String> = rows
            .into_iter()
            .filter_map(|r| {
                let id = crate::surreal_helpers::thing_to_record_id(&r.player_id);
                if id.is_empty() {
                    None
                } else {
                    Some(id)
                }
            })
            .collect();
        Ok(out)
    }

    pub async fn get_contest_game(&self, contest_id: &str) -> Result<Option<String>> {
        let key = crate::surreal_helpers::record_id_to_key(contest_id, "contest");
        if key.is_empty() {
            return Ok(None);
        }
        let record_id = surrealdb::types::RecordId::new("contest", key.as_str());
        let mut res = self
            .db
            .query("SELECT `out` AS game_id FROM played_with WHERE `in` = $record_id LIMIT 1")
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to fetch contest game: {}", e)))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct Row {
            game_id: Option<surrealdb::types::RecordId>,
        }
        let rows: Vec<Row> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("Failed to take contest game: {}", e)))?;
        Ok(rows.into_iter().next().and_then(|r| {
            let id = crate::surreal_helpers::thing_to_record_id(&r.game_id);
            if id.is_empty() {
                None
            } else {
                Some(id)
            }
        }))
    }

    pub async fn get_latest_rating(
        &self,
        scope_type: &str,
        scope_id: Option<&str>,
        player_id: &str,
    ) -> Result<Option<Value>> {
        let scope_type = scope_type.to_string();
        let pid = player_id
            .trim_start_matches("player/")
            .trim_start_matches("player:")
            .trim_matches('`')
            .to_string();
        let scope_id_owned: Option<String> = scope_id.map(|s| s.to_string());
        let player_record_id = surrealdb::types::RecordId::new("player", pid.as_str());
        let mut q = self.db.query(
            "SELECT * FROM rating_latest WHERE scope_type = $scope_type AND player_id = $record_id \
             AND (($scope_id == NONE AND scope_id == NONE) OR scope_id = $scope_id) LIMIT 1",
        );
        q = q
            .bind(("scope_type", scope_type))
            .bind(("record_id", player_record_id));
        if let Some(sid) = scope_id_owned {
            q = q.bind(("scope_id", sid));
        } else {
            q = q.bind(("scope_id", Option::<String>::None));
        }
        let mut res = q
            .await
            .map_err(|e| SharedError::Database(format!("Failed to fetch latest rating: {}", e)))?;
        let rows: Vec<Value> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("Failed to take latest rating: {}", e)))?;
        Ok(rows.into_iter().next())
    }

    /// Fetch all player_ids that have a latest rating for a given scope
    pub async fn get_all_latest_player_ids(
        &self,
        scope_type: &str,
        scope_id: Option<&str>,
    ) -> Result<Vec<String>> {
        let scope_type = scope_type.to_string();
        let scope_id_owned: Option<String> = scope_id.map(|s| s.to_string());
        let mut q = self.db.query(
            "SELECT player_id FROM rating_latest WHERE scope_type = $scope_type \
             AND (($scope_id == NONE AND scope_id == NONE) OR scope_id = $scope_id)",
        );
        q = q.bind(("scope_type", scope_type));
        if let Some(sid) = scope_id_owned {
            q = q.bind(("scope_id", sid));
        } else {
            q = q.bind(("scope_id", Option::<String>::None));
        }
        let mut res = q.await.map_err(|e| {
            SharedError::Database(format!("Failed to fetch latest player ids: {}", e))
        })?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct Row {
            player_id: Option<surrealdb::types::RecordId>,
        }
        let rows: Vec<Row> = res.take(0).map_err(|e| {
            SharedError::Database(format!("Failed to take latest player ids: {}", e))
        })?;
        let out: Vec<String> = rows
            .into_iter()
            .filter_map(|r| {
                let id = crate::surreal_helpers::thing_to_record_id(&r.player_id);
                if id.is_empty() {
                    None
                } else {
                    Some(id)
                }
            })
            .collect();
        Ok(out)
    }

    pub async fn upsert_latest_rating(&self, doc: Value) -> Result<()> {
        let player_id = doc
            .get("player_id")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_default();
        let pid = player_id
            .trim_start_matches("player/")
            .trim_start_matches("player:")
            .trim_matches('`')
            .to_string();
        let scope_type = doc
            .get("scope_type")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| "global".to_string());
        let scope_id: Option<String> = doc
            .get("scope_id")
            .and_then(|v| v.as_str())
            .map(String::from);
        // Upsert: delete existing then insert (rating_latest is keyed by player_id + scope)
        let mut del_q = self.db.query(
            "DELETE FROM rating_latest WHERE player_id = type::record('player', $pid) AND scope_type = $scope_type \
             AND (($scope_id == NONE AND scope_id == NONE) OR scope_id = $scope_id)",
        );
        del_q = del_q
            .bind(("pid", pid.clone()))
            .bind(("scope_type", scope_type.clone()));
        if let Some(sid) = scope_id {
            del_q = del_q.bind(("scope_id", sid));
        } else {
            del_q = del_q.bind(("scope_id", Option::<String>::None));
        }
        del_q.await.map_err(|e| {
            SharedError::Database(format!("Failed to delete previous latest rating: {}", e))
        })?;
        let mut doc_copy = doc.clone();
        if let Some(obj) = doc_copy.as_object_mut() {
            obj.insert(
                "player_id".into(),
                serde_json::Value::String(format!("player:{}", pid)),
            );
        }
        self.db
            .query("INSERT INTO rating_latest $doc")
            .bind(("doc", doc_copy))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to insert latest rating: {}", e)))?;
        Ok(())
    }

    pub async fn insert_rating_history(&self, doc: Value) -> Result<()> {
        self.db
            .query("INSERT INTO rating_history $doc")
            .bind(("doc", doc))
            .await
            .map_err(|e| {
                SharedError::Database(format!("Failed to insert rating history: {}", e))
            })?;
        Ok(())
    }

    /// Upsert latest rating row (global scope) with proper RecordId typing.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_latest_rating_global(
        &self,
        player_id: &str,
        rating: f64,
        rd: f64,
        volatility: f64,
        games_played: i32,
        last_period_end: &str,
        updated_at: &str,
    ) -> Result<()> {
        let pid = player_id
            .trim_start_matches("player/")
            .trim_start_matches("player:")
            .trim_matches('`')
            .to_string();
        if pid.is_empty() {
            return Err(SharedError::BadRequest("missing player_id".into()));
        }
        let record_id = surrealdb::types::RecordId::new("player", pid.as_str());

        // Delete existing row for this (player, scope).
        let mut del_q = self.db.query(
            "DELETE FROM rating_latest \
             WHERE player_id = $record_id AND scope_type = 'global' AND scope_id == NONE",
        );
        del_q = del_q.bind(("record_id", record_id.clone()));
        del_q.await.map_err(|e| {
            SharedError::Database(format!("Failed to delete previous latest rating: {}", e))
        })?;

        // Insert with correctly typed `player_id` (RecordId) and datetime fields (SCHEMAFULL expects datetime).
        let mut ins_res = self
            .db
            .query(
                "INSERT INTO rating_latest { \
                    player_id: $record_id, \
                    scope_type: 'global', \
                    scope_id: NONE, \
                    rating: $rating, \
                    rd: $rd, \
                    volatility: $volatility, \
                    games_played: $games_played, \
                    last_period_end: type::datetime($last_period_end), \
                    updated_at: type::datetime($updated_at) \
                }",
            )
            .bind(("record_id", record_id))
            .bind(("rating", rating))
            .bind(("rd", rd))
            .bind(("volatility", volatility))
            .bind(("games_played", games_played))
            .bind(("last_period_end", last_period_end.to_string()))
            .bind(("updated_at", updated_at.to_string()))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to insert latest rating: {}", e)))?;
        let inserted: Vec<Value> = match ins_res.take(0) {
            Ok(v) => v,
            Err(e) => {
                log::error!(
                    "upsert_latest_rating_global: INSERT failed (player_id={}, last_period_end={}, err={})",
                    player_id,
                    last_period_end,
                    e
                );
                return Err(SharedError::Database(format!(
                    "Failed to insert latest rating (player_id={}, last_period_end={}): {}",
                    player_id, last_period_end, e
                )));
            }
        };
        if inserted.is_empty() {
            log::warn!(
                "upsert_latest_rating_global: INSERT returned 0 rows without error (player_id={}, last_period_end={})",
                player_id,
                last_period_end
            );
        }
        Ok(())
    }

    /// Insert rating history row (global scope) with proper RecordId typing.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_rating_history_global(
        &self,
        player_id: &str,
        period_end: &str,
        rating: f64,
        rd: f64,
        volatility: f64,
        period_games: i32,
        wins: i32,
        losses: i32,
        draws: i32,
        created_at: &str,
    ) -> Result<()> {
        let pid = player_id
            .trim_start_matches("player/")
            .trim_start_matches("player:")
            .trim_matches('`')
            .to_string();
        if pid.is_empty() {
            return Err(SharedError::BadRequest("missing player_id".into()));
        }
        let record_id = surrealdb::types::RecordId::new("player", pid.as_str());

        // SCHEMAFULL expects datetime for period_end and created_at; use type::datetime() so server accepts.
        let mut ins_res = self
            .db
            .query(
                "INSERT INTO rating_history { \
                    player_id: $record_id, \
                    scope_type: 'global', \
                    scope_id: NONE, \
                    period_end: type::datetime($period_end), \
                    rating: $rating, \
                    rd: $rd, \
                    volatility: $volatility, \
                    period_games: $period_games, \
                    wins: $wins, \
                    losses: $losses, \
                    draws: $draws, \
                    created_at: type::datetime($created_at) \
                }",
            )
            .bind(("record_id", record_id))
            .bind(("period_end", period_end.to_string()))
            .bind(("rating", rating))
            .bind(("rd", rd))
            .bind(("volatility", volatility))
            .bind(("period_games", period_games))
            .bind(("wins", wins))
            .bind(("losses", losses))
            .bind(("draws", draws))
            .bind(("created_at", created_at.to_string()))
            .await
            .map_err(|e| {
                SharedError::Database(format!("Failed to insert rating history: {}", e))
            })?;
        let inserted: Vec<Value> = match ins_res.take(0) {
            Ok(v) => v,
            Err(e) => {
                log::error!(
                    "insert_rating_history_global: INSERT failed (player_id={}, period_end={}, err={})",
                    player_id,
                    period_end,
                    e
                );
                return Err(SharedError::Database(format!(
                    "Failed to insert rating history (player_id={}, period_end={}): {}",
                    player_id, period_end, e
                )));
            }
        };
        if inserted.is_empty() {
            log::warn!(
                "insert_rating_history_global: INSERT returned 0 rows without error (player_id={}, period_end={})",
                player_id,
                period_end
            );
        }
        Ok(())
    }

    pub async fn get_leaderboard(
        &self,
        scope_type: &str,
        scope_id: Option<&str>,
        min_games: i32,
        limit: i32,
    ) -> Result<Vec<Value>> {
        let scope_type = scope_type.to_string();
        let scope_id_owned: Option<String> = scope_id.map(|s| s.to_string());
        let mut q = self.db.query(
            "SELECT * FROM rating_latest WHERE scope_type = $scope_type \
             AND (($scope_id == NONE AND scope_id == NONE) OR scope_id = $scope_id) \
             AND games_played >= $min_games ORDER BY rating DESC LIMIT $limit",
        );
        q = q
            .bind(("scope_type", scope_type))
            .bind(("min_games", min_games))
            .bind(("limit", limit));
        if let Some(sid) = scope_id_owned {
            q = q.bind(("scope_id", sid));
        } else {
            q = q.bind(("scope_id", Option::<String>::None));
        }
        let mut res = q
            .await
            .map_err(|e| SharedError::Database(format!("Failed to fetch leaderboard: {}", e)))?;
        let rows: Vec<Value> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("Failed to take leaderboard: {}", e)))?;
        let mut out = Vec::new();
        for mut row in rows {
            if let Some(obj) = row.as_object_mut() {
                if let Some(pid) = obj.get("player_id") {
                    let pid_str = pid
                        .to_string()
                        .trim_matches('"')
                        .replace("player:", "player/");
                    obj.insert("player_id".into(), serde_json::Value::String(pid_str));
                }
                let pid_str = obj.get("player_id").and_then(Value::as_str);
                // Fetch player handle/firstname by record id
                if let Some(pid_s) = pid_str {
                    let pk = pid_s
                        .trim_start_matches("player/")
                        .trim_start_matches("player:")
                        .trim_matches('`');
                    if let Ok(Some(pl)) = self.get_player_record(pk).await {
                        if let Some(handle) = pl.get("handle") {
                            obj.insert("handle".into(), handle.clone());
                        }
                        if let Some(firstname) = pl.get("firstname") {
                            obj.insert("firstname".into(), firstname.clone());
                        }
                    }
                }
                obj.insert(
                    "last_active".into(),
                    obj.get("last_period_end").cloned().unwrap_or(Value::Null),
                );
            }
            out.push(row);
        }
        Ok(out)
    }

    async fn get_player_record(&self, key: &str) -> Result<Option<Value>> {
        let key = key.to_string();
        if key.is_empty() {
            return Ok(None);
        }
        let record_id = surrealdb::types::RecordId::new("player", key.as_str());
        let mut res = self
            .db
            .query("SELECT * FROM player WHERE id = $record_id")
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<Value> = res.take(0).unwrap_or_default();
        Ok(rows.into_iter().next())
    }

    /// Simple leaderboard query that just returns rating data without complex joins
    pub async fn get_simple_leaderboard(
        &self,
        scope_type: &str,
        scope_id: Option<&str>,
        min_games: i32,
        limit: i32,
    ) -> Result<Vec<Value>> {
        // Use rating_latest (has games_played) and join player for handle/firstname/email
        let scope_type = scope_type.to_string();
        let scope_id_owned: Option<String> = scope_id.map(|s| s.to_string());
        let mut q = self.db.query(
            "SELECT * FROM rating_latest WHERE scope_type = $scope_type \
             AND (($scope_id == NONE AND scope_id == NONE) OR scope_id = $scope_id) \
             AND games_played >= $min_games ORDER BY rating DESC LIMIT $limit",
        );
        q = q
            .bind(("scope_type", scope_type))
            .bind(("min_games", min_games))
            .bind(("limit", limit));
        if let Some(sid) = scope_id_owned {
            q = q.bind(("scope_id", sid));
        } else {
            q = q.bind(("scope_id", Option::<String>::None));
        }
        let mut res = q.await.map_err(|e| {
            SharedError::Database(format!("Failed to fetch simple leaderboard: {}", e))
        })?;
        let rows: Vec<Value> = res.take(0).map_err(|e| {
            SharedError::Database(format!("Failed to take simple leaderboard: {}", e))
        })?;
        let mut out = Vec::new();
        for row in rows {
            if let Some(obj) = row.as_object() {
                let pid = obj.get("player_id").cloned();
                let pid_str = pid
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .map(|s| s.replace("player:", "player/"));
                let rating = obj.get("rating").cloned();
                let rd = obj.get("rd").cloned();
                let games_played = obj
                    .get("games_played")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let last_active = obj.get("last_period_end").cloned();
                let pk = pid_str
                    .as_deref()
                    .unwrap_or("")
                    .trim_start_matches("player/")
                    .trim_start_matches("player:")
                    .trim_matches('`');
                let (handle, firstname, email) =
                    if let Ok(Some(pl)) = self.get_player_record(pk).await {
                        (
                            pl.get("handle").cloned(),
                            pl.get("firstname").cloned(),
                            pl.get("email").cloned(),
                        )
                    } else {
                        (None, None, None)
                    };
                let player_id_str = pid_str.or_else(|| {
                    pid.as_ref()
                        .and_then(|v| v.as_str())
                        .map(|s| s.replace("player:", "player/"))
                });
                out.push(serde_json::json!({
                    "player_id": player_id_str,
                    "rating": rating,
                    "rd": rd,
                    "games_played": games_played,
                    "last_active": last_active,
                    "player_handle": handle.or(firstname.clone()).or(email.clone()),
                    "player_firstname": firstname,
                    "player_email": email
                }));
            }
        }
        Ok(out)
    }

    /// Diagnostic function to check what's happening with player IDs
    pub async fn debug_player_ids(&self) -> Result<Vec<Value>> {
        let mut res = self
            .db
            .query(
                "SELECT r.player_id AS rating_player_id, r.player_id AS player_ref FROM rating_latest AS r LIMIT 5",
            )
            .await
            .map_err(|e| SharedError::Database(format!("Failed to debug player IDs: {}", e)))?;
        let rows: Vec<Value> = res.take(0).unwrap_or_default();
        Ok(rows)
    }

    /// Check what's in resulted_in edges vs player collection
    pub async fn debug_resulted_in_vs_players(&self) -> Result<Vec<Value>> {
        let mut res = self
            .db
            .query(
                "SELECT id AS edge_id, `in` AS edge_from, `out` AS edge_to FROM resulted_in LIMIT 5",
            )
            .await
            .map_err(|e| {
                SharedError::Database(format!("Failed to debug resulted_in vs players: {}", e))
            })?;
        let rows: Vec<Value> = res.take(0).unwrap_or_default();
        Ok(rows)
    }

    /// Check what tables exist in the database
    pub async fn debug_collections(&self) -> Result<Vec<Value>> {
        let mut res = self
            .db
            .query("INFO FOR DB")
            .await
            .map_err(|e| SharedError::Database(format!("Failed to debug tables: {}", e)))?;
        let rows: Vec<Value> = res.take(0).unwrap_or_default();
        let info: Value = rows.into_iter().next().unwrap_or(Value::Null);
        let tables = info
            .get("tb")
            .cloned()
            .unwrap_or(Value::Object(serde_json::Map::new()));
        let mut out = Vec::new();
        if let Some(obj) = tables.as_object() {
            for (name, _) in obj {
                out.push(serde_json::json!({ "collection_name": name, "document_count": "N/A" }));
            }
        }
        Ok(out)
    }

    /// Debug function to check what fields are in the player collection
    pub async fn debug_player_fields(&self) -> Result<Vec<Value>> {
        let mut res = self
            .db
            .query("SELECT * FROM player LIMIT 3")
            .await
            .map_err(|e| SharedError::Database(format!("Failed to debug player fields: {}", e)))?;
        let rows: Vec<Value> = res.take(0).unwrap_or_default();
        Ok(rows)
    }

    /// Simple test to see what's in a player document
    pub async fn debug_player_document(&self, player_id: &str) -> Result<Vec<Value>> {
        let pid = player_id
            .trim_start_matches("player/")
            .trim_start_matches("player:")
            .trim_matches('`')
            .to_string();
        if pid.is_empty() {
            return Ok(Vec::new());
        }
        let record_id = surrealdb::types::RecordId::new("player", pid.as_str());
        let mut res = self
            .db
            .query("SELECT * FROM player WHERE id = $record_id")
            .bind(("record_id", record_id))
            .await
            .map_err(|e| {
                SharedError::Database(format!("Failed to debug player document: {}", e))
            })?;
        let rows: Vec<Value> = res.take(0).unwrap_or_default();
        Ok(rows)
    }

    /// Fetch all latest rating rows for a player from rating_latest.
    /// Matches whether player_id is stored as record<player> or string (player/key or player:key).
    pub async fn get_player_latest_ratings(&self, player_id: &str) -> Result<Vec<Value>> {
        let pid = player_id
            .trim_start_matches("player/")
            .trim_start_matches("player:")
            .trim_matches('`')
            .to_string();
        let player_id_slash = format!("player/{}", pid);
        let player_id_colon = format!("player:{}", pid);
        let player_record_id = surrealdb::types::RecordId::new("player", pid.as_str());
        // Match record id or string form (slash or colon)
        let mut res = self
            .db
            .query(
                "SELECT * FROM rating_latest WHERE (player_id = $record_id \
                 OR string::concat(player_id) = $player_id_slash OR string::concat(player_id) = $player_id_colon)",
            )
            .bind(("record_id", player_record_id))
            .bind(("player_id_slash", player_id_slash))
            .bind(("player_id_colon", player_id_colon))
            .await
            .map_err(|e| {
                SharedError::Database(format!("Failed to fetch player latest ratings: {}", e))
            })?;
        let rows: Vec<Value> = res.take(0).map_err(|e| {
            SharedError::Database(format!("Failed to take player latest ratings: {}", e))
        })?;
        // Normalize player_id in each row to "player/key" for API responses
        let out: Vec<Value> = rows
            .into_iter()
            .map(|mut v| {
                if let Some(obj) = v.as_object_mut() {
                    if let Some(pid_val) = obj.get("player_id") {
                        let normalized = pid_val
                            .to_string()
                            .trim_matches('"')
                            .replace("player:", "player/")
                            .replace('`', "");
                        obj.insert("player_id".into(), Value::String(normalized));
                    }
                }
                v
            })
            .collect();
        Ok(out)
    }

    /// Fetch rating history for a player. Matches player_id as record or string (player/key or player:key).
    pub async fn get_rating_history(
        &self,
        player_id: &str,
        scope_type: &str,
        scope_id: Option<&str>,
        limit: i32,
    ) -> Result<Vec<Value>> {
        let pid = player_id
            .trim_start_matches("player/")
            .trim_start_matches("player:")
            .trim_matches('`')
            .to_string();
        let player_id_slash = format!("player/{}", pid);
        let player_id_colon = format!("player:{}", pid);
        let player_record_id = surrealdb::types::RecordId::new("player", pid.as_str());
        let scope_type = scope_type.to_string();
        let scope_id_owned: Option<String> = scope_id.map(|s| s.to_string());
        let mut q = self.db.query(
            "SELECT * FROM rating_history WHERE (player_id = $record_id \
             OR string::concat(player_id) = $player_id_slash OR string::concat(player_id) = $player_id_colon) \
             AND scope_type = $scope_type AND (($scope_id == NONE AND scope_id == NONE) OR scope_id = $scope_id) \
             ORDER BY period_end DESC LIMIT $limit",
        );
        q = q
            .bind(("record_id", player_record_id))
            .bind(("player_id_slash", player_id_slash))
            .bind(("player_id_colon", player_id_colon))
            .bind(("scope_type", scope_type))
            .bind(("limit", limit));
        if let Some(sid) = scope_id_owned {
            q = q.bind(("scope_id", sid));
        } else {
            q = q.bind(("scope_id", Option::<String>::None));
        }
        let mut res = q
            .await
            .map_err(|e| SharedError::Database(format!("Failed to fetch rating history: {}", e)))?;
        let rows: Vec<Value> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("Failed to take rating history: {}", e)))?;
        let out: Vec<Value> = rows
            .into_iter()
            .map(|mut v| {
                if let Some(obj) = v.as_object_mut() {
                    if let Some(pid_val) = obj.get("player_id") {
                        let normalized = pid_val
                            .to_string()
                            .trim_matches('"')
                            .replace("player:", "player/")
                            .replace('`', "");
                        obj.insert("player_id".into(), Value::String(normalized));
                    }
                }
                v
            })
            .collect();
        Ok(out)
    }

    pub async fn clear_all_ratings(&self) -> Result<()> {
        self.db
            .query("DELETE FROM rating_latest")
            .await
            .map_err(|e| SharedError::Database(format!("Failed to clear rating_latest: {}", e)))?;
        self.db
            .query("DELETE FROM rating_history")
            .await
            .map_err(|e| SharedError::Database(format!("Failed to clear rating_history: {}", e)))?;
        Ok(())
    }

    // --- Ratings rebuild run persistence (for admin UI/status across restarts) ---

    pub async fn create_rebuild_run(&self, started_at: &str) -> Result<String> {
        let mut res = self
            .db
            .query(
                "CREATE ratings_rebuild_run SET \
                    started_at = type::datetime($started_at), \
                    running = true, \
                    processed_periods = 0, \
                    total_periods = 0 \
                 RETURN string::concat(id) AS id",
            )
            .bind(("started_at", started_at.to_string()))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to create rebuild run: {}", e)))?;

        let rows: Vec<Value> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("Failed to take rebuild run id: {}", e)))?;

        let id = rows
            .first()
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if id.is_empty() {
            return Err(SharedError::Database(
                "Failed to create rebuild run: missing id".into(),
            ));
        }
        Ok(id)
    }

    pub async fn update_rebuild_run_progress(
        &self,
        run_id: &str,
        current_period: Option<&str>,
        processed_periods: u32,
        total_periods: u32,
        last_error: Option<&str>,
    ) -> Result<()> {
        let key = crate::surreal_helpers::record_id_to_key(run_id, "ratings_rebuild_run");
        if key.is_empty() {
            return Err(SharedError::BadRequest("invalid rebuild run id".into()));
        }
        let rid = surrealdb::types::RecordId::new("ratings_rebuild_run", key.as_str());

        self.db
            .query(
                "UPDATE $rid SET \
                    running = true, \
                    current_period = $current_period, \
                    processed_periods = $processed_periods, \
                    total_periods = $total_periods, \
                    last_error = $last_error",
            )
            .bind(("rid", rid))
            .bind(("current_period", current_period.map(|s| s.to_string())))
            .bind(("processed_periods", processed_periods))
            .bind(("total_periods", total_periods))
            .bind(("last_error", last_error.map(|s| s.to_string())))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to update rebuild run: {}", e)))?;

        Ok(())
    }

    pub async fn finish_rebuild_run(
        &self,
        run_id: &str,
        finished_at: &str,
        last_error: Option<&str>,
    ) -> Result<()> {
        let key = crate::surreal_helpers::record_id_to_key(run_id, "ratings_rebuild_run");
        if key.is_empty() {
            return Err(SharedError::BadRequest("invalid rebuild run id".into()));
        }
        let rid = surrealdb::types::RecordId::new("ratings_rebuild_run", key.as_str());

        self.db
            .query(
                "UPDATE $rid SET \
                    running = false, \
                    finished_at = type::datetime($finished_at), \
                    current_period = NONE, \
                    last_error = $last_error",
            )
            .bind(("rid", rid))
            .bind(("finished_at", finished_at.to_string()))
            .bind(("last_error", last_error.map(|s| s.to_string())))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to finish rebuild run: {}", e)))?;

        Ok(())
    }

    pub async fn get_last_completed_rebuild_run(&self) -> Result<Option<RatingsRebuildRun>> {
        let mut res = self
            .db
            .query(
                "SELECT \
                    string::concat(id) AS id, \
                    started_at, \
                    finished_at, \
                    running, \
                    current_period, \
                    processed_periods, \
                    total_periods, \
                    last_error \
                 FROM ratings_rebuild_run \
                 WHERE finished_at != NONE \
                 ORDER BY finished_at DESC \
                 LIMIT 1",
            )
            .await
            .map_err(|e| {
                SharedError::Database(format!("Failed to fetch last rebuild run: {}", e))
            })?;

        let rows: Vec<RatingsRebuildRun> = res.take(0).map_err(|e| {
            SharedError::Database(format!("Failed to take last rebuild run: {}", e))
        })?;
        Ok(rows.into_iter().next())
    }

    pub async fn get_earliest_contest_date(&self) -> Result<String> {
        let mut res = self
            .db
            .query("SELECT start FROM contest ORDER BY start ASC LIMIT 1")
            .await
            .map_err(|e| {
                SharedError::Database(format!("Failed to fetch earliest contest date: {}", e))
            })?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct Row {
            start: Option<serde_json::Value>,
        }
        let rows: Vec<Row> = res.take(0).map_err(|e| {
            SharedError::Database(format!("Failed to take earliest contest: {}", e))
        })?;
        fn as_rfc3339(v: &serde_json::Value) -> Option<String> {
            if let Some(s) = v.as_str() {
                return chrono::DateTime::parse_from_rfc3339(s)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc).to_rfc3339());
            }
            chrono::DateTime::parse_from_rfc3339(&v.to_string())
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc).to_rfc3339())
        }
        let earliest_date = rows
            .into_iter()
            .next()
            .and_then(|r| r.start.and_then(|v| as_rfc3339(&v)))
            .unwrap_or_else(|| "2000-01-01T00:00:00Z".to_string());
        Ok(earliest_date)
    }

    /// Leaderboard with player info extracted from contest data
    pub async fn get_leaderboard_with_contest_data(
        &self,
        scope_type: &str,
        scope_id: Option<&str>,
        min_games: i32,
        limit: i32,
    ) -> Result<Vec<Value>> {
        let scope_type = scope_type.to_string();
        let scope_id_owned: Option<String> = scope_id.map(|s| s.to_string());
        let mut q = self.db.query(
            r#"
            SELECT
                r.player_id AS player_id,
                r.rating AS rating,
                r.rd AS rd,
                r.games_played AS games_played,
                r.last_period_end AS last_active,
                (SELECT { contest_name: c.name, contest_date: c.start, player_place: res.place } FROM resulted_in AS res, contest AS c WHERE res.`in` = c.id AND res.`out` = r.player_id LIMIT 1)[0] AS contest_info
            FROM rating_latest AS r
            WHERE r.scope_type = $scope_type
              AND (($scope_id == NONE AND r.scope_id == NONE) OR r.scope_id = $scope_id)
              AND r.games_played >= $min_games
            ORDER BY r.rating DESC
            LIMIT $limit
            "#,
        );
        q = q
            .bind(("scope_type", scope_type))
            .bind(("min_games", min_games))
            .bind(("limit", limit));
        if let Some(sid) = scope_id_owned {
            q = q.bind(("scope_id", sid));
        } else {
            q = q.bind(("scope_id", Option::<String>::None));
        }
        let mut res = q.await.map_err(|e| {
            SharedError::Database(format!(
                "Failed to fetch leaderboard with contest data: {}",
                e
            ))
        })?;
        let rows: Vec<Value> = res.take(0).map_err(|e| {
            SharedError::Database(format!(
                "Failed to take leaderboard with contest data: {}",
                e
            ))
        })?;
        let mut out = Vec::new();
        for mut row in rows {
            if let Some(obj) = row.as_object_mut() {
                if let Some(pid) = obj.get("player_id") {
                    let pid_str = pid
                        .to_string()
                        .trim_matches('"')
                        .replace("player:", "player/");
                    obj.insert("player_id".into(), serde_json::Value::String(pid_str));
                }
                obj.insert("player_handle".into(), Value::String("Player".to_string()));
                obj.insert(
                    "player_firstname".into(),
                    Value::String("Unknown".to_string()),
                );
                obj.insert("player_email".into(), Value::String("Unknown".to_string()));
            }
            out.push(row);
        }
        Ok(out)
    }

    /// Get player ID by email (used by ratings usecase for debug / current user)
    pub async fn get_player_id_by_email(&self, email: &str) -> Result<Option<String>> {
        let email = email.to_string();
        let mut res = self
            .db
            .query("SELECT id FROM player WHERE string::lowercase(email) = string::lowercase($email) LIMIT 1")
            .bind(("email", email))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to query player by email: {}", e)))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct Row {
            id: Option<surrealdb::types::RecordId>,
        }
        let rows: Vec<Row> = res
            .take(0)
            .map_err(|e| SharedError::Database(e.to_string()))?;
        Ok(rows.into_iter().next().and_then(|r| {
            let id = crate::surreal_helpers::thing_to_record_id(&r.id);
            if id.is_empty() {
                None
            } else {
                Some(id)
            }
        }))
    }
}
