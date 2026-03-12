use crate::db::Db;
use serde_json::Value;
use shared::{Result, SharedError};
use surrealdb::types::SurrealValue;

#[derive(Clone)]
pub struct RatingsRepository {
    pub db: Db,
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
            .query("SELECT * FROM contest WHERE start >= $start AND start < $end")
            .bind(("start", start))
            .bind(("end", end))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to fetch contests: {}", e)))?;
        let rows: Vec<Value> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("Failed to take contests: {}", e)))?;
        // Normalize Surreal record id (contest:key) to API shape (_id = contest/key)
        let out: Vec<Value> = rows
            .into_iter()
            .map(|mut v| {
                if let Some(obj) = v.as_object_mut() {
                    if let Some(id) = obj.get("id") {
                        let s = id.to_string().trim_matches('"').replace("contest:", "contest/");
                        obj.insert("_id".into(), Value::String(s));
                    }
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
        let mut res = self
            .db
            .query(
                "SELECT `out` AS player_id, place FROM resulted_in WHERE `in` = type::record('contest', $key)",
            )
            .bind(("key", key))
            .await
            .map_err(|e| {
                SharedError::Database(format!("Failed to fetch contest results: {}", e))
            })?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct Row {
            player_id: Option<surrealdb::types::RecordId>,
            place: Option<i64>,
        }
        let rows: Vec<Row> = res.take(0).map_err(|e| {
            SharedError::Database(format!("Failed to take contest results: {}", e))
        })?;
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
        let mut res = self
            .db
            .query("SELECT `out` AS player_id FROM resulted_in WHERE `in` = type::record('contest', $key)")
            .bind(("key", key))
            .await
            .map_err(|e| {
                SharedError::Database(format!("Failed to fetch contest players: {}", e))
            })?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct Row {
            player_id: Option<surrealdb::types::RecordId>,
        }
        let rows: Vec<Row> = res.take(0).map_err(|e| {
            SharedError::Database(format!("Failed to take contest players: {}", e))
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

    pub async fn get_contest_game(&self, contest_id: &str) -> Result<Option<String>> {
        let key = crate::surreal_helpers::record_id_to_key(contest_id, "contest");
        let mut res = self
            .db
            .query("SELECT `out` AS game_id FROM played_with WHERE `in` = type::record('contest', $key) LIMIT 1")
            .bind(("key", key))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to fetch contest game: {}", e)))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct Row {
            game_id: Option<surrealdb::types::RecordId>,
        }
        let rows: Vec<Row> = res.take(0).map_err(|e| {
            SharedError::Database(format!("Failed to take contest game: {}", e))
        })?;
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
        let mut q = self.db.query(
            "SELECT * FROM rating_latest WHERE scope_type = $scope_type AND player_id = type::record('player', $player_id) \
             AND (($scope_id == NONE AND scope_id == NONE) OR scope_id = $scope_id) LIMIT 1",
        );
        q = q.bind(("scope_type", scope_type)).bind(("player_id", pid));
        if let Some(sid) = scope_id_owned {
            q = q.bind(("scope_id", sid));
        } else {
            q = q.bind(("scope_id", Option::<String>::None));
        }
        let mut res = q.await.map_err(|e| {
            SharedError::Database(format!("Failed to fetch latest rating: {}", e))
        })?;
        let rows: Vec<Value> = res.take(0).map_err(|e| {
            SharedError::Database(format!("Failed to take latest rating: {}", e))
        })?;
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
        let player_id = doc.get("player_id").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
        let pid = player_id
            .trim_start_matches("player/")
            .trim_start_matches("player:")
            .trim_matches('`')
            .to_string();
        let scope_type = doc.get("scope_type").and_then(|v| v.as_str()).map(String::from).unwrap_or_else(|| "global".to_string());
        let scope_id: Option<String> = doc.get("scope_id").and_then(|v| v.as_str()).map(String::from);
        // Upsert: delete existing then insert (rating_latest is keyed by player_id + scope)
        let mut del_q = self.db.query(
            "DELETE FROM rating_latest WHERE player_id = type::record('player', $pid) AND scope_type = $scope_type \
             AND (($scope_id == NONE AND scope_id == NONE) OR scope_id = $scope_id)",
        );
        del_q = del_q.bind(("pid", pid.clone())).bind(("scope_type", scope_type.clone()));
        if let Some(sid) = scope_id {
            del_q = del_q.bind(("scope_id", sid));
        } else {
            del_q = del_q.bind(("scope_id", Option::<String>::None));
        }
        del_q.await.map_err(|e| SharedError::Database(format!("Failed to delete previous latest rating: {}", e)))?;
        let mut doc_copy = doc.clone();
        if let Some(obj) = doc_copy.as_object_mut() {
            obj.insert("player_id".into(), serde_json::Value::String(format!("player:{}", pid)));
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
            .map_err(|e| SharedError::Database(format!("Failed to insert rating history: {}", e)))?;
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
        q = q.bind(("scope_type", scope_type)).bind(("min_games", min_games)).bind(("limit", limit));
        if let Some(sid) = scope_id_owned {
            q = q.bind(("scope_id", sid));
        } else {
            q = q.bind(("scope_id", Option::<String>::None));
        }
        let mut res = q.await.map_err(|e| {
            SharedError::Database(format!("Failed to fetch leaderboard: {}", e))
        })?;
        let rows: Vec<Value> = res.take(0).map_err(|e| {
            SharedError::Database(format!("Failed to take leaderboard: {}", e))
        })?;
        let mut out = Vec::new();
        for mut row in rows {
            if let Some(obj) = row.as_object_mut() {
                if let Some(pid) = obj.get("player_id") {
                    let pid_str = pid.to_string().trim_matches('"').replace("player:", "player/");
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
                obj.insert("last_active".into(), obj.get("last_period_end").cloned().unwrap_or(Value::Null));
            }
            out.push(row);
        }
        Ok(out)
    }

    async fn get_player_record(&self, key: &str) -> Result<Option<Value>> {
        let key = key.to_string();
        let mut res = self
            .db
            .query("SELECT * FROM player WHERE id = type::record('player', $key)")
            .bind(("key", key))
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
        q = q.bind(("scope_type", scope_type)).bind(("min_games", min_games)).bind(("limit", limit));
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
                let pid_str = pid.as_ref().and_then(|v| v.as_str()).map(|s| s.replace("player:", "player/"));
                let rating = obj.get("rating").cloned();
                let rd = obj.get("rd").cloned();
                let games_played = obj.get("games_played").and_then(|v| v.as_i64()).unwrap_or(0);
                let last_active = obj.get("last_period_end").cloned();
                let pk = pid_str
                    .as_deref()
                    .unwrap_or("")
                    .trim_start_matches("player/")
                    .trim_start_matches("player:")
                    .trim_matches('`');
                let (handle, firstname, email) = if let Ok(Some(pl)) = self.get_player_record(pk).await {
                    (
                        pl.get("handle").cloned(),
                        pl.get("firstname").cloned(),
                        pl.get("email").cloned(),
                    )
                } else {
                    (None, None, None)
                };
                let player_id_str = pid_str.or_else(|| pid.as_ref().and_then(|v| v.as_str()).map(|s| s.replace("player:", "player/")));
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
        let tables = info.get("tb").map(|v| v.clone()).unwrap_or(Value::Object(serde_json::Map::new()));
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
        let mut res = self
            .db
            .query("SELECT * FROM player WHERE id = type::record('player', $pid)")
            .bind(("pid", pid))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to debug player document: {}", e)))?;
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
        // Match record id or string form (slash or colon)
        let mut res = self
            .db
            .query(
                "SELECT * FROM rating_latest WHERE (player_id = type::record('player', $pid) \
                 OR string::concat(player_id) = $player_id_slash OR string::concat(player_id) = $player_id_colon)",
            )
            .bind(("pid", pid.clone()))
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
        let scope_type = scope_type.to_string();
        let scope_id_owned: Option<String> = scope_id.map(|s| s.to_string());
        let mut q = self.db.query(
            "SELECT * FROM rating_history WHERE (player_id = type::record('player', $pid) \
             OR string::concat(player_id) = $player_id_slash OR string::concat(player_id) = $player_id_colon) \
             AND scope_type = $scope_type AND (($scope_id == NONE AND scope_id == NONE) OR scope_id = $scope_id) \
             ORDER BY period_end DESC LIMIT $limit",
        );
        q = q
            .bind(("pid", pid))
            .bind(("player_id_slash", player_id_slash))
            .bind(("player_id_colon", player_id_colon))
            .bind(("scope_type", scope_type))
            .bind(("limit", limit));
        if let Some(sid) = scope_id_owned {
            q = q.bind(("scope_id", sid));
        } else {
            q = q.bind(("scope_id", Option::<String>::None));
        }
        let mut res = q.await.map_err(|e| {
            SharedError::Database(format!("Failed to fetch rating history: {}", e))
        })?;
        let rows: Vec<Value> = res.take(0).map_err(|e| {
            SharedError::Database(format!("Failed to take rating history: {}", e))
        })?;
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
            start: Option<String>,
        }
        let rows: Vec<Row> = res.take(0).map_err(|e| {
            SharedError::Database(format!("Failed to take earliest contest: {}", e))
        })?;
        let earliest_date = rows
            .into_iter()
            .next()
            .and_then(|r| r.start)
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
        q = q.bind(("scope_type", scope_type)).bind(("min_games", min_games)).bind(("limit", limit));
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
                    let pid_str = pid.to_string().trim_matches('"').replace("player:", "player/");
                    obj.insert("player_id".into(), serde_json::Value::String(pid_str));
                }
                obj.insert("player_handle".into(), Value::String("Player".to_string()));
                obj.insert("player_firstname".into(), Value::String("Unknown".to_string()));
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
        let rows: Vec<Row> = res.take(0).map_err(|e| SharedError::Database(e.to_string()))?;
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
