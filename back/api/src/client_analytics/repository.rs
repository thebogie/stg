use crate::db::Db;
use crate::surreal_helpers::scalar_i64;
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use log;
use shared::dto::client_sync::*;
use shared::error::SharedError;
use shared::models::{contest::Contest, game::Game, player::Player, venue::Venue};

/// Repository trait for client analytics data access
#[async_trait]
pub trait ClientAnalyticsRepository: Send + Sync {
    /// Gets all contests for a player
    async fn get_all_contests_for_player(
        &self,
        player_id: &str,
    ) -> Result<Vec<Contest>, SharedError>;

    /// Gets contests since a specific timestamp
    async fn get_contests_since(
        &self,
        player_id: &str,
        since: DateTime<FixedOffset>,
    ) -> Result<Vec<Contest>, SharedError>;

    /// Gets filtered contests based on query parameters
    async fn get_filtered_contests(
        &self,
        player_id: &str,
        query: &ClientAnalyticsQuery,
    ) -> Result<Vec<Contest>, SharedError>;

    /// Gets the game for a specific contest
    async fn get_game_for_contest(&self, contest_id: &str) -> Result<Game, SharedError>;

    /// Gets the venue for a specific contest
    async fn get_venue_for_contest(&self, contest_id: &str) -> Result<Venue, SharedError>;

    /// Gets all participants for a contest
    async fn get_contest_participants(
        &self,
        contest_id: &str,
    ) -> Result<Vec<ContestParticipant>, SharedError>;

    /// Gets all games a player has played
    async fn get_games_for_player(&self, player_id: &str) -> Result<Vec<Game>, SharedError>;

    /// Gets all venues a player has played at
    async fn get_venues_for_player(&self, player_id: &str) -> Result<Vec<Venue>, SharedError>;

    /// Gets all opponents a player has faced
    async fn get_opponents_for_player(&self, player_id: &str) -> Result<Vec<Player>, SharedError>;

    /// Gets total contest count for a player
    async fn get_total_contests_for_player(&self, player_id: &str) -> Result<usize, SharedError>;

    /// Gets the last contest for a player
    async fn get_last_contest_for_player(
        &self,
        player_id: &str,
    ) -> Result<Option<Contest>, SharedError>;

    /// Gets gaming communities and regular opponents
    async fn get_gaming_communities(
        &self,
        player_id: &str,
        min_contests: i32,
    ) -> Result<Vec<serde_json::Value>, SharedError>;

    /// Gets player networking insights (who plays with whom)
    async fn get_player_networking(
        &self,
        player_id: &str,
    ) -> Result<serde_json::Value, SharedError>;
}

/// Contest participant with result data
#[derive(Debug, Clone)]
pub struct ContestParticipant {
    pub player_id: String,
    pub handle: String,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub place: i32,
    pub result: String,
    pub points: Option<i32>,
}

/// Implementation of client analytics repository
pub struct ClientAnalyticsRepositoryImpl {
    db: Db,
}

fn to_rid(s: &str) -> String {
    if s.contains(':') {
        s.to_string()
    } else {
        s.replace('/', ":")
    }
}

/// Extract record id from SurrealDB row (id may be string "table:key" or Thing { tb, id }) as "table/key".
fn record_id_from_value(v: &serde_json::Value) -> Option<String> {
    let id_val = v.get("id").or_else(|| v.get("_id"))?;
    if let Some(s) = id_val.as_str() {
        return Some(s.replace(":", "/"));
    }
    if let Some(tb) = id_val.get("tb").and_then(|x| x.as_str()) {
        if let Some(id_part) = id_val.get("id").and_then(json_id_part_to_string) {
            return Some(format!("{}/{}", tb, id_part));
        }
    }
    None
}

/// SurrealDB can return record id as string or number in Thing { tb, id }.
fn json_id_part_to_string(v: &serde_json::Value) -> Option<String> {
    v.as_str()
        .map(String::from)
        .or_else(|| v.as_i64().map(|n| n.to_string()))
        .or_else(|| v.as_u64().map(|n| n.to_string()))
}

/// Extract edge "out" as "table:key" from a SurrealDB row (out may be string or Thing object).
fn edge_out_to_rid(v: &serde_json::Value) -> Option<String> {
    let out_val = v.get("out").or_else(|| v.get("contest_rid"))?;
    if let Some(s) = out_val.as_str() {
        return Some(s.to_string());
    }
    if let Some(tb) = out_val.get("tb").and_then(|x| x.as_str()) {
        if let Some(id_part) = out_val.get("id").and_then(json_id_part_to_string) {
            return Some(format!("{}:{}", tb, id_part));
        }
    }
    None
}

/// Extract edge "in" as "table:key" from a SurrealDB row (in may be string or Thing object).
fn edge_in_to_rid(v: &serde_json::Value) -> Option<String> {
    let in_val = v.get("in")?;
    if let Some(s) = in_val.as_str() {
        return Some(s.to_string());
    }
    if let Some(tb) = in_val.get("tb").and_then(|x| x.as_str()) {
        if let Some(id_part) = in_val.get("id").and_then(json_id_part_to_string) {
            return Some(format!("{}:{}", tb, id_part));
        }
    }
    None
}

fn split_rid(rid: &str) -> (&str, &str) {
    if let Some((tb, id)) = rid.split_once(':') {
        (tb, id)
    } else {
        (rid, "")
    }
}

/// Owned (String, String) for use with SurrealDB bind (requires 'static).
fn split_rid_owned(rid: &str) -> (String, String) {
    let (tb, id) = split_rid(rid);
    (tb.to_string(), id.to_string())
}

impl ClientAnalyticsRepositoryImpl {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Fetch contest ids for a player (two-query workaround for IN (SELECT VALUE out) returning [] in some SurrealDB setups).
    async fn contest_ids_for_player(&self, player_key: &str) -> Result<Vec<String>, SharedError> {
        let key = player_key.to_string();
        let mut res = self
            .db
            .query("SELECT out FROM resulted_in WHERE in = type::thing('player', $player_key)")
            .bind(("player_key", key))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        Ok(rows.iter().filter_map(edge_out_to_rid).collect())
    }
}

#[async_trait]
impl ClientAnalyticsRepository for ClientAnalyticsRepositoryImpl {
    async fn get_all_contests_for_player(
        &self,
        player_id: &str,
    ) -> Result<Vec<Contest>, SharedError> {
        let rid = to_rid(player_id);
        let (_, player_key) = split_rid_owned(&rid);
        let contest_ids = self.contest_ids_for_player(&player_key).await?;
        if contest_ids.is_empty() {
            log::info!("Retrieved 0 contests for player: {}", player_id);
            return Ok(Vec::new());
        }
        let mut res = self
            .db
            .query("SELECT * FROM contest WHERE id INSIDE $contest_ids ORDER BY start DESC")
            .bind(("contest_ids", contest_ids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let contests: Vec<Contest> = res.take(0).unwrap_or_default();
        log::info!("Retrieved {} contests for player: {}", contests.len(), player_id);
        Ok(contests)
    }

    async fn get_contests_since(
        &self,
        player_id: &str,
        since: DateTime<FixedOffset>,
    ) -> Result<Vec<Contest>, SharedError> {
        let rid = to_rid(player_id);
        let (_, player_key) = split_rid_owned(&rid);
        let contest_ids = self.contest_ids_for_player(&player_key).await?;
        if contest_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut res = self
            .db
            .query("SELECT * FROM contest WHERE id INSIDE $contest_ids AND start >= $since ORDER BY start DESC")
            .bind(("contest_ids", contest_ids))
            .bind(("since", since.to_rfc3339()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let contests: Vec<Contest> = res.take(0).unwrap_or_default();
        log::info!("Retrieved {} contests since {} for player: {}", contests.len(), since, player_id);
        Ok(contests)
    }

    async fn get_filtered_contests(
        &self,
        player_id: &str,
        query: &ClientAnalyticsQuery,
    ) -> Result<Vec<Contest>, SharedError> {
        let rid = to_rid(player_id);
        let (_, player_key) = split_rid_owned(&rid);
        let contest_ids = self.contest_ids_for_player(&player_key).await?;
        if contest_ids.is_empty() {
            return Ok(Vec::new());
        }
        let (sql, date_range) = if let Some(dr) = &query.date_range {
            (
                "SELECT * FROM contest WHERE id INSIDE $contest_ids AND start >= $start_date AND start <= $end_date ORDER BY start DESC",
                Some((dr.start.to_rfc3339(), dr.end.to_rfc3339())),
            )
        } else {
            (
                "SELECT * FROM contest WHERE id INSIDE $contest_ids ORDER BY start DESC",
                None,
            )
        };
        let mut q = self.db.query(sql).bind(("contest_ids", contest_ids));
        if let Some((start, end)) = date_range {
            q = q.bind(("start_date", start)).bind(("end_date", end));
        }
        let mut res = q.await.map_err(|e| SharedError::Database(e.to_string()))?;
        let contests: Vec<Contest> = res.take(0).unwrap_or_default();
        log::info!("Retrieved {} filtered contests for player: {}", contests.len(), player_id);
        Ok(contests)
    }

    async fn get_game_for_contest(&self, contest_id: &str) -> Result<Game, SharedError> {
        let rid = to_rid(contest_id);
        let mut edge_res = self
            .db
            .query("SELECT in FROM played_with WHERE out = $rid LIMIT 1")
            .bind(("rid", rid.clone()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let edge_rows: Vec<serde_json::Value> = edge_res.take(0).unwrap_or_default();
        let game_ids: Vec<String> = edge_rows.iter().filter_map(edge_in_to_rid).collect();
        if game_ids.is_empty() {
            return Err(SharedError::NotFound(format!("No game found for contest: {}", contest_id)));
        }
        let mut res = self
            .db
            .query("SELECT * FROM game WHERE id INSIDE $game_ids LIMIT 1")
            .bind(("game_ids", game_ids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let games: Vec<Game> = res.take(0).unwrap_or_default();
        games.into_iter().next().ok_or_else(|| SharedError::NotFound(format!("No game found for contest: {}", contest_id)))
    }

    async fn get_venue_for_contest(&self, contest_id: &str) -> Result<Venue, SharedError> {
        let rid = to_rid(contest_id);
        let mut edge_res = self
            .db
            .query("SELECT in FROM played_at WHERE out = $rid LIMIT 1")
            .bind(("rid", rid.clone()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let edge_rows: Vec<serde_json::Value> = edge_res.take(0).unwrap_or_default();
        let venue_ids: Vec<String> = edge_rows.iter().filter_map(edge_in_to_rid).collect();
        if venue_ids.is_empty() {
            return Err(SharedError::NotFound(format!("No venue found for contest: {}", contest_id)));
        }
        let mut res = self
            .db
            .query("SELECT * FROM venue WHERE id INSIDE $venue_ids LIMIT 1")
            .bind(("venue_ids", venue_ids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let venues: Vec<Venue> = res.take(0).unwrap_or_default();
        venues.into_iter().next().ok_or_else(|| SharedError::NotFound(format!("No venue found for contest: {}", contest_id)))
    }

    async fn get_contest_participants(
        &self,
        contest_id: &str,
    ) -> Result<Vec<ContestParticipant>, SharedError> {
        let rid = to_rid(contest_id);
        let sql = "SELECT `in` AS player_id, place, result, points FROM resulted_in WHERE `out` = $rid";
        let mut res = self.db.query(sql).bind(("rid", rid)).await.map_err(|e| SharedError::Database(e.to_string()))?;
        let edges: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let player_ids: Vec<String> = edges.iter().filter_map(|e| e.get("player_id").and_then(|v| v.as_str()).map(|s| s.replace("player:", "player/"))).collect();
        if player_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids_surreal: Vec<String> = player_ids.iter().map(|s| to_rid(s)).collect();
        let mut res2 = self
            .db
            .query("SELECT string::concat(id) AS id, handle, firstname, lastname FROM player WHERE id INSIDE $ids")
            .bind(("ids", ids_surreal))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let players: Vec<serde_json::Value> = res2.take(0).unwrap_or_default();
        let player_map: std::collections::HashMap<String, (String, Option<String>, Option<String>)> = players.into_iter().filter_map(|p| {
            let id = record_id_from_value(&p)?;
            let handle = p.get("handle").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let firstname = p.get("firstname").and_then(|v| v.as_str()).map(String::from);
            let lastname = p.get("lastname").and_then(|v| v.as_str()).map(String::from);
            Some((id, (handle, firstname, lastname)))
        }).collect();
        let mut result = Vec::new();
        for e in edges {
            let player_id = e.get("player_id").and_then(|v| v.as_str()).map(|s| s.replace("player:", "player/")).unwrap_or_default();
            let (handle, firstname, lastname) = player_map.get(&player_id).cloned().unwrap_or((String::new(), None, None));
            result.push(ContestParticipant {
                player_id,
                handle,
                firstname,
                lastname,
                place: e.get("place").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                result: e.get("result").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                points: e.get("points").and_then(|v| v.as_i64()).map(|p| p as i32),
            });
        }
        Ok(result)
    }

    async fn get_games_for_player(&self, player_id: &str) -> Result<Vec<Game>, SharedError> {
        let (_, player_key) = split_rid_owned(&to_rid(player_id));
        let contest_ids = self.contest_ids_for_player(&player_key).await?;
        if contest_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut edge_res = self
            .db
            .query("SELECT in FROM played_with WHERE out INSIDE $contest_ids")
            .bind(("contest_ids", contest_ids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let edge_rows: Vec<serde_json::Value> = edge_res.take(0).unwrap_or_default();
        let game_ids: Vec<String> = edge_rows.iter().filter_map(edge_in_to_rid).collect::<std::collections::HashSet<_>>().into_iter().collect();
        if game_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut res = self
            .db
            .query("SELECT * FROM game WHERE id INSIDE $game_ids")
            .bind(("game_ids", game_ids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let games: Vec<Game> = res.take(0).unwrap_or_default();
        log::info!("Retrieved {} unique games for player: {}", games.len(), player_id);
        Ok(games)
    }

    async fn get_venues_for_player(&self, player_id: &str) -> Result<Vec<Venue>, SharedError> {
        let (_, player_key) = split_rid_owned(&to_rid(player_id));
        let contest_ids = self.contest_ids_for_player(&player_key).await?;
        if contest_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut edge_res = self
            .db
            .query("SELECT in FROM played_at WHERE out INSIDE $contest_ids")
            .bind(("contest_ids", contest_ids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let edge_rows: Vec<serde_json::Value> = edge_res.take(0).unwrap_or_default();
        let venue_ids: Vec<String> = edge_rows.iter().filter_map(edge_in_to_rid).collect::<std::collections::HashSet<_>>().into_iter().collect();
        if venue_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut res = self
            .db
            .query("SELECT * FROM venue WHERE id INSIDE $venue_ids")
            .bind(("venue_ids", venue_ids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let venues: Vec<Venue> = res.take(0).unwrap_or_default();
        log::info!("Retrieved {} unique venues for player: {}", venues.len(), player_id);
        Ok(venues)
    }

    async fn get_opponents_for_player(&self, player_id: &str) -> Result<Vec<Player>, SharedError> {
        let rid = to_rid(player_id);
        let (_, player_key) = split_rid_owned(&rid);
        let contest_ids = self.contest_ids_for_player(&player_key).await?;
        if contest_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut edge_res = self
            .db
            .query("SELECT in FROM resulted_in WHERE out INSIDE $contest_ids AND in != type::thing('player', $player_key)")
            .bind(("contest_ids", contest_ids))
            .bind(("player_key", player_key))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let edge_rows: Vec<serde_json::Value> = edge_res.take(0).unwrap_or_default();
        let player_ids: Vec<String> = edge_rows.iter().filter_map(edge_in_to_rid).collect::<std::collections::HashSet<_>>().into_iter().collect();
        if player_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut res = self
            .db
            .query("SELECT * FROM player WHERE id INSIDE $player_ids")
            .bind(("player_ids", player_ids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let opponents: Vec<Player> = res.take(0).unwrap_or_default();
        log::info!("Retrieved {} unique opponents for player: {}", opponents.len(), player_id);
        Ok(opponents)
    }

    async fn get_total_contests_for_player(&self, player_id: &str) -> Result<usize, SharedError> {
        let rid = to_rid(player_id);
        let (_, player_key) = split_rid_owned(&rid);
        let sql = "SELECT count() AS n FROM resulted_in WHERE in = type::thing('player', $player_key) GROUP ALL";
        let mut res = self
            .db
            .query(sql)
            .bind(("player_key", player_key))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res
            .take(0)
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let count = rows
            .into_iter()
            .next()
            .map(|r| scalar_i64(r.get("n").unwrap_or(&serde_json::Value::Null)))
            .unwrap_or(0) as usize;
        Ok(count)
    }

    async fn get_last_contest_for_player(
        &self,
        player_id: &str,
    ) -> Result<Option<Contest>, SharedError> {
        let rid = to_rid(player_id);
        let (_, player_key) = split_rid_owned(&rid);
        let contest_ids = self.contest_ids_for_player(&player_key).await?;
        if contest_ids.is_empty() {
            return Ok(None);
        }
        let sql = "SELECT * FROM contest WHERE id INSIDE $contest_ids ORDER BY start DESC LIMIT 1";
        let mut res = self.db.query(sql).bind(("contest_ids", contest_ids)).await.map_err(|e| SharedError::Database(e.to_string()))?;
        let contests: Vec<Contest> = res.take(0).unwrap_or_default();
        Ok(contests.into_iter().next())
    }

    async fn get_gaming_communities(
        &self,
        player_id: &str,
        min_contests: i32,
    ) -> Result<Vec<serde_json::Value>, SharedError> {
        log::info!("🔍 Getting gaming communities for player: {} (min_contests={})", player_id, min_contests);
        let _ = (self, player_id, min_contests);
        Ok(Vec::new())
    }

    async fn get_player_networking(
        &self,
        player_id: &str,
    ) -> Result<serde_json::Value, SharedError> {
        log::info!("🔍 Getting player networking insights for player: {}", player_id);
        // Stub: return minimal structure; full SurrealQL can be added later
        Ok(serde_json::json!({
            "player_id": player_id,
            "player_handle": "",
            "opponent_analysis": [],
            "network_metrics": { "total_opponents": 0 }
        }))
    }
}
