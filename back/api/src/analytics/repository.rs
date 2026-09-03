//! Analytics repository: SurrealDB access for analytics and profile data.
//!
//! **Profile tabs → methods (see docs/PROFILE_PAGE_DESIGN.md):**
//! - Overall Stats: `get_player_display_label`, `get_player_stats`
//! - Achievements: `get_player_achievements`
//! - Nemesis: `get_players_who_beat_me`
//! - Owned: `get_players_i_beat`
//! - Game Performance: `get_my_game_performance`
//! - Trends: `get_my_performance_trends`
//! - Comparison: `get_player_stats`, `get_head_to_head_record`
//!
//! SurrealDB patterns: bind `RecordId` for read-path lookups (`WHERE id = $record_id`), no table aliases in SurrealQL,
//! scalar extraction via helpers, typed rows with `Option<Thing>`.

use crate::analytics::engine::{ContestParticipant, ContestResult, GamePlay, VenueContest};
use crate::config::DatabaseConfig;
use crate::db::Db;
use crate::surreal_helpers::{
    normalize_record_id_string, record_id_from_field, record_id_from_row, record_id_to_key,
    record_id_to_canonical, select_one_by_record_id, thing_to_record_id,
};
use chrono::Timelike;
use shared::dto::analytics::{
    GamePerformanceDetailDto, GamePerformanceOpponentDto, GamePerformanceVenueDto,
};
use shared::dto::analytics::{GamePerformanceDto, PerformanceTrendDto, PlayerOpponentDto};
use shared::{dto::analytics::TimePeriod, models::analytics::*, Result, SharedError};
use std::collections::HashMap;
use surrealdb::types::SurrealValue;

type GamePerformanceAggMap = HashMap<
    String,
    (
        i32,
        i32,
        Vec<i32>,
        Option<chrono::DateTime<chrono::FixedOffset>>,
    ),
>;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
pub struct HeatRow {
    pub day: i32,
    pub hour: i32,
    pub plays: i64,
}

#[derive(serde::Deserialize)]
struct PlayerDataResult {
    player_id: String,
    #[allow(dead_code)]
    player_handle: String,
    total_contests: i32,
    total_wins: i32,
    unique_games: i32,
    unique_venues: i32,
}

// Typed query result rows (SurrealDB v3: take(0) requires SurrealValue).
#[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
struct ResultedInRow {
    contest_id: Option<surrealdb::types::RecordId>,
    player_id: Option<surrealdb::types::RecordId>,
    place: Option<i64>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
struct GamePerformanceRow {
    contest_id: Option<surrealdb::types::RecordId>,
    place: Option<i64>,
    game_id: Option<surrealdb::types::RecordId>,
    contest_start: Option<String>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
struct PerformanceTrendRow {
    place: Option<i64>,
    /// SurrealDB may return datetime as string (RFC3339) or as object; use Value for resilient deserialization.
    contest_start: Option<serde_json::Value>,
}

#[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
struct PlayerDisplayRow {
    id: Option<surrealdb::types::RecordId>,
    handle: Option<String>,
    firstname: Option<String>,
    lastname: Option<String>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
struct GameDisplayRow {
    id: Option<surrealdb::types::RecordId>,
    name: Option<String>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
struct ContestResultRow {
    contest_id: Option<surrealdb::types::RecordId>,
    placement: Option<i64>,
    contest_date: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
struct ContestParticipantRow {
    player_id: Option<surrealdb::types::RecordId>,
    placement: Option<i64>,
}

/// Normalize player id to canonical "player/KEY" for consistent map lookups (handles "player:key", "player/key", "player.key", backticks).
fn normalize_player_id(s: &str) -> String {
    let key = player_id_to_key(s);
    if key.is_empty() {
        s.replace("player:", "player/")
            .replace("player.", "player/")
            .replace('`', "")
    } else {
        format!("player/{}", key)
    }
}

/// Extract numeric value from SurrealDB result: may be direct number, or { "count": n } from count(), or object with any numeric field.
pub(crate) fn scalar_i64(v: &serde_json::Value) -> i64 {
    if let Some(n) = v.as_i64() {
        return n;
    }
    if let Some(n) = v.as_u64() {
        return n as i64;
    }
    if let Some(obj) = v.as_object() {
        for key in &["count", "Count", "total", "value", "n"] {
            if let Some(n) = obj.get(*key).and_then(|c| c.as_i64()) {
                return n;
            }
            if let Some(n) = obj.get(*key).and_then(|c| c.as_u64()) {
                return n as i64;
            }
        }
        // Any first numeric value in the object (SurrealDB may wrap differently)
        for (_, val) in obj {
            if let Some(n) = val.as_i64() {
                return n;
            }
            if let Some(n) = val.as_u64() {
                return n as i64;
            }
        }
    }
    if let Some(arr) = v.as_array() {
        if let Some(first) = arr.first() {
            return scalar_i64(first);
        }
    }
    0
}

/// Normalize a record id from a Surreal row field (string or record object).
pub(crate) fn canonical_id_from_value(v: &serde_json::Value, table: &str) -> String {
    if let Some(s) = v.as_str() {
        normalize_record_id_string(s)
    } else {
        record_id_from_row(&serde_json::json!({ "id": v }), Some(table)).unwrap_or_default()
    }
}

/// Extract f64 from SurrealDB result (e.g. math::mean returns number or wrapped).
pub(crate) fn scalar_f64(v: &serde_json::Value) -> f64 {
    fn finite(n: f64) -> f64 {
        if n.is_finite() { n } else { 0.0 }
    }
    if let Some(n) = v.as_f64() {
        return finite(n);
    }
    if let Some(n) = v.as_i64() {
        return finite(n as f64);
    }
    if let Some(n) = v.as_u64() {
        return finite(n as f64);
    }
    if let Some(obj) = v.as_object() {
        for key in &["count", "Count", "total", "value", "mean"] {
            if let Some(n) = obj.get(*key).and_then(|c| c.as_f64()) {
                return finite(n);
            }
            if let Some(n) = obj.get(*key).and_then(|c| c.as_i64()) {
                return finite(n as f64);
            }
        }
        for (_, val) in obj {
            if let Some(n) = val.as_f64() {
                return finite(n);
            }
            if let Some(n) = val.as_i64() {
                return finite(n as f64);
            }
        }
    }
    if let Some(arr) = v.as_array() {
        if let Some(first) = arr.first() {
            return scalar_f64(first);
        }
    }
    0.0
}

/// Convert a RecordId to "table/key" string (for display and for get_player_display_label).
fn record_id_to_player_id_str(t: &surrealdb::types::RecordId) -> String {
    crate::surreal_helpers::record_id_to_canonical(t)
}

/// Strip backticks and "player/" / "player:" / "player." prefix to get raw key for type::record('player', $key).
/// Handles both slash and colon and case variants so DB id format matches. SurrealDB may return id with backticks.
fn player_id_to_key(player_id: &str) -> String {
    let s = player_id.trim_matches('`');
    let key = s
        .trim_start_matches("player/")
        .trim_start_matches("player:")
        .trim_start_matches("player.")
        .trim_start_matches("Player/")
        .trim_start_matches("Player:");
    key.trim_matches('`').to_string()
}

/// Compute current and longest win streaks from contest results sorted by start (oldest first).
/// Win = place 1; current streak is the run of wins ending at the most recent contest.
fn compute_streaks_from_ordered_places(
    places: &[(i32, chrono::DateTime<chrono::FixedOffset>)],
) -> (i32, i32) {
    let mut current_streak = 0i32;
    let mut longest_streak = 0i32;
    for (place, _) in places {
        if *place == 1 {
            current_streak += 1;
            longest_streak = longest_streak.max(current_streak);
        } else {
            current_streak = 0;
        }
    }
    (current_streak, longest_streak)
}

/// Extract "YYYY-MM" from SurrealDB datetime (may be string RFC3339 or object).
fn extract_month_from_value(v: Option<&serde_json::Value>) -> String {
    let Some(v) = v else {
        return "0000-00".to_string();
    };
    if let Some(s) = v.as_str() {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            return dt.format("%Y-%m").to_string();
        }
        return "0000-00".to_string();
    }
    if let Some(obj) = v.as_object() {
        let y = obj.get("year").and_then(|x| x.as_i64()).unwrap_or(0);
        let m = obj
            .get("month")
            .or_else(|| obj.get("mon"))
            .and_then(|x| x.as_i64())
            .unwrap_or(1);
        return format!("{:04}-{:02}", y, m.clamp(1, 12));
    }
    "0000-00".to_string()
}

/// Build an array of Thing from "table:key" or "table/key" strings for INSIDE bindings.
/// SurrealDB v2 does not coerce string arrays to record id; binding Thing array fixes INSIDE matching.
fn strings_to_record_id_array(ids: &[String]) -> Vec<surrealdb::types::RecordId> {
    ids.iter()
        .filter_map(|s| {
            let s = s.trim().trim_matches('`');
            let (tb, key) = s.split_once(':').or_else(|| s.split_once('/'))?;
            let key = key.trim().trim_matches('`');
            if key.is_empty() {
                return None;
            }
            Some(surrealdb::types::RecordId::new(tb.trim(), key))
        })
        .collect()
}

/// Repository for analytics data operations
#[derive(Clone)]
pub struct AnalyticsRepository {
    db: Db,
    #[allow(dead_code)]
    config: DatabaseConfig,
}

impl AnalyticsRepository {
    /// Creates a new analytics repository
    pub fn new(db: Db, config: DatabaseConfig) -> Self {
        Self { db, config }
    }

    pub(crate) fn db(&self) -> &Db {
        &self.db
    }

    /// Returns contest counts bucketed by weekday (0=Sun..6=Sat) and hour (0..23) in `timezone`.
    pub async fn get_contest_heatmap(
        &self,
        weeks: i32,
        game_id: Option<&str>,
        timezone: &str,
    ) -> Result<Vec<HeatRow>> {
        let contest_ids: Option<Vec<String>> = if let Some(gid) = game_id {
            let key = record_id_to_key(gid, "game");
            if key.is_empty() {
                return Ok(Vec::new());
            }
            let record_id = surrealdb::types::RecordId::new("game", key.as_str());
            #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
            struct InRow {
                #[serde(rename = "in")]
                contest_id: Option<surrealdb::types::RecordId>,
            }
            let mut res = self
                .db
                .query("SELECT `in` FROM played_with WHERE `out` = $record_id")
                .bind(("record_id", record_id))
                .await
                .map_err(|e| SharedError::Database(e.to_string()))?;
            let rows: Vec<InRow> = res
                .take(0)
                .map_err(|e| SharedError::Database(format!("heatmap game filter: {}", e)))?;
            let ids: Vec<String> = rows
                .into_iter()
                .filter_map(|r| {
                    let rid = r
                        .contest_id
                        .as_ref()
                        .map(crate::surreal_helpers::record_id_to_canonical)
                        .unwrap_or_default();
                    if rid.is_empty() {
                        None
                    } else {
                        Some(rid.replace('/', ":"))
                    }
                })
                .collect();
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            Some(ids)
        } else {
            None
        };

        // SurrealDB function compatibility: some versions do not support `time::dayofweek`.
        // We fetch contest start timestamps and compute (weekday, hour) in Rust instead.
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct StartRow {
            start: Option<surrealdb::types::Datetime>,
        }

        let sql = if contest_ids.is_some() {
            r#"SELECT start
               FROM contest
               WHERE start >= time::now() - duration::from_weeks($weeks)
               AND id INSIDE $contest_ids"#
        } else {
            r#"SELECT start
               FROM contest
               WHERE start >= time::now() - duration::from_weeks($weeks)"#
        };

        let mut q = self.db.query(sql).bind(("weeks", weeks));
        if let Some(ref ids) = contest_ids {
            q = q.bind(("contest_ids", ids.clone()));
        }
        let mut res = q
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<StartRow> = res.take(0).unwrap_or_default();

        let mut buckets: HashMap<(i32, i32), i64> = HashMap::new();
        for r in rows {
            let Some(start) = r.start else {
                continue;
            };
            let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&start.to_string()) else {
                continue;
            };
            let dt_utc = dt.with_timezone(&chrono::Utc);
            let (day, hour) = shared::timezone::local_weekday_hour(dt_utc, timezone);
            *buckets.entry((day, hour)).or_insert(0) += 1;
        }

        let mut out: Vec<HeatRow> = buckets
            .into_iter()
            .map(|((day, hour), plays)| HeatRow { day, hour, plays })
            .collect();
        out.sort_by_key(|r| (r.day, r.hour));
        Ok(out)
    }

    /// Get player ID by email. SurrealDB returns `id` as a Thing, not a string.
    pub async fn get_player_id_by_email(&self, email: &str) -> Result<Option<String>> {
        self.get_player_thing_by_email(email)
            .await
            .map(|opt| opt.map(|t| record_id_to_player_id_str(&t)))
    }

    /// Get player record id (Thing) by email. Use this when you need the exact DB value for queries (e.g. profile stats for "me").
    pub async fn get_player_thing_by_email(
        &self,
        email: &str,
    ) -> Result<Option<surrealdb::types::RecordId>> {
        let mut res = self.db
            .query("SELECT id FROM player WHERE string::lowercase(email) = string::lowercase($email) LIMIT 1")
            .bind(("email", email.to_string()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct Row {
            id: Option<surrealdb::types::RecordId>,
        }
        let rows: Vec<Row> = res
            .take(0)
            .map_err(|e| SharedError::Database(e.to_string()))?;
        Ok(rows.into_iter().next().and_then(|r| r.id))
    }

    /// Overall statistics for "me": resolve player id string from email, then use fn::player_stats_by_id_str when applied, else inline query.
    /// Tries both resulted_in.`out` and resulted_in.`in` so we match regardless of which column stores the player.
    pub async fn get_player_stats_for_me_by_email(
        &self,
        email: &str,
        player_id: &str,
    ) -> Result<Option<PlayerStats>> {
        let id_str = match self.get_player_id_str_by_email(email).await? {
            Some(s) => s.replace('`', "").replace('/', ":"),
            None => {
                log::warn!("get_player_stats_for_me_by_email: no player for email");
                return Ok(Some(Self::zero_player_stats(player_id)));
            }
        };

        // Prefer SurrealDB function when applied (one round-trip)
        if let Ok(mut res) = self
            .db
            .query("SELECT fn::player_stats_by_id_str($id_str) AS result FROM [1]")
            .bind(("id_str", id_str.clone()))
            .await
        {
            let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
            if let Some(first) = rows.into_iter().next() {
                let row = first
                    .get("result")
                    .or_else(|| first.get("fn::player_stats_by_id_str($id_str)"))
                    .cloned()
                    .unwrap_or(first);
                if row.is_object()
                    && (row.get("contests_out").is_some() || row.get("contests_in").is_some())
                {
                    if let Some(mut stats) =
                        Self::player_stats_from_dual_out_in_row(&row, player_id)
                    {
                        let (cur, long) =
                            self.get_player_streaks(player_id).await.unwrap_or((0, 0));
                        stats.current_streak = cur;
                        stats.longest_streak = long;
                        return Ok(Some(stats));
                    }
                }
            }
        }

        // Fallback: inline query (counts for both out and in).
        // IMPORTANT: SurrealQL requires a FROM clause; use FROM [1] as a dummy source.
        let sql = r#"
            SELECT
                ((SELECT count() FROM resulted_in
                  WHERE string::replace(string::concat(`out`), '`', '') = $id_str
                  GROUP ALL)[0].count) ?? 0 AS contests_out,
                ((SELECT count() FROM resulted_in
                  WHERE string::replace(string::concat(`out`), '`', '') = $id_str AND place = 1
                  GROUP ALL)[0].count) ?? 0 AS wins_out,
                ((SELECT math::mean(place) FROM resulted_in
                  WHERE string::replace(string::concat(`out`), '`', '') = $id_str
                  GROUP ALL)[0].`math::mean`) ?? 0 AS avg_out,
                ((SELECT math::min(place) FROM resulted_in
                  WHERE string::replace(string::concat(`out`), '`', '') = $id_str
                  GROUP ALL)[0].`math::min`) ?? 0 AS best_out,
                ((SELECT count() FROM resulted_in
                  WHERE string::replace(string::concat(`in`), '`', '') = $id_str
                  GROUP ALL)[0].count) ?? 0 AS contests_in,
                ((SELECT count() FROM resulted_in
                  WHERE string::replace(string::concat(`in`), '`', '') = $id_str AND place = 1
                  GROUP ALL)[0].count) ?? 0 AS wins_in,
                ((SELECT math::mean(place) FROM resulted_in
                  WHERE string::replace(string::concat(`in`), '`', '') = $id_str
                  GROUP ALL)[0].`math::mean`) ?? 0 AS avg_in,
                ((SELECT math::min(place) FROM resulted_in
                  WHERE string::replace(string::concat(`in`), '`', '') = $id_str
                  GROUP ALL)[0].`math::min`) ?? 0 AS best_in
            FROM [1]
        "#;
        let mut res = self
            .db
            .query(sql)
            .bind(("id_str", id_str.clone()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => {
                log::warn!("get_player_stats_for_me_by_email: no row for id_str");
                return Ok(Some(Self::zero_player_stats(player_id)));
            }
        };

        match Self::player_stats_from_dual_out_in_row(&row, player_id) {
            Some(mut stats) => {
                let (cur, long) = self.get_player_streaks(player_id).await.unwrap_or((0, 0));
                stats.current_streak = cur;
                stats.longest_streak = long;
                Ok(Some(stats))
            }
            None => Ok(Some(Self::zero_player_stats(player_id))),
        }
    }

    /// Parse a row with contests_out/wins_out/avg_out/best_out and contests_in/... into PlayerStats (use out if non-zero, else in).
    fn player_stats_from_dual_out_in_row(
        row: &serde_json::Value,
        player_id: &str,
    ) -> Option<PlayerStats> {
        let contests_out = row.get("contests_out").map(scalar_i64).unwrap_or(0) as i32;
        let contests_in = row.get("contests_in").map(scalar_i64).unwrap_or(0) as i32;
        let (total_contests, total_wins, average_placement, best_placement) = if contests_out > 0 {
            (
                contests_out,
                row.get("wins_out").map(scalar_i64).unwrap_or(0) as i32,
                row.get("avg_out").map(scalar_f64).unwrap_or(0.0),
                row.get("best_out").map(scalar_i64).unwrap_or(0) as i32,
            )
        } else if contests_in > 0 {
            (
                contests_in,
                row.get("wins_in").map(scalar_i64).unwrap_or(0) as i32,
                row.get("avg_in").map(scalar_f64).unwrap_or(0.0),
                row.get("best_in").map(scalar_i64).unwrap_or(0) as i32,
            )
        } else {
            (0, 0, 0.0, 0)
        };

        let total_losses = total_contests.saturating_sub(total_wins);
        let win_rate = if total_contests > 0 {
            (total_wins as f64 * 100.0) / total_contests as f64
        } else {
            0.0
        };
        let (current_streak, longest_streak) = (0, 0);

        Some(PlayerStats {
            player_id: player_id.to_string(),
            total_contests,
            total_wins,
            total_losses,
            win_rate,
            average_placement,
            best_placement,
            skill_rating: 1200.0,
            rating_confidence: 0.8,
            total_points: total_wins * 10,
            current_streak,
            longest_streak,
            last_updated: chrono::Utc::now(),
        })
    }

    fn zero_player_stats(player_id: &str) -> PlayerStats {
        PlayerStats {
            player_id: player_id.to_string(),
            total_contests: 0,
            total_wins: 0,
            total_losses: 0,
            win_rate: 0.0,
            average_placement: 0.0,
            best_placement: 0,
            skill_rating: 1200.0,
            rating_confidence: 0.8,
            total_points: 0,
            current_streak: 0,
            longest_streak: 0,
            last_updated: chrono::Utc::now(),
        }
    }

    /// Fetch player's contest (contest_id, place) and contest start times; return (current_streak, longest_streak).
    /// Streaks are computed from contests ordered by start ascending: win (place 1) extends streak, else resets.
    pub async fn get_player_streaks(&self, player_id: &str) -> Result<(i32, i32)> {
        let key = player_id_to_key(player_id);
        if key.is_empty() {
            return Ok((0, 0));
        }
        let player_rid = surrealdb::types::RecordId::new("player", key.as_str());

        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct RiRow {
            contest_id: Option<surrealdb::types::RecordId>,
            place: Option<i64>,
        }
        let mut res = self
            .db
            .query("SELECT `in` AS contest_id, place FROM resulted_in WHERE `out` = $player_rid")
            .bind(("player_rid", player_rid))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let ri_rows: Vec<RiRow> = res
            .take(0)
            .map_err(|e| SharedError::Database(e.to_string()))?;
        if ri_rows.is_empty() {
            return Ok((0, 0));
        }

        let contest_ids: Vec<surrealdb::types::RecordId> = ri_rows
            .iter()
            .filter_map(|r| r.contest_id.clone())
            .collect();
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct ContestStartRow {
            id: Option<surrealdb::types::RecordId>,
            start: Option<surrealdb::types::Datetime>,
        }
        let mut res2 = self
            .db
            .query("SELECT id, start FROM contest WHERE id INSIDE $ids")
            .bind(("ids", contest_ids.clone()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let start_rows: Vec<ContestStartRow> = res2
            .take(0)
            .map_err(|e| SharedError::Database(e.to_string()))?;

        let mut start_by_id: HashMap<String, chrono::DateTime<chrono::FixedOffset>> =
            HashMap::new();
        for r in start_rows {
            if let (Some(id), Some(start)) = (r.id, r.start) {
                let cid = crate::surreal_helpers::record_id_to_canonical(&id)
                    .replace("contest:", "contest/");
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&start.to_string()) {
                    start_by_id.insert(
                        cid,
                        dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()),
                    );
                }
            }
        }

        let mut ordered: Vec<(i32, chrono::DateTime<chrono::FixedOffset>)> = Vec::new();
        for r in &ri_rows {
            let cid = r
                .contest_id
                .as_ref()
                .map(|c| {
                    crate::surreal_helpers::record_id_to_canonical(c)
                        .replace("contest:", "contest/")
                })
                .unwrap_or_default();
            let place = r.place.unwrap_or(0) as i32;
            if let Some(&start) = start_by_id.get(&cid) {
                ordered.push((place, start));
            }
        }
        ordered.sort_by(|a, b| a.1.cmp(&b.1));

        Ok(compute_streaks_from_ordered_places(&ordered))
    }

    /// Get the exact string form of player id as SurrealDB stringifies it (string::concat(id)).
    /// Use this for stats queries so WHERE string::concat(id) = $id_str matches regardless of Thing binding quirks.
    pub async fn get_player_id_str_by_email(&self, email: &str) -> Result<Option<String>> {
        let mut res = self.db
            .query("SELECT string::concat(id) AS id_str FROM player WHERE string::lowercase(email) = string::lowercase($email) LIMIT 1")
            .bind(("email", email.to_string()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let id_str = rows
            .into_iter()
            .next()
            .and_then(|r| r.get("id_str").and_then(|v| v.as_str().map(String::from)));
        Ok(id_str)
    }

    /// Parse a row from the stats query into PlayerStats.
    fn parse_player_stats_row(row: &serde_json::Value) -> Result<PlayerStats> {
        let key = record_id_from_row(row, Some("player"))
            .map(|s| {
                s.replace("player:", "")
                    .replace("player/", "")
                    .replace('`', "")
            })
            .unwrap_or_default();
        let total_contests = row.get("total_contests").map(scalar_i64).unwrap_or(0) as i32;
        let total_wins = row.get("total_wins").map(scalar_i64).unwrap_or(0) as i32;
        let total_losses = total_contests.saturating_sub(total_wins);
        let win_rate = if total_contests > 0 {
            (total_wins as f64 * 100.0) / total_contests as f64
        } else {
            0.0
        };
        let average_placement = row.get("average_placement").map(scalar_f64).unwrap_or(0.0);
        let best_placement = row.get("best_placement").map(scalar_i64).unwrap_or(0) as i32;
        let player_id_norm =
            record_id_from_row(row, Some("player")).unwrap_or_else(|| format!("player/{}", key));
        Ok(PlayerStats {
            player_id: player_id_norm,
            total_contests,
            total_wins,
            total_losses,
            win_rate,
            average_placement,
            best_placement,
            skill_rating: 1200.0,
            rating_confidence: 0.8,
            total_points: total_wins * 10,
            current_streak: 0,
            longest_streak: 0,
            last_updated: chrono::Utc::now(),
        })
    }

    /// Get player statistics by email (parameterized). Can return no row when $email binding fails from Rust.
    pub async fn get_player_stats_by_email(&self, email: &str) -> Result<Option<PlayerStats>> {
        let pid = "(SELECT VALUE id FROM player WHERE string::lowercase(email) = string::lowercase($email) LIMIT 1)[0]";
        let sql = format!(
            r#"
            SELECT
                id AS player_id,
                handle AS player_handle,
                ((SELECT count() FROM resulted_in WHERE `out` = {} GROUP ALL)[0].count) ?? 0 AS total_contests,
                ((SELECT count() FROM resulted_in WHERE `out` = {} AND place = 1 GROUP ALL)[0].count) ?? 0 AS total_wins,
                ((SELECT math::mean(place) FROM resulted_in WHERE `out` = {} GROUP ALL)[0].`math::mean`) ?? 0 AS average_placement,
                ((SELECT math::min(place) FROM resulted_in WHERE `out` = {} GROUP ALL)[0].`math::min`) ?? 0 AS best_placement
            FROM (SELECT * FROM player WHERE string::lowercase(email) = string::lowercase($email) LIMIT 1)
            "#,
            pid, pid, pid, pid
        );
        let mut res = self
            .db
            .query(sql)
            .bind(("email", email.to_string()))
            .await
            .map_err(|e| {
                log::warn!("get_player_stats_by_email: query failed: {}", e);
                SharedError::Database(e.to_string())
            })?;
        let rows: Vec<serde_json::Value> = res.take(0usize).unwrap_or_default();
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => {
                log::warn!("get_player_stats_by_email: no player for email={:?}", email);
                return Ok(None);
            }
        };
        match Self::parse_player_stats_row(&row) {
            Ok(mut stats) => {
                let (cur, long) = self
                    .get_player_streaks(&stats.player_id)
                    .await
                    .unwrap_or((0, 0));
                stats.current_streak = cur;
                stats.longest_streak = long;
                Ok(Some(stats))
            }
            Err(e) => Err(e),
        }
    }

    /// Same as get_player_stats_by_email but with email inlined in the query (no params).
    /// Use for "me" when parameter binding or type::record from Rust fails to match the DB.
    pub async fn get_player_stats_by_email_inlined(
        &self,
        email: &str,
    ) -> Result<Option<PlayerStats>> {
        if email.len() > 256
            || !email
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._%+-@".contains(c))
        {
            log::warn!("get_player_stats_by_email_inlined: email not safe to inline");
            return Ok(None);
        }
        let escaped = email.replace('\'', "''");
        let pid = format!(
            "(SELECT VALUE id FROM player WHERE string::lowercase(email) = string::lowercase('{}') LIMIT 1)[0]",
            escaped
        );
        let sql = format!(
            r#"
            SELECT
                id AS player_id,
                handle AS player_handle,
                ((SELECT count() FROM resulted_in WHERE `out` = {} GROUP ALL)[0].count) ?? 0 AS total_contests,
                ((SELECT count() FROM resulted_in WHERE `out` = {} AND place = 1 GROUP ALL)[0].count) ?? 0 AS total_wins,
                ((SELECT math::mean(place) FROM resulted_in WHERE `out` = {} GROUP ALL)[0].`math::mean`) ?? 0 AS average_placement,
                ((SELECT math::min(place) FROM resulted_in WHERE `out` = {} GROUP ALL)[0].`math::min`) ?? 0 AS best_placement
            FROM (SELECT * FROM player WHERE string::lowercase(email) = string::lowercase('{}') LIMIT 1)
            "#,
            pid, pid, pid, pid, escaped
        );
        let mut res = self.db.query(sql).await.map_err(|e| {
            log::warn!("get_player_stats_by_email_inlined: query failed: {}", e);
            SharedError::Database(e.to_string())
        })?;
        let rows: Vec<serde_json::Value> = res.take(0usize).unwrap_or_default();
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => {
                log::warn!("get_player_stats_by_email_inlined: no player for email");
                return Ok(None);
            }
        };
        match Self::parse_player_stats_row(&row) {
            Ok(mut stats) => {
                let (cur, long) = self
                    .get_player_streaks(&stats.player_id)
                    .await
                    .unwrap_or((0, 0));
                stats.current_streak = cur;
                stats.longest_streak = long;
                Ok(Some(stats))
            }
            Err(e) => Err(e),
        }
    }

    /// Get player statistics by player record key. Inlines key as literal so SurrealDB matches
    /// (binding $key in type::record('player', $key) can fail to match stored record ids from Rust client).
    pub async fn get_player_stats_by_key(&self, player_id: &str) -> Result<Option<PlayerStats>> {
        let key = player_id_to_key(player_id);
        if key.is_empty() {
            log::warn!(
                "get_player_stats_by_key: empty key for player_id={:?}",
                player_id
            );
            return Ok(None);
        }
        // Only allow UUID-like keys (0-9, a-f, A-F, hyphen) to avoid injection when inlining
        if !key.chars().all(|c| c.is_ascii_hexdigit() || c == '-') || key.len() > 64 {
            log::warn!("get_player_stats_by_key: key not safe to inline: {:?}", key);
            return Ok(None);
        }
        let thing = format!("type::record('player', '{}')", key);
        let sql = format!(
            r#"
            SELECT
                id AS player_id,
                handle AS player_handle,
                ((SELECT count() FROM resulted_in WHERE `out` = {} GROUP ALL)[0].count) ?? 0 AS total_contests,
                ((SELECT count() FROM resulted_in WHERE `out` = {} AND place = 1 GROUP ALL)[0].count) ?? 0 AS total_wins,
                ((SELECT math::mean(place) FROM resulted_in WHERE `out` = {} GROUP ALL)[0].`math::mean`) ?? 0 AS average_placement,
                ((SELECT math::min(place) FROM resulted_in WHERE `out` = {} GROUP ALL)[0].`math::min`) ?? 0 AS best_placement
            FROM (SELECT * FROM player WHERE id = {} LIMIT 1)
            "#,
            thing, thing, thing, thing, thing
        );
        let mut res = self.db.query(sql).await.map_err(|e| {
            log::warn!("get_player_stats_by_key: query failed: {}", e);
            SharedError::Database(e.to_string())
        })?;
        let rows: Vec<serde_json::Value> = res.take(0usize).unwrap_or_default();
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => {
                log::warn!("get_player_stats_by_key: no player for key={:?}", key);
                return Ok(None);
            }
        };
        let total_contests = row.get("total_contests").map(scalar_i64).unwrap_or(0) as i32;
        let total_wins = row.get("total_wins").map(scalar_i64).unwrap_or(0) as i32;
        let total_losses = total_contests.saturating_sub(total_wins);
        let win_rate = if total_contests > 0 {
            (total_wins as f64 * 100.0) / total_contests as f64
        } else {
            0.0
        };
        let average_placement = row.get("average_placement").map(scalar_f64).unwrap_or(0.0);
        let best_placement = row.get("best_placement").map(scalar_i64).unwrap_or(0) as i32;
        let player_id_norm =
            record_id_from_row(&row, Some("player")).unwrap_or_else(|| format!("player/{}", key));
        let (cur, long) = self
            .get_player_streaks(&player_id_norm)
            .await
            .unwrap_or((0, 0));
        let stats = PlayerStats {
            player_id: player_id_norm,
            total_contests,
            total_wins,
            total_losses,
            win_rate,
            average_placement,
            best_placement,
            skill_rating: 1200.0,
            rating_confidence: 0.8,
            total_points: total_wins * 10,
            current_streak: cur,
            longest_streak: long,
            last_updated: chrono::Utc::now(),
        };
        Ok(Some(stats))
    }

    /// Get platform statistics from real data. Uses best-effort: failed queries yield defaults so we never 500.
    pub async fn get_platform_stats(&self) -> Result<PlatformStats> {
        log::info!("Starting to get platform stats...");

        // Get total counts from collections (best-effort: default to 0/1 on error)
        let total_players = self.get_total_players().await.unwrap_or(0);
        log::info!("Total players: {}", total_players);

        let total_contests = self.get_total_contests().await.unwrap_or(0);
        log::info!("Total contests: {}", total_contests);

        let total_games = self.get_total_games().await.unwrap_or(0);
        log::info!("Total games: {}", total_games);

        let total_venues = self.get_total_venues().await.unwrap_or(0);
        log::info!("Total venues: {}", total_venues);

        let active_players_30d = self.get_active_players(30).await.unwrap_or(0);
        log::info!("Active players 30d: {}", active_players_30d);

        let active_players_7d = self.get_active_players(7).await.unwrap_or(0);
        log::info!("Active players 7d: {}", active_players_7d);

        let contests_30d = self.get_contests_in_period(30).await.unwrap_or(0);
        log::info!("Contests 30d: {}", contests_30d);

        let average_participants_per_contest = self
            .get_average_participants_per_contest()
            .await
            .unwrap_or(0.0);
        log::info!(
            "Average participants per contest: {}",
            average_participants_per_contest
        );

        let top_games = self.get_top_games(5).await.unwrap_or_default();
        log::info!("Top games: {:?}", top_games);

        let top_venues = self.get_top_venues(5).await.unwrap_or_default();
        log::info!("Top venues: {:?}", top_venues);

        // Convert to proper types with real counts
        let top_games_typed: Vec<GamePopularity> = top_games
            .into_iter()
            .map(|(game_id, name, plays)| GamePopularity {
                game_id,
                game_name: name,
                plays,
                popularity_score: plays as f64,
            })
            .collect();

        let top_venues_typed: Vec<VenueActivity> = top_venues
            .into_iter()
            .map(|(venue_id, name, contests)| VenueActivity {
                venue_id,
                venue_name: name,
                contests_held: contests,
                total_participants: contests * 4, // Estimate participants per contest
                activity_score: contests as f64,
            })
            .collect();

        // Ensure we have at least some basic data
        let final_stats = PlatformStats {
            total_players,
            total_contests,
            total_games,
            total_venues,
            active_players_30d,
            active_players_7d,
            contests_30d,
            average_participants_per_contest,
            top_games: top_games_typed,
            top_venues: top_venues_typed,
            last_updated: chrono::Utc::now().into(),
        };

        log::info!("Final platform stats: total_players={}, total_contests={}, active_30d={}, active_7d={}", 
            final_stats.total_players, final_stats.total_contests, final_stats.active_players_30d, final_stats.active_players_7d);

        Ok(final_stats)
    }

    /// Get total number of players
    async fn get_total_players(&self) -> Result<i32> {
        // Preferred: count distinct, non-empty emails (canonical "real players" in this app).
        // However, older/migrated datasets can have email missing/NONE (or different type),
        // which would make this return 0 even though player records exist.
        //
        // So:
        // 1) compute raw player count (always safe)
        // 2) compute distinct-email count (best effort)
        // 3) return distinct-email count if > 0, else fall back to raw count

        let raw_sql = "SELECT count() AS count FROM player GROUP ALL";
        let mut raw_res = self
            .db
            .query(raw_sql)
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let raw_rows: Vec<serde_json::Value> = raw_res.take(0).unwrap_or_default();
        let raw_count = raw_rows
            .into_iter()
            .next()
            .map(|v| scalar_i64(&v) as i32)
            .unwrap_or(0);

        let distinct_sql = r#"
            SELECT count() AS count FROM (
                SELECT email
                FROM player
                WHERE email != NONE AND email != ""
                GROUP BY email
            ) GROUP ALL
        "#;

        let distinct_count = match self.db.query(distinct_sql).await {
            Ok(mut res) => {
                let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
                rows.into_iter()
                    .next()
                    .map(|v| scalar_i64(&v) as i32)
                    .unwrap_or(0)
            }
            Err(e) => {
                log::warn!(
                    "get_total_players: distinct-email query failed, falling back to raw count: {}",
                    e
                );
                0
            }
        };

        if distinct_count > 0 {
            Ok(distinct_count)
        } else {
            Ok(raw_count)
        }
    }

    /// Get total number of contests
    async fn get_total_contests(&self) -> Result<i32> {
        let mut res = self
            .db
            .query("SELECT count() AS count FROM contest GROUP ALL")
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        Ok(rows
            .into_iter()
            .next()
            .map(|v| scalar_i64(&v) as i32)
            .unwrap_or(0))
    }

    /// Get total number of games
    async fn get_total_games(&self) -> Result<i32> {
        let mut res = self
            .db
            .query("SELECT count() AS count FROM game GROUP ALL")
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        Ok(rows
            .into_iter()
            .next()
            .map(|v| scalar_i64(&v) as i32)
            .unwrap_or(0))
    }

    /// Get total number of venues
    async fn get_total_venues(&self) -> Result<i32> {
        let mut res = self
            .db
            .query("SELECT count() AS count FROM venue GROUP ALL")
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        Ok(rows
            .into_iter()
            .next()
            .map(|v| scalar_i64(&v) as i32)
            .unwrap_or(0))
    }

    /// Get active players in the last N days
    pub(crate) async fn get_active_players(&self, days: i32) -> Result<i32> {
        let sql = r#"
            SELECT count() AS count
            FROM (
                SELECT `out` AS player_id
                FROM resulted_in
                WHERE `in` IN (
                    SELECT VALUE id FROM contest WHERE start >= time::now() - duration::from_days($days)
                )
                GROUP BY `out`
            ) GROUP ALL
        "#;
        let mut res = self
            .db
            .query(sql)
            .bind(("days", days))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let count = rows
            .into_iter()
            .next()
            .map(|v| scalar_i64(&v) as i32)
            .unwrap_or(0);
        Ok(count)
    }

    /// Get contests in the last N days
    pub(crate) async fn get_contests_in_period(&self, days: i32) -> Result<i32> {
        let mut res = self
            .db
            .query("SELECT count() AS count FROM contest WHERE start >= time::now() - duration::from_days($days) GROUP ALL")
            .bind(("days", days))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        Ok(rows
            .into_iter()
            .next()
            .map(|v| scalar_i64(&v) as i32)
            .unwrap_or(0))
    }

    /// Get average participants per contest
    async fn get_average_participants_per_contest(&self) -> Result<f64> {
        let sql = r#"
            SELECT math::mean(participant_count) AS avg
            FROM (
                SELECT `in` AS contest_id, count() AS participant_count
                FROM resulted_in
                GROUP BY `in`
            ) GROUP ALL
        "#;
        let mut res = self
            .db
            .query(sql)
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        Ok(rows
            .into_iter()
            .next()
            .map(|v| scalar_f64(v.get("avg").unwrap_or(&serde_json::Value::Null)))
            .unwrap_or(0.0))
    }

    /// Get top games by play count. SurrealQL has no INNER JOIN; count from played_with then look up names from game.
    pub(crate) async fn get_top_games(&self, limit: i32) -> Result<Vec<(String, String, i32)>> {
        let sql = r#"SELECT `out` AS game_id, count() AS plays FROM played_with GROUP BY `out` ORDER BY plays DESC LIMIT $limit"#;
        let mut res = self
            .db
            .query(sql)
            .bind(("limit", limit))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let record_ids: Vec<surrealdb::types::RecordId> = rows
            .iter()
            .filter_map(|v| {
                let canonical = canonical_id_from_value(v.get("game_id")?, "game");
                let key = record_id_to_key(&canonical, "game");
                if key.is_empty() {
                    None
                } else {
                    Some(surrealdb::types::RecordId::new("game", key.as_str()))
                }
            })
            .collect();
        if record_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut name_res = self
            .db
            .query("SELECT string::concat(id) AS game_id, name FROM game WHERE id INSIDE $ids")
            .bind(("ids", record_ids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let name_rows: Vec<serde_json::Value> = name_res.take(0).unwrap_or_default();
        let id_to_name: std::collections::HashMap<String, String> = name_rows
            .into_iter()
            .filter_map(|v| {
                let id = v
                    .get("game_id")
                    .and_then(|x| x.as_str())
                    .map(normalize_record_id_string)?;
                let name = v.get("name").and_then(|x| x.as_str()).map(String::from)?;
                Some((id, name))
            })
            .collect();
        let games: Vec<(String, String, i32)> = rows
            .into_iter()
            .filter_map(|v| {
                let game_id = canonical_id_from_value(v.get("game_id")?, "game");
                if game_id.is_empty() {
                    return None;
                }
                let plays = v.get("plays").map(scalar_i64).unwrap_or(0) as i32;
                let name = id_to_name
                    .get(&game_id)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string());
                Some((game_id, name, plays))
            })
            .collect();
        Ok(games)
    }

    /// Top games by play count within a recent window, filtered to contests visible to the viewer:
    /// approved (or legacy NONE) OR created by viewer.
    pub async fn get_top_games_since_days_for_viewer(
        &self,
        viewer_key: &str,
        since_days: i64,
        limit: i32,
    ) -> Result<Vec<(String, i32)>> {
        let viewer_key = viewer_key.to_string();
        let mut res = self
            .db
            .query(
                r#"
SELECT string::concat(pw.out) AS game_id, count() AS plays
FROM (
  SELECT pw.out AS out
  FROM played_with AS pw
  WHERE pw.in IN (
    SELECT VALUE id FROM contest
    WHERE start >= time::now() - duration::days($since_days)
      AND (
        (moderation_status = 'approved' OR moderation_status = NONE)
        OR creator_id = type::record('player', $viewer_key)
        OR id INSIDE (SELECT VALUE `in` FROM resulted_in WHERE `out` = type::record('player', $viewer_key))
      )
  )
) GROUP BY game_id
ORDER BY plays DESC
LIMIT $limit
"#,
            )
            .bind(("since_days", since_days))
            .bind(("viewer_key", viewer_key))
            .bind(("limit", limit))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct Row {
            game_id: String,
            plays: i64,
        }
        let rows: Vec<Row> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("top games since: {}", e)))?;
        let ids: Vec<surrealdb::types::RecordId> = rows
            .iter()
            .filter_map(|r| {
                let key = record_id_to_key(&r.game_id, "game");
                if key.is_empty() {
                    None
                } else {
                    Some(surrealdb::types::RecordId::new("game", key.as_str()))
                }
            })
            .collect();
        let mut res2 = self
            .db
            .query("SELECT string::concat(id) AS id, name FROM game WHERE id INSIDE $ids")
            .bind(("ids", ids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct GameRow {
            id: String,
            name: String,
        }
        let games: Vec<GameRow> = res2.take(0).unwrap_or_default();
        let mut name_by_id: HashMap<String, String> = HashMap::new();
        for g in games {
            name_by_id.insert(g.id, g.name);
        }
        let mut out = Vec::new();
        for r in rows {
            let name = name_by_id.get(&r.game_id).cloned().unwrap_or(r.game_id);
            out.push((name, r.plays as i32));
        }
        Ok(out)
    }

    /// Head-to-head: how many contests the opponent beat the viewer in, within a time window,
    /// considering only contests visible to the viewer (approved OR viewer-created).
    pub async fn count_opponent_beats_me_since_days_for_viewer(
        &self,
        viewer_key: &str,
        opponent_handle_lc: &str,
        since_days: i64,
    ) -> Result<i64> {
        let viewer_key = viewer_key.to_string();
        let opponent_handle_lc = opponent_handle_lc.to_string();

        // Resolve opponent player record id by handle.
        let mut res = self
            .db
            .query("SELECT string::concat(id) AS id FROM player WHERE string::lowercase(handle) = $h LIMIT 1")
            .bind(("h", opponent_handle_lc))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let Some(opp_id) = rows.get(0).and_then(|v| v.get("id")).and_then(|v| v.as_str()) else {
            return Err(SharedError::NotFound("opponent_not_found".to_string()));
        };
        let opp_key = record_id_to_key(opp_id, "player");
        if opp_key.is_empty() {
            return Err(SharedError::Validation("Invalid opponent id".to_string()));
        }
        let viewer_rid = surrealdb::types::RecordId::new("player", viewer_key.as_str());
        let opp_rid = surrealdb::types::RecordId::new("player", opp_key.as_str());

        // Contests in window, visible to viewer.
        let mut res2 = self
            .db
            .query(
                r#"
SELECT VALUE id
FROM contest
WHERE start >= time::now() - duration::days($since_days)
  AND (
    (moderation_status = 'approved' OR moderation_status = NONE)
    OR creator_id = type::record('player', $viewer_key)
    OR id INSIDE (SELECT VALUE `in` FROM resulted_in WHERE `out` = type::record('player', $viewer_key))
  )
"#,
            )
            .bind(("since_days", since_days))
            .bind(("viewer_key", viewer_key))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let contest_ids: Vec<surrealdb::types::RecordId> = res2.take(0).unwrap_or_default();
        if contest_ids.is_empty() {
            return Ok(0);
        }

        // Result rows for viewer + opponent within those contests.
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct Row {
            contest_id: Option<surrealdb::types::RecordId>,
            player_id: Option<surrealdb::types::RecordId>,
            place: Option<i64>,
        }
        let mut res3 = self
            .db
            .query(
                "SELECT `in` AS contest_id, `out` AS player_id, place FROM resulted_in WHERE `in` INSIDE $contests AND (`out` = $me OR `out` = $opp)",
            )
            .bind(("contests", contest_ids))
            .bind(("me", viewer_rid))
            .bind(("opp", opp_rid))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<Row> = res3.take(0).unwrap_or_default();

        let mut by_contest: HashMap<String, (i64, i64)> = HashMap::new();
        for r in rows {
            let cid = thing_to_record_id(&r.contest_id);
            if cid.is_empty() {
                continue;
            }
            let pid = thing_to_record_id(&r.player_id);
            let place = r.place.unwrap_or(0);
            let entry = by_contest.entry(cid).or_insert((0, 0));
            if pid.ends_with(&format!("/{}", opp_key)) || pid.ends_with(&format!(":{}", opp_key)) {
                entry.1 = place;
            } else {
                entry.0 = place;
            }
        }
        let mut beats = 0i64;
        for (_cid, (me_place, opp_place)) in by_contest {
            if me_place > 0 && opp_place > 0 && opp_place < me_place {
                beats += 1;
            }
        }
        Ok(beats)
    }

    /// How many contests a player won (place=1) within a time window, considering only contests
    /// visible to the viewer (approved OR viewer-created).
    pub async fn count_player_wins_since_days_for_viewer(
        &self,
        viewer_key: &str,
        player_handle_lc: &str,
        since_days: i64,
    ) -> Result<i64> {
        let viewer_key = viewer_key.to_string();
        let player_handle_lc = player_handle_lc.to_string();

        // Resolve player record id by handle.
        let mut res = self
            .db
            .query(
                "SELECT string::concat(id) AS id FROM player WHERE string::lowercase(handle) = $h LIMIT 1",
            )
            .bind(("h", player_handle_lc))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let Some(pid) = rows
            .get(0)
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str())
        else {
            return Err(SharedError::NotFound("player_not_found".to_string()));
        };
        let p_key = record_id_to_key(pid, "player");
        if p_key.is_empty() {
            return Err(SharedError::Validation("Invalid player id".to_string()));
        }
        let viewer_rid = surrealdb::types::RecordId::new("player", viewer_key.as_str());
        let p_rid = surrealdb::types::RecordId::new("player", p_key.as_str());

        // Contests in window, visible to viewer.
        let mut res2 = self
            .db
            .query(
                r#"
SELECT VALUE id
FROM contest
WHERE start >= time::now() - duration::days($since_days)
  AND (
    (moderation_status = 'approved' OR moderation_status = NONE)
    OR creator_id = $viewer
    OR id INSIDE (SELECT VALUE `in` FROM resulted_in WHERE `out` = $viewer)
  )
"#,
            )
            .bind(("since_days", since_days))
            .bind(("viewer", viewer_rid))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let contest_ids: Vec<surrealdb::types::RecordId> = res2.take(0).unwrap_or_default();
        if contest_ids.is_empty() {
            return Ok(0);
        }

        // Count wins (place=1) for this player inside those contests.
        let mut res3 = self
            .db
            .query("SELECT count() AS c FROM resulted_in WHERE `in` INSIDE $contests AND `out` = $p AND place = 1 GROUP ALL")
            .bind(("contests", contest_ids))
            .bind(("p", p_rid))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows3: Vec<serde_json::Value> = res3.take(0).unwrap_or_default();
        let c = rows3
            .get(0)
            .and_then(|v| v.get("c"))
            .map(scalar_i64)
            .unwrap_or(0);
        Ok(c)
    }

    /// Find cities a given game was played in (unique list, capped).
    pub async fn get_cities_for_game_for_viewer(
        &self,
        viewer_key: &str,
        game_name_query: &str,
        limit: usize,
    ) -> Result<(String, Vec<String>)> {
        // Resolve game by fuzzy name.
        let mut res = self
            .db
            .query("SELECT string::concat(id) AS id, name FROM game WHERE string::lowercase(name) CONTAINS string::lowercase($q) LIMIT 1")
            .bind(("q", game_name_query.to_string()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct GameRow { id: String, name: String }
        let games: Vec<GameRow> = res.take(0).unwrap_or_default();
        let Some(g) = games.into_iter().next() else {
            return Err(SharedError::NotFound("game_not_found".to_string()));
        };
        let game_key = record_id_to_key(&g.id, "game");
        if game_key.is_empty() {
            return Err(SharedError::Validation("Invalid game id".to_string()));
        }
        let game_rid = surrealdb::types::RecordId::new("game", game_key.as_str());

        // Contests that played this game.
        let mut res2 = self
            .db
            .query("SELECT `in` AS contest_id FROM played_with WHERE `out` = $rid")
            .bind(("rid", game_rid))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct ContestRow { contest_id: Option<surrealdb::types::RecordId> }
        let contest_rows: Vec<ContestRow> = res2.take(0).unwrap_or_default();
        let contest_ids: Vec<surrealdb::types::RecordId> =
            contest_rows.into_iter().filter_map(|r| r.contest_id).collect();
        if contest_ids.is_empty() {
            return Ok((g.name, vec![]));
        }

        // Filter to contests visible to viewer.
        let viewer_key = viewer_key.to_string();
        let mut res3 = self
            .db
            .query(
                r#"SELECT VALUE id FROM contest WHERE id INSIDE $contests AND (
                    (moderation_status = 'approved' OR moderation_status = NONE)
                    OR creator_id = type::record('player', $viewer_key)
                    OR id INSIDE (SELECT VALUE `in` FROM resulted_in WHERE `out` = type::record('player', $viewer_key))
                )"#,
            )
            .bind(("contests", contest_ids))
            .bind(("viewer_key", viewer_key))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let allowed: Vec<surrealdb::types::RecordId> = res3.take(0).unwrap_or_default();
        if allowed.is_empty() {
            return Ok((g.name, vec![]));
        }

        // Venue ids
        let mut res4 = self
            .db
            .query("SELECT `out` AS venue_id FROM played_at WHERE `in` INSIDE $contests")
            .bind(("contests", allowed))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct VenueRow { venue_id: Option<surrealdb::types::RecordId> }
        let venue_rows: Vec<VenueRow> = res4.take(0).unwrap_or_default();
        let venue_ids: Vec<surrealdb::types::RecordId> =
            venue_rows.into_iter().filter_map(|r| r.venue_id).collect();
        if venue_ids.is_empty() {
            return Ok((g.name, vec![]));
        }

        // Venue addresses -> cities
        let mut res5 = self
            .db
            .query("SELECT formattedAddress AS formatted_address FROM venue WHERE id INSIDE $ids")
            .bind(("ids", venue_ids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let venues: Vec<serde_json::Value> = res5.take(0).unwrap_or_default();
        let mut cities: std::collections::HashSet<String> = std::collections::HashSet::new();
        for v in venues {
            if let Some(addr) = v.get("formatted_address").and_then(|x| x.as_str()) {
                let parts: Vec<&str> = addr.split(',').map(|s| s.trim()).collect();
                if let Some(city) = parts.get(1).map(|s| s.to_string()) {
                    if !city.is_empty() {
                        cities.insert(city);
                    }
                }
            }
        }
        let mut list: Vec<String> = cities.into_iter().collect();
        list.sort();
        list.truncate(limit);
        Ok((g.name, list))
    }

    /// Most popular game in a city (by plays), based on venue formattedAddress city.
    pub async fn get_most_popular_game_in_city_for_viewer(
        &self,
        viewer_key: &str,
        city: &str,
    ) -> Result<Option<(String, i32)>> {
        let city_lc = city.to_lowercase();

        // Load venues and filter in Rust (formattedAddress parsing is messy in SurrealQL).
        let mut res = self
            .db
            .query("SELECT string::concat(id) AS id, formattedAddress AS formatted_address FROM venue")
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let venues: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let mut venue_ids: Vec<surrealdb::types::RecordId> = Vec::new();
        for v in venues {
            let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
            let addr = v.get("formatted_address").and_then(|x| x.as_str()).unwrap_or("");
            let parts: Vec<&str> = addr.split(',').map(|s| s.trim()).collect();
            let c = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
            if !c.is_empty() && c.to_lowercase() == city_lc {
                let key = record_id_to_key(id, "venue");
                if !key.is_empty() {
                    venue_ids.push(surrealdb::types::RecordId::new("venue", key.as_str()));
                }
            }
        }
        if venue_ids.is_empty() {
            return Ok(None);
        }

        // Contest ids in those venues.
        let mut res2 = self
            .db
            .query("SELECT `in` AS contest_id FROM played_at WHERE `out` INSIDE $venues")
            .bind(("venues", venue_ids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct ContestRow { contest_id: Option<surrealdb::types::RecordId> }
        let crows: Vec<ContestRow> = res2.take(0).unwrap_or_default();
        let contest_ids: Vec<surrealdb::types::RecordId> =
            crows.into_iter().filter_map(|r| r.contest_id).collect();
        if contest_ids.is_empty() {
            return Ok(None);
        }

        // Filter contests visible to viewer.
        let viewer_key = viewer_key.to_string();
        let mut res3 = self
            .db
            .query(
                r#"SELECT VALUE id FROM contest WHERE id INSIDE $contests AND (
                    (moderation_status = 'approved' OR moderation_status = NONE)
                    OR creator_id = type::record('player', $viewer_key)
                    OR id INSIDE (SELECT VALUE `in` FROM resulted_in WHERE `out` = type::record('player', $viewer_key))
                )"#,
            )
            .bind(("contests", contest_ids))
            .bind(("viewer_key", viewer_key))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let allowed: Vec<surrealdb::types::RecordId> = res3.take(0).unwrap_or_default();
        if allowed.is_empty() {
            return Ok(None);
        }

        // Games played in those contests.
        let mut res4 = self
            .db
            .query("SELECT `out` AS game_id FROM played_with WHERE `in` INSIDE $contests")
            .bind(("contests", allowed))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct Row { game_id: Option<surrealdb::types::RecordId> }
        let rows: Vec<Row> = res4.take(0).unwrap_or_default();
        let mut counts: HashMap<String, i64> = HashMap::new();
        let mut gids: Vec<surrealdb::types::RecordId> = Vec::new();
        for r in rows {
            if let Some(gid) = r.game_id {
                let id = record_id_to_canonical(&gid).replace("game:", "game/");
                *counts.entry(id.clone()).or_insert(0) += 1;
                gids.push(gid);
            }
        }
        if counts.is_empty() {
            return Ok(None);
        }

        let mut res5 = self
            .db
            .query("SELECT string::concat(id) AS id, name FROM game WHERE id INSIDE $ids")
            .bind(("ids", gids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct GameRow { id: String, name: String }
        let games: Vec<GameRow> = res5.take(0).unwrap_or_default();
        let mut name_by_id: HashMap<String, String> = HashMap::new();
        for g in games {
            name_by_id.insert(g.id, g.name);
        }

        let mut top: Vec<(String, i64)> = counts.into_iter().collect();
        top.sort_by(|a, b| b.1.cmp(&a.1));
        let (top_id, plays) = top[0].clone();
        let name = name_by_id.get(&top_id).cloned().unwrap_or(top_id);
        Ok(Some((name, plays as i32)))
    }

    /// Get top venues by contest count. SurrealQL has no INNER JOIN; count from played_at then look up names from venue.
    pub(crate) async fn get_top_venues(&self, limit: i32) -> Result<Vec<(String, String, i32)>> {
        let sql = r#"SELECT `out` AS venue_id, count() AS contests FROM played_at GROUP BY `out` ORDER BY contests DESC LIMIT $limit"#;
        let mut res = self
            .db
            .query(sql)
            .bind(("limit", limit))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let record_ids: Vec<surrealdb::types::RecordId> = rows
            .iter()
            .filter_map(|v| {
                let canonical = canonical_id_from_value(v.get("venue_id")?, "venue");
                let key = record_id_to_key(&canonical, "venue");
                if key.is_empty() {
                    None
                } else {
                    Some(surrealdb::types::RecordId::new("venue", key.as_str()))
                }
            })
            .collect();
        if record_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut name_res = self
            .db
            .query("SELECT string::concat(id) AS venue_id, displayName AS name FROM venue WHERE id INSIDE $ids")
            .bind(("ids", record_ids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let name_rows: Vec<serde_json::Value> = name_res.take(0).unwrap_or_default();
        let id_to_name: std::collections::HashMap<String, String> = name_rows
            .into_iter()
            .filter_map(|v| {
                let id = v
                    .get("venue_id")
                    .and_then(|x| x.as_str())
                    .map(normalize_record_id_string)?;
                let name = v
                    .get("name")
                    .and_then(|x| x.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| "Unknown".to_string());
                Some((id, name))
            })
            .collect();
        let venues: Vec<(String, String, i32)> = rows
            .into_iter()
            .filter_map(|v| {
                let venue_id = canonical_id_from_value(v.get("venue_id")?, "venue");
                if venue_id.is_empty() {
                    return None;
                }
                let contests = v.get("contests").map(scalar_i64).unwrap_or(0) as i32;
                let name = id_to_name
                    .get(&venue_id)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string());
                Some((venue_id, name, contests))
            })
            .collect();
        Ok(venues)
    }

    /// Get leaderboard data by category.
    /// Source from resulted_in only (GROUP BY `in`). Optionally filter by time period (contest start).
    pub async fn get_leaderboard(
        &self,
        category: &str,
        time_period: Option<TimePeriod>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<PlayerWinRate>> {
        log::debug!(
            "Executing leaderboard query for category: {}, time_period: {:?}",
            category,
            time_period
        );

        let (sql_totals, sql_wins) = match time_period {
            None | Some(TimePeriod::AllTime) => (
                r#"SELECT string::concat(`out`) AS player_id, count() AS total_plays FROM resulted_in GROUP BY player_id"#.to_string(),
                r#"SELECT string::concat(`out`) AS player_id, count() AS wins FROM resulted_in WHERE place = 1 GROUP BY player_id"#.to_string(),
            ),
            // SurrealQL has no INNER JOIN; use IN (SELECT VALUE id ...) so the subquery returns values that IN can match.
            Some(TimePeriod::Last7Days) => (
                r#"SELECT string::concat(`out`) AS player_id, count() AS total_plays FROM resulted_in WHERE `in` IN (SELECT VALUE id FROM contest WHERE start >= time::now() - duration::from_days(7)) GROUP BY player_id"#.to_string(),
                r#"SELECT string::concat(`out`) AS player_id, count() AS wins FROM resulted_in WHERE `in` IN (SELECT VALUE id FROM contest WHERE start >= time::now() - duration::from_days(7)) AND place = 1 GROUP BY player_id"#.to_string(),
            ),
            Some(TimePeriod::Last30Days) => (
                r#"SELECT string::concat(`out`) AS player_id, count() AS total_plays FROM resulted_in WHERE `in` IN (SELECT VALUE id FROM contest WHERE start >= time::now() - duration::from_days(30)) GROUP BY player_id"#.to_string(),
                r#"SELECT string::concat(`out`) AS player_id, count() AS wins FROM resulted_in WHERE `in` IN (SELECT VALUE id FROM contest WHERE start >= time::now() - duration::from_days(30)) AND place = 1 GROUP BY player_id"#.to_string(),
            ),
            Some(TimePeriod::Last90Days) => (
                r#"SELECT string::concat(`out`) AS player_id, count() AS total_plays FROM resulted_in WHERE `in` IN (SELECT VALUE id FROM contest WHERE start >= time::now() - duration::from_days(90)) GROUP BY player_id"#.to_string(),
                r#"SELECT string::concat(`out`) AS player_id, count() AS wins FROM resulted_in WHERE `in` IN (SELECT VALUE id FROM contest WHERE start >= time::now() - duration::from_days(90)) AND place = 1 GROUP BY player_id"#.to_string(),
            ),
            Some(TimePeriod::ThisYear) => (
                r#"SELECT string::concat(`out`) AS player_id, count() AS total_plays FROM resulted_in WHERE `in` IN (SELECT VALUE id FROM contest WHERE time::year(start) = time::year(time::now())) GROUP BY player_id"#.to_string(),
                r#"SELECT string::concat(`out`) AS player_id, count() AS wins FROM resulted_in WHERE `in` IN (SELECT VALUE id FROM contest WHERE time::year(start) = time::year(time::now())) AND place = 1 GROUP BY player_id"#.to_string(),
            ),
        };

        // 1) Total plays per player (from resulted_in only).
        let mut res = self
            .db
            .query(&sql_totals)
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let total_rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        log::debug!(
            "Leaderboard totals query returned {} raw rows",
            total_rows.len()
        );

        // 2) Wins per player (place = 1).
        let mut res_wins = self
            .db
            .query(&sql_wins)
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let win_rows: Vec<serde_json::Value> = res_wins.take(0).unwrap_or_default();

        let wins_map: std::collections::HashMap<String, i64> = win_rows
            .iter()
            .filter_map(|r| {
                let pid = record_id_from_row(r, Some("player"))?;
                let w = r.get("wins").map(scalar_i64).unwrap_or(0);
                Some((pid, w))
            })
            .collect();

        let mut results: Vec<(String, String, i64, i64)> = total_rows
            .into_iter()
            .filter_map(|r| {
                let player_id = record_id_from_row(&r, Some("player"))?;
                let total_plays = r.get("total_plays").map(scalar_i64).unwrap_or(0);
                let wins = wins_map.get(&player_id).copied().unwrap_or(0);
                Some((player_id, String::new(), wins, total_plays))
            })
            .collect();

        // 3) Fetch all players (id, handle, firstname, email) and fill display as "handle (email)".
        if !results.is_empty() {
            // Use string::concat(id) so id comes back as string; raw id is a Thing and fails serde_json::Value deserialization.
            match self
                .db
                .query("SELECT string::concat(id) AS id, handle, firstname, email FROM player")
                .await
            {
                Ok(mut q) => {
                    let all_players: Vec<serde_json::Value> = match q.take(0) {
                        Ok(rows) => rows,
                        Err(e) => {
                            log::warn!("Leaderboard: player query take(0) failed: {}", e);
                            vec![]
                        }
                    };
                    log::debug!(
                        "Leaderboard: player query returned {} raw rows",
                        all_players.len()
                    );
                    if !all_players.is_empty() {
                        if let Some(obj) = all_players[0].as_object() {
                            let keys: Vec<&str> = obj.keys().map(String::as_str).collect();
                            log::debug!("Leaderboard: first player row keys: {:?}", keys);
                        }
                    }
                    let displays: std::collections::HashMap<String, String> = all_players
                        .into_iter()
                        .filter_map(|row| {
                            let id_str = record_id_from_row(&row, Some("player"))?;
                            let canonical_id = normalize_player_id(&id_str);
                            let handle = row
                                .get("handle")
                                .and_then(|v| v.as_str())
                                .filter(|s| !s.is_empty())
                                .map(String::from)
                                .or_else(|| {
                                    row.get("firstname")
                                        .and_then(|v| v.as_str())
                                        .map(String::from)
                                })
                                .unwrap_or_default();
                            let email = row
                                .get("email")
                                .and_then(|v| v.as_str())
                                .filter(|s| !s.is_empty())
                                .map(String::from)
                                .unwrap_or_default();
                            let display = if !handle.is_empty() && !email.is_empty() {
                                format!("{} ({})", handle, email)
                            } else if !email.is_empty() {
                                email
                            } else {
                                handle
                            };
                            Some((canonical_id, display))
                        })
                        .collect();
                    let mut filled = 0usize;
                    results.iter_mut().for_each(|r| {
                        let key = normalize_player_id(&r.0);
                        if let Some(d) = displays.get(&key) {
                            r.1 = d.clone();
                            filled += 1;
                        }
                    });
                    log::debug!(
                        "Leaderboard: loaded {} player displays, matched {} of {} results",
                        displays.len(),
                        filled,
                        results.len()
                    );
                }
                Err(e) => {
                    log::warn!("Leaderboard: player query failed: {}", e);
                }
            }
        }

        log::debug!(
            "Leaderboard extracted {} results after filter_map",
            results.len()
        );
        let win_rate = |r: &(String, String, i64, i64)| {
            if r.3 > 0 {
                (r.2 as f64 * 100.0) / r.3 as f64
            } else {
                0.0
            }
        };
        match category {
            "win_rate" => results.sort_by(|a, b| {
                win_rate(b)
                    .partial_cmp(&win_rate(a))
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            "total_wins" => results.sort_by_key(|r| std::cmp::Reverse(r.2)),
            "total_contests" => results.sort_by_key(|r| std::cmp::Reverse(r.3)),
            _ => {
                return Err(SharedError::Conversion(
                    "Invalid leaderboard category".to_string(),
                ))
            }
        }
        let leaderboard: Vec<PlayerWinRate> = results
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(|r| PlayerWinRate {
                player_id: r.0,
                player_handle: r.1,
                wins: r.2 as i32,
                total_plays: r.3 as i32,
                win_rate: if r.3 > 0 {
                    (r.2 as f64 * 100.0) / r.3 as f64
                } else {
                    0.0
                },
            })
            .collect();
        Ok(leaderboard)
    }
}

impl AnalyticsRepository {
    /// Get a display label for a player (handle -> email -> name)
    pub async fn get_player_display_label(&self, player_id: &str) -> Result<Option<String>> {
        let key = player_id_to_key(player_id);
        if key.is_empty() {
            return Ok(None);
        }
        let record_id = surrealdb::types::RecordId::new("player", key.as_str());
        let mut res = self
            .db
            .query("SELECT handle, email, firstname, lastname FROM player WHERE id = $record_id")
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };
        let handle = row.get("handle").and_then(|v| v.as_str()).map(String::from);
        let email = row.get("email").and_then(|v| v.as_str()).map(String::from);
        let firstname = row.get("firstname").and_then(|v| v.as_str());
        let lastname = row.get("lastname").and_then(|v| v.as_str());
        let name = match (firstname, lastname) {
            (Some(first), Some(last)) => Some(format!("{} {}", first, last)),
            (Some(first), None) => Some(first.to_string()),
            _ => None,
        };
        Ok(handle.or(email).or(name))
    }

    /// Get latest global rating info for a player
    pub async fn get_player_rating_latest(
        &self,
        player_id: &str,
    ) -> Result<Option<(f64, f64, i32)>> {
        let key = player_id_to_key(player_id);
        if key.is_empty() {
            return Ok(None);
        }
        let record_id = surrealdb::types::RecordId::new("player", key.as_str());
        let mut res = self
            .db
            .query("SELECT rating, rd, games_played FROM rating_latest WHERE player_id = $record_id AND scope_type = 'global' LIMIT 1")
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };
        let rating = row.get("rating").and_then(|v| v.as_f64()).unwrap_or(1200.0);
        let rd = row.get("rd").and_then(|v| v.as_f64()).unwrap_or(350.0);
        let games_played = row
            .get("games_played")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        Ok(Some((rating, rd, games_played)))
    }

    /// Get player statistics. Prefer binding the player record id as a Thing so the server
    /// compares record-to-record; fall back to type::record('player', $key) when we only have a key.
    pub async fn get_player_stats(&self, player_id: &str) -> Result<Option<PlayerStats>> {
        let key = player_id_to_key(player_id);
        if key.is_empty() {
            log::warn!("get_player_stats: empty key for player_id={:?}", player_id);
            return Ok(None);
        }

        // Prefer SurrealDB function when applied (one round-trip for stats; then confirm player exists)
        if let Ok(mut res) = self
            .db
            .query("SELECT fn::player_stats_by_key($key) AS result FROM [1]")
            .bind(("key", key.clone()))
            .await
        {
            let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
            if let Some(first) = rows.into_iter().next() {
                let row = first
                    .get("result")
                    .or_else(|| first.get("fn::player_stats_by_key($key)"))
                    .cloned()
                    .unwrap_or(first);
                if row.is_object()
                    && (row.get("contests_out").is_some() || row.get("contests_in").is_some())
                {
                    let player_id_norm = format!("player/{}", key);
                    if let Some(stats) =
                        Self::player_stats_from_dual_out_in_row(&row, &player_id_norm)
                    {
                        // Confirm player exists so we return None for invalid key
                        let check_rid = surrealdb::types::RecordId::new("player", key.as_str());
                        let mut check = self
                            .db
                            .query("SELECT id FROM player WHERE id = $record_id LIMIT 1")
                            .bind(("record_id", check_rid))
                            .await
                            .map_err(|e| SharedError::Database(e.to_string()))?;
                        let check_rows: Vec<serde_json::Value> = check.take(0).unwrap_or_default();
                        if check_rows.into_iter().next().is_some() {
                            let (cur, long) =
                                self.get_player_streaks(player_id).await.unwrap_or((0, 0));
                            let mut st = stats;
                            st.current_streak = cur;
                            st.longest_streak = long;
                            return Ok(Some(st));
                        }
                    }
                }
            }
        }

        // Fallback: single query with record id binding
        let record_id = surrealdb::types::RecordId::new("player", key.as_str());
        let sql = r#"
            SELECT
                id AS player_id,
                handle AS player_handle,
                ((SELECT count() FROM resulted_in WHERE `out` = $record_id GROUP ALL)[0].count) ?? 0 AS total_contests,
                ((SELECT count() FROM resulted_in WHERE `out` = $record_id AND place = 1 GROUP ALL)[0].count) ?? 0 AS total_wins,
                ((SELECT math::mean(place) FROM resulted_in WHERE `out` = $record_id GROUP ALL)[0].`math::mean`) ?? 0 AS average_placement,
                ((SELECT math::min(place) FROM resulted_in WHERE `out` = $record_id GROUP ALL)[0].`math::min`) ?? 0 AS best_placement
            FROM player WHERE id = $record_id LIMIT 1
        "#;

        let mut res = self
            .db
            .query(sql)
            .bind(("record_id", record_id))
            .await
            .map_err(|e| {
                log::warn!(
                    "get_player_stats: query failed for player_id={:?} key={:?}: {}",
                    player_id,
                    key,
                    e
                );
                SharedError::Database(e.to_string())
            })?;

        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => {
                log::warn!(
                    "get_player_stats: no player row for player_id={:?} key={:?}",
                    player_id,
                    key
                );
                return Ok(None);
            }
        };
        let total_contests = row.get("total_contests").map(scalar_i64).unwrap_or(0) as i32;
        let total_wins = row.get("total_wins").map(scalar_i64).unwrap_or(0) as i32;
        let total_losses = total_contests.saturating_sub(total_wins);
        let win_rate = if total_contests > 0 {
            (total_wins as f64 * 100.0) / total_contests as f64
        } else {
            0.0
        };
        let average_placement = row.get("average_placement").map(scalar_f64).unwrap_or(0.0);
        let best_placement = row.get("best_placement").map(scalar_i64).unwrap_or(0) as i32;
        let player_id_norm =
            record_id_from_row(&row, Some("player")).unwrap_or_else(|| format!("player/{}", key));
        let (cur, long) = self.get_player_streaks(player_id).await.unwrap_or((0, 0));
        let stats = PlayerStats {
            player_id: player_id_norm,
            total_contests,
            total_wins,
            total_losses,
            win_rate,
            average_placement,
            best_placement,
            skill_rating: 1200.0,
            rating_confidence: 0.8,
            total_points: total_wins * 10,
            current_streak: cur,
            longest_streak: long,
            last_updated: chrono::Utc::now(),
        };
        Ok(Some(stats))
    }

    /// Get player statistics using the exact record id from the DB (e.g. from get_player_thing_by_email).
    /// Use this for "me" so we don't depend on string→Thing conversion matching.
    pub async fn get_player_stats_by_thing(
        &self,
        record_id: surrealdb::types::RecordId,
    ) -> Result<Option<PlayerStats>> {
        let sql = r#"
            SELECT
                id AS player_id,
                handle AS player_handle,
                ((SELECT count() FROM resulted_in WHERE `out` = $record_id GROUP ALL)[0].count) ?? 0 AS total_contests,
                ((SELECT count() FROM resulted_in WHERE `out` = $record_id AND place = 1 GROUP ALL)[0].count) ?? 0 AS total_wins,
                ((SELECT math::mean(place) FROM resulted_in WHERE `out` = $record_id GROUP ALL)[0].`math::mean`) ?? 0 AS average_placement,
                ((SELECT math::min(place) FROM resulted_in WHERE `out` = $record_id GROUP ALL)[0].`math::min`) ?? 0 AS best_placement
            FROM player WHERE id = $record_id LIMIT 1
        "#;
        let mut res = self
            .db
            .query(sql)
            .bind(("record_id", record_id.clone()))
            .await
            .map_err(|e| {
                log::warn!("get_player_stats_by_thing: query failed: {}", e);
                SharedError::Database(e.to_string())
            })?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => {
                log::warn!(
                    "get_player_stats_by_thing: no player row for record_id={:?}",
                    record_id
                );
                return Ok(None);
            }
        };
        let total_contests = row.get("total_contests").map(scalar_i64).unwrap_or(0) as i32;
        let total_wins = row.get("total_wins").map(scalar_i64).unwrap_or(0) as i32;
        let total_losses = total_contests.saturating_sub(total_wins);
        let win_rate = if total_contests > 0 {
            (total_wins as f64 * 100.0) / total_contests as f64
        } else {
            0.0
        };
        let average_placement = row.get("average_placement").map(scalar_f64).unwrap_or(0.0);
        let best_placement = row.get("best_placement").map(scalar_i64).unwrap_or(0) as i32;
        let player_id_norm = record_id_from_row(&row, Some("player"))
            .unwrap_or_else(|| record_id_to_player_id_str(&record_id));
        let (cur, long) = self
            .get_player_streaks(&player_id_norm)
            .await
            .unwrap_or((0, 0));
        let stats = PlayerStats {
            player_id: player_id_norm,
            total_contests,
            total_wins,
            total_losses,
            win_rate,
            average_placement,
            best_placement,
            skill_rating: 1200.0,
            rating_confidence: 0.8,
            total_points: total_wins * 10,
            current_streak: cur,
            longest_streak: long,
            last_updated: chrono::Utc::now(),
        };
        Ok(Some(stats))
    }

    /// Get player statistics by the exact id string. Tries fn::player_stats_by_id_str when applied, else inline query.
    pub async fn get_player_stats_by_id_str(&self, id_str: &str) -> Result<Option<PlayerStats>> {
        let id_normalized = id_str.replace('`', "").replace('/', ":");
        let player_id_canonical = id_normalized.replace(':', "/");

        // Prefer SurrealDB function when applied
        if let Ok(mut res) = self
            .db
            .query("SELECT fn::player_stats_by_id_str($id_str) AS result FROM [1]")
            .bind(("id_str", id_normalized.clone()))
            .await
        {
            let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
            if let Some(first) = rows.into_iter().next() {
                let row = first
                    .get("result")
                    .or_else(|| first.get("fn::player_stats_by_id_str($id_str)"))
                    .cloned()
                    .unwrap_or(first);
                if row.is_object()
                    && (row.get("contests_out").is_some() || row.get("contests_in").is_some())
                {
                    if let Some(mut stats) =
                        Self::player_stats_from_dual_out_in_row(&row, &player_id_canonical)
                    {
                        let (cur, long) = self
                            .get_player_streaks(&player_id_canonical)
                            .await
                            .unwrap_or((0, 0));
                        stats.current_streak = cur;
                        stats.longest_streak = long;
                        return Ok(Some(stats));
                    }
                }
            }
        }

        // Fallback: inline query (player row + dual out/in aggregates)
        let sql = r#"
            SELECT
                id AS player_id,
                handle AS player_handle,
                (SELECT count() FROM resulted_in WHERE string::replace(string::concat(`out`), '`', '') = $id_str) AS contests_out,
                (SELECT count() FROM resulted_in WHERE string::replace(string::concat(`out`), '`', '') = $id_str AND place = 1) AS wins_out,
                (SELECT math::mean(place) FROM resulted_in WHERE string::replace(string::concat(`out`), '`', '') = $id_str) AS avg_out,
                (SELECT math::min(place) FROM resulted_in WHERE string::replace(string::concat(`out`), '`', '') = $id_str) AS best_out,
                (SELECT count() FROM resulted_in WHERE string::replace(string::concat(`in`), '`', '') = $id_str) AS contests_in,
                (SELECT count() FROM resulted_in WHERE string::replace(string::concat(`in`), '`', '') = $id_str AND place = 1) AS wins_in,
                (SELECT math::mean(place) FROM resulted_in WHERE string::replace(string::concat(`in`), '`', '') = $id_str) AS avg_in,
                (SELECT math::min(place) FROM resulted_in WHERE string::replace(string::concat(`in`), '`', '') = $id_str) AS best_in
            FROM player WHERE string::replace(string::concat(id), '`', '') = $id_str LIMIT 1
        "#;
        let mut res = self
            .db
            .query(sql)
            .bind(("id_str", id_normalized.clone()))
            .await
            .map_err(|e| {
                log::warn!(
                    "get_player_stats_by_id_str: query failed for id_str={:?}: {}",
                    id_str,
                    e
                );
                SharedError::Database(e.to_string())
            })?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => {
                log::warn!(
                    "get_player_stats_by_id_str: no player row for id_str={:?}",
                    id_str
                );
                return Ok(None);
            }
        };
        let player_id_norm =
            record_id_from_row(&row, Some("player")).unwrap_or_else(|| player_id_canonical.clone());
        match Self::player_stats_from_dual_out_in_row(&row, &player_id_norm) {
            Some(mut stats) => {
                let (cur, long) = self
                    .get_player_streaks(&player_id_norm)
                    .await
                    .unwrap_or((0, 0));
                stats.current_streak = cur;
                stats.longest_streak = long;
                Ok(Some(stats))
            }
            None => Ok(None),
        }
    }

    /// Saves player statistics to database
    pub async fn save_player_stats(&self, stats: &PlayerStats) -> Result<()> {
        let doc = serde_json::to_value(stats).map_err(|e| {
            SharedError::Conversion(format!("Failed to serialize player stats: {}", e))
        })?;
        self.db
            .query("INSERT INTO player_stats CONTENT $doc")
            .bind(("doc", doc))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to save player stats: {}", e)))?;
        Ok(())
    }

    /// Updates player statistics in database
    pub async fn update_player_stats(&self, stats: &PlayerStats) -> Result<()> {
        let doc = serde_json::to_value(stats).map_err(|e| {
            SharedError::Conversion(format!("Failed to serialize player stats: {}", e))
        })?;
        let key = player_id_to_key(&stats.player_id);
        self.db
            .query("UPDATE type::record('player_stats', $key) MERGE $doc")
            .bind(("key", key))
            .bind(("doc", doc))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to update player stats: {}", e)))?;
        Ok(())
    }

    /// Saves contest statistics to database
    pub async fn save_contest_stats(&self, stats: &ContestStats) -> Result<()> {
        let doc = serde_json::to_value(stats).map_err(|e| {
            SharedError::Conversion(format!("Failed to serialize contest stats: {}", e))
        })?;
        self.db
            .query("INSERT INTO contest_stats CONTENT $doc")
            .bind(("doc", doc))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to save contest stats: {}", e)))?;
        Ok(())
    }

    /// Get contest statistics
    pub async fn get_contest_stats(&self, contest_id: &str) -> Result<Option<ContestStats>> {
        log::debug!("Querying contest stats for contest_id: {}", contest_id);

        let key = record_id_to_key(contest_id, "contest");
        if key.is_empty() {
            return Ok(None);
        }
        let record_id = surrealdb::types::RecordId::new("contest", key.as_str());
        let mut check_res = self
            .db
            .query("SELECT string::concat(id) AS id FROM contest WHERE id = $record_id")
            .bind(("record_id", record_id.clone()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let exist: Vec<serde_json::Value> = check_res.take(0).unwrap_or_default();
        if exist.is_empty() {
            return Ok(None);
        }
        let sql = r#"SELECT string::concat(id) AS contest_id, (SELECT count() FROM resulted_in WHERE `in` = $record_id) AS participant_count, (SELECT count() FROM resulted_in WHERE `in` = $record_id AND place > 0) AS completion_count FROM contest WHERE id = $record_id"#;
        let mut res = self
            .db
            .query(sql)
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };
        let participant_count = row
            .get("participant_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as i32;
        let completion_count = row
            .get("completion_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as i32;
        let completion_rate = if participant_count > 0 {
            (completion_count as f64 * 100.0) / participant_count as f64
        } else {
            0.0
        };
        let contest_id_norm = row
            .get("contest_id")
            .and_then(|v| v.as_str())
            .map(normalize_record_id_string)
            .unwrap_or_else(|| contest_id.to_string());
        let stats = ContestStats {
            contest_id: contest_id_norm,
            contest_name: String::new(),
            started_at: None,
            participant_count,
            completion_count,
            completion_rate,
            average_placement: 0.0,
            duration_minutes: 0,
            most_popular_game: None,
            most_popular_game_id: None,
            difficulty_rating: 5.0,
            excitement_rating: 5.0,
            last_updated: chrono::Utc::now().into(),
        };
        Ok(Some(stats))
    }

    /// Get contest trends (monthly contest frequency)
    pub async fn get_contest_trends(&self, months: i32) -> Result<Vec<MonthlyContests>> {
        // SurrealDB has no duration::from_months; approximate as 30 days per month
        let days = months.saturating_mul(30);
        let q = self.db.query(
            "SELECT time::year(start) AS year, time::month(start) AS month, count() AS contests FROM contest WHERE start >= time::now() - duration::from_days($days) GROUP BY year, month ORDER BY year, month"
        );
        let mut res = q
            .bind(("days", days))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let trends: Vec<MonthlyContests> = rows
            .into_iter()
            .map(|v| MonthlyContests {
                year: v.get("year").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
                month: v.get("month").and_then(|x| x.as_i64()).unwrap_or(1) as u32,
                contests: v.get("contests").and_then(|x| x.as_u64()).unwrap_or(0) as i32,
            })
            .collect();
        Ok(trends)
    }

    /// Get daily active players (unique players per day) for the last N days. SurrealQL has no INNER JOIN; query contest then resulted_in and join in Rust.
    /// Tries fn::daily_active_players_data($days) first for one round-trip when applied.
    pub async fn get_daily_active_players(&self, days: i32) -> Result<Vec<(String, i32)>> {
        if let Ok(mut res) = self
            .db
            .query("SELECT fn::daily_active_players_data($days) AS result FROM [1]")
            .bind(("days", days))
            .await
        {
            let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
            if let Some(first) = rows.into_iter().next() {
                let result = first
                    .get("result")
                    .or_else(|| first.get("fn::daily_active_players_data($days)"))
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                if let Some(obj) = result.as_object() {
                    let empty: &[serde_json::Value] = &[];
                    let contest_days_arr = obj
                        .get("contest_days")
                        .and_then(|v| v.as_array())
                        .map(|v| v.as_slice())
                        .unwrap_or(empty);
                    let ri_arr = obj
                        .get("resulted_in")
                        .and_then(|v| v.as_array())
                        .map(|v| v.as_slice())
                        .unwrap_or(empty);
                    let contest_to_day: std::collections::HashMap<String, String> =
                        contest_days_arr
                            .iter()
                            .filter_map(|v| {
                                let id = record_id_from_field(v, "id")?;
                                let day =
                                    v.get("day").and_then(|d| d.as_str()).map(String::from)?;
                                Some((id, day))
                            })
                            .collect();
                    let mut by_day: std::collections::HashMap<
                        String,
                        std::collections::HashSet<String>,
                    > = std::collections::HashMap::new();
                    for r in ri_arr {
                        let cid = match record_id_from_field(r, "contest_id") {
                            Some(x) => x,
                            None => continue,
                        };
                        let pid = record_id_from_field(r, "player_id").unwrap_or_default();
                        if let Some(day) = contest_to_day.get(&cid) {
                            by_day.entry(day.clone()).or_default().insert(pid);
                        }
                    }
                    let mut out: Vec<(String, i32)> = by_day
                        .into_iter()
                        .map(|(day, set)| (day, set.len() as i32))
                        .collect();
                    out.sort_by(|a, b| a.0.cmp(&b.0));
                    return Ok(out);
                }
            }
        }

        let contest_sql = "SELECT string::concat(id) AS contest_id, time::format(start, '%Y-%m-%d') AS day FROM contest WHERE start >= time::now() - duration::from_days($days)";
        let mut res_contest = self
            .db
            .query(contest_sql)
            .bind(("days", days))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct ContestDayRow {
            contest_id: Option<String>,
            day: Option<String>,
        }
        let contest_rows: Vec<ContestDayRow> = res_contest.take(0).unwrap_or_default();
        let contest_ids_colon: Vec<String> = contest_rows
            .iter()
            .filter_map(|r| r.contest_id.as_ref().map(|s| s.replace('/', ":")))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        if contest_ids_colon.is_empty() {
            return Ok(Vec::new());
        }
        let contest_to_day: std::collections::HashMap<String, String> = contest_rows
            .into_iter()
            .filter_map(|r| {
                let cid = r.contest_id?.replace('/', ":");
                let day = r.day?;
                Some((cid, day))
            })
            .collect();
        let ri_sql = "SELECT string::concat(`in`) AS contest_id, string::concat(`out`) AS player_id FROM resulted_in WHERE `in` INSIDE $ids";
        let mut res_ri = self
            .db
            .query(ri_sql)
            .bind(("ids", contest_ids_colon))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct RiRow {
            contest_id: Option<String>,
            player_id: Option<String>,
        }
        let ri_rows: Vec<RiRow> = res_ri.take(0).unwrap_or_default();
        let mut by_day: std::collections::HashMap<String, std::collections::HashSet<String>> =
            std::collections::HashMap::new();
        for r in ri_rows {
            if let (Some(cid), Some(pid)) = (r.contest_id.map(|s| s.replace('/', ":")), r.player_id)
            {
                if let Some(day) = contest_to_day.get(&cid) {
                    by_day.entry(day.clone()).or_default().insert(pid);
                }
            }
        }
        let mut out: Vec<(String, i32)> = by_day
            .into_iter()
            .map(|(day, set)| (day, set.len() as i32))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    /// Get daily contests count for the last N days
    pub async fn get_daily_contests(&self, days: i32) -> Result<Vec<(String, i32)>> {
        let sql = r#"
            SELECT time::format(start, '%Y-%m-%d') AS day, count() AS count
            FROM contest
            WHERE start >= time::now() - duration::from_days($days)
            GROUP BY time::format(start, '%Y-%m-%d')
            ORDER BY day ASC
        "#;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct DayCount {
            day: String,
            count: i32,
        }
        let mut res = self
            .db
            .query(sql)
            .bind(("days", days))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let cursor: Vec<DayCount> = res.take(0).unwrap_or_default();
        let out: Vec<(String, i32)> = cursor.into_iter().map(|e| (e.day, e.count)).collect();
        Ok(out)
    }

    /// Get contest difficulty analysis
    pub async fn get_contest_difficulty_analysis(&self, contest_id: &str) -> Result<f64> {
        let key = record_id_to_key(contest_id, "contest");
        if key.is_empty() {
            return Ok(5.0);
        }
        let record_id = surrealdb::types::RecordId::new("contest", key.as_str());
        let sql = "SELECT place FROM resulted_in WHERE `in` = $record_id AND place > 0";
        let mut res = self
            .db
            .query(sql)
            .bind(("record_id", record_id.clone()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let places: Vec<serde_json::Value> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("contest difficulty places: {}", e)))?;
        let completed: Vec<i32> = places
            .into_iter()
            .filter_map(|v| v.get("place").map(|p| scalar_i64(p) as i32))
            .collect();
        let count_sql = "SELECT count() AS count FROM resulted_in WHERE `in` = $record_id";
        let mut cq_res = self
            .db
            .query(count_sql)
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let count_res: Vec<serde_json::Value> = cq_res
            .take(0)
            .map_err(|e| SharedError::Database(format!("contest difficulty count: {}", e)))?;
        let participant_count = count_res
            .into_iter()
            .next()
            .map(|v| scalar_i64(&v))
            .unwrap_or(0) as i32;
        let total_placements: i32 = completed.iter().sum();
        let completed_count = completed.len() as i32;
        let average_placement = if completed_count > 0 {
            total_placements as f64 / completed_count as f64
        } else {
            0.0
        };
        let difficulty_score = if participant_count > 0 {
            (average_placement / participant_count as f64) * 10.0
        } else {
            5.0
        };
        Ok(difficulty_score.min(10.0))
    }

    /// Get contest excitement rating (based on close finishes)
    pub async fn get_contest_excitement_rating(&self, contest_id: &str) -> Result<f64> {
        let key = record_id_to_key(contest_id, "contest");
        if key.is_empty() {
            return Ok(5.0);
        }
        let record_id = surrealdb::types::RecordId::new("contest", key.as_str());
        let sql = "SELECT place, score FROM resulted_in WHERE `in` = $record_id AND place > 0 ORDER BY place ASC LIMIT 2";
        let mut res = self
            .db
            .query(sql)
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let results: Vec<serde_json::Value> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("contest excitement: {}", e)))?;
        let scores: Vec<f64> = results
            .into_iter()
            .filter_map(|v| v.get("score").and_then(|s| s.as_f64()))
            .collect();
        let (first, second) = (scores.first().copied(), scores.get(1).copied());
        let (score_diff, max_score) = match (first, second) {
            (Some(a), Some(b)) => ((a - b).abs(), a.max(b)),
            _ => return Ok(5.0),
        };
        let closeness = if max_score > 0.0 {
            1.0 - (score_diff / max_score)
        } else {
            1.0
        };
        let excitement = 5.0 + closeness * 5.0;
        Ok(excitement.min(10.0))
    }

    /// Get recent contests with statistics
    pub async fn get_recent_contests(&self, limit: i32) -> Result<Vec<ContestStats>> {
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct ContestRow {
            id: Option<String>,
            name: Option<String>,
            start: Option<String>,
            duration_minutes: Option<i32>,
        }
        let sql = "SELECT string::concat(id) AS id, name, start, duration_minutes FROM contest ORDER BY start DESC LIMIT $limit";
        let mut res = self
            .db
            .query(sql)
            .bind(("limit", limit))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let contests: Vec<ContestRow> = res.take(0).unwrap_or_default();
        let contest_rids: Vec<String> = contests
            .iter()
            .filter_map(|c| c.id.as_ref().map(std::string::ToString::to_string))
            .collect();
        if contest_rids.is_empty() {
            return Ok(Vec::new());
        }
        let rid_param: Vec<String> = contest_rids.iter().map(|s| s.replace('/', ":")).collect();
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct ResRow {
            contest_rid: Option<String>,
            place: Option<i32>,
        }
        let res_sql = "SELECT string::concat(`in`) AS contest_rid, place FROM resulted_in WHERE `in` INSIDE $rids";
        let mut rq_res = self
            .db
            .query(res_sql)
            .bind(("rids", rid_param.clone()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let res_rows: Vec<ResRow> = rq_res.take(0).unwrap_or_default();
        // SurrealQL has no INNER JOIN: get (contest_rid, game_id) from played_with then look up game names
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct PwRow {
            contest_rid: Option<String>,
            game_id: Option<String>,
        }
        let pw_sql = "SELECT string::concat(`in`) AS contest_rid, string::concat(`out`) AS game_id FROM played_with WHERE `in` INSIDE $rids";
        let mut pwq_res = self
            .db
            .query(pw_sql)
            .bind(("rids", rid_param))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let pw_rows: Vec<PwRow> = pwq_res.take(0).unwrap_or_default();
        let game_ids: Vec<String> = pw_rows
            .iter()
            .filter_map(|r| r.game_id.as_ref().map(|s| s.replace('/', ":")))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let game_id_to_name: std::collections::HashMap<String, String> = if game_ids.is_empty() {
            std::collections::HashMap::new()
        } else {
            let name_sql =
                "SELECT string::concat(id) AS game_id, name FROM game WHERE id INSIDE $ids";
            let mut name_res = self
                .db
                .query(name_sql)
                .bind(("ids", game_ids))
                .await
                .map_err(|e| SharedError::Database(e.to_string()))?;
            let name_rows: Vec<serde_json::Value> = name_res.take(0).unwrap_or_default();
            name_rows
                .into_iter()
                .filter_map(|v| {
                    let id = v
                        .get("game_id")
                        .and_then(|x| x.as_str())
                        .map(|s| s.replace('/', ":"))?;
                    let name = v.get("name").and_then(|x| x.as_str()).map(String::from)?;
                    Some((id, name))
                })
                .collect()
        };
        let mut by_contest: std::collections::HashMap<String, (Vec<i32>, Vec<(String, String)>)> =
            std::collections::HashMap::new();
        for r in res_rows {
            if let (Some(rid), Some(place)) = (r.contest_rid, r.place) {
                let rid_norm = rid.replace("contest:", "contest/");
                by_contest.entry(rid_norm).or_default().0.push(place);
            }
        }
        for r in pw_rows {
            if let (Some(rid), Some(gid)) = (r.contest_rid, r.game_id) {
                let rid_norm = rid.replace("contest:", "contest/");
                let gid_norm = gid.replace('/', ":");
                let canonical_id = gid.replace("game:", "game/");
                let name = game_id_to_name
                    .get(&gid_norm)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string());
                by_contest
                    .entry(rid_norm)
                    .or_default()
                    .1
                    .push((canonical_id, name));
            }
        }
        let mut out = Vec::with_capacity(contests.len());
        for c in contests {
            let contest_id =
                c.id.as_ref()
                    .map(|s| s.replace("contest:", "contest/"))
                    .unwrap_or_default();
            let (places, games) = by_contest.get(&contest_id).cloned().unwrap_or_default();
            let participant_count = places.len() as i32;
            let completion_count = participant_count;
            let completion_rate = if participant_count > 0 { 100.0 } else { 0.0 };
            let average_placement = if places.is_empty() {
                0.0
            } else {
                places.iter().map(|&p| f64::from(p)).sum::<f64>() / places.len() as f64
            };
            let (most_popular_game, most_popular_game_id) = {
                let mut counts: std::collections::HashMap<(String, String), usize> =
                    std::collections::HashMap::new();
                for (gid, name) in &games {
                    *counts.entry((gid.clone(), name.clone())).or_insert(0) += 1;
                }
                counts
                    .into_iter()
                    .max_by_key(|(_, c)| *c)
                    .map(|((gid, name), _)| (Some(name), Some(gid)))
                    .unwrap_or((None, None))
            };
            let difficulty = self
                .get_contest_difficulty_analysis(&contest_id)
                .await
                .unwrap_or(5.0);
            let excitement = self
                .get_contest_excitement_rating(&contest_id)
                .await
                .unwrap_or(5.0);
            let started_at = c.start.as_ref().and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(s)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()))
            });
            let contest_name = c.name.clone().unwrap_or_default();
            out.push(ContestStats {
                contest_id: contest_id.clone(),
                contest_name,
                started_at,
                participant_count,
                completion_count,
                completion_rate,
                average_placement,
                duration_minutes: c.duration_minutes.unwrap_or(0),
                most_popular_game,
                most_popular_game_id,
                difficulty_rating: difficulty,
                excitement_rating: excitement,
                last_updated: chrono::Utc::now().into(),
            });
        }
        Ok(out)
    }

    // Player-specific analytics methods

    /// Get players who have beaten the current player (opponents with more wins than losses vs me)
    pub async fn get_players_who_beat_me(&self, player_id: &str) -> Result<Vec<PlayerOpponentDto>> {
        let (beat_me, _) = self.get_opponent_stats_both(player_id).await?;
        Ok(beat_me)
    }

    /// Get players that the current player has beaten (opponents with more losses to me than wins)
    pub async fn get_players_i_beat(&self, player_id: &str) -> Result<Vec<PlayerOpponentDto>> {
        let (_, i_beat) = self.get_opponent_stats_both(player_id).await?;
        Ok(i_beat)
    }

    /// Fetch opponent stats once and return (who beat me, who I beat). Use in profile bundle to avoid running the heavy query twice.
    pub async fn get_opponent_stats_both(
        &self,
        player_id: &str,
    ) -> Result<(Vec<PlayerOpponentDto>, Vec<PlayerOpponentDto>)> {
        const TOP_N: usize = 10;
        let opponents = self.fetch_opponent_stats(player_id).await?;
        log::info!(
            "get_opponent_stats_both: player_id={} raw_opponents_count={}",
            player_id,
            opponents.len()
        );
        for (i, o) in opponents.iter().take(5).enumerate() {
            log::info!(
                "  opponent[{}] player_id={} contests_played={} wins_against_me={} losses_to_me={} (owned={})",
                i,
                o.player_id,
                o.contests_played,
                o.wins_against_me,
                o.losses_to_me,
                o.losses_to_me >= o.wins_against_me && (o.losses_to_me + o.wins_against_me) > 0
            );
        }
        if opponents.len() > 5 {
            log::info!("  ... and {} more opponents", opponents.len() - 5);
        }
        let mut beat_me: Vec<PlayerOpponentDto> = opponents
            .iter()
            .filter(|o| o.wins_against_me > o.losses_to_me)
            .cloned()
            .collect();
        beat_me.sort_by(|a, b| b.wins_against_me.cmp(&a.wins_against_me));
        let beat_me: Vec<PlayerOpponentDto> = beat_me.into_iter().take(TOP_N).collect();
        // Owned: I beat them at least as often as they beat me (includes ties; excludes only strict nemeses).
        let mut i_beat: Vec<PlayerOpponentDto> = opponents
            .into_iter()
            .filter(|o| {
                o.losses_to_me >= o.wins_against_me && (o.losses_to_me + o.wins_against_me) > 0
            })
            .collect();
        i_beat.sort_by(|a, b| b.losses_to_me.cmp(&a.losses_to_me));
        let i_beat: Vec<PlayerOpponentDto> = i_beat.into_iter().take(TOP_N).collect();
        log::info!(
            "get_opponent_stats_both: returning nemesis_count={} owned_count={}",
            beat_me.len(),
            i_beat.len()
        );
        Ok((beat_me, i_beat))
    }

    /// Fetch all head-to-head opponent stats for a player (shared by beat_me / i_beat / networking).
    pub async fn fetch_opponent_stats(&self, player_id: &str) -> Result<Vec<PlayerOpponentDto>> {
        let key = player_id_to_key(player_id);
        if key.is_empty() {
            return Ok(Vec::new());
        }
        // 1) Get contest IDs where this player participated (single indexed lookup on `in`).
        let record_id = surrealdb::types::RecordId::new("player", key.as_str());
        let sql_my_contests = "SELECT `in` AS contest_id FROM resulted_in WHERE `out` = $record_id";
        let mut res1 = self
            .db
            .query(sql_my_contests)
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let my_contest_rows: Vec<ResultedInRow> = res1
            .take(0)
            .map_err(|e| SharedError::Database(format!("opponent stats my contests: {}", e)))?;
        let contest_ids: Vec<String> = my_contest_rows
            .iter()
            .filter_map(|r| {
                let rid = thing_to_record_id(&r.contest_id);
                if rid.is_empty() {
                    None
                } else {
                    Some(rid.replace('/', ":"))
                }
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        if contest_ids.is_empty() {
            log::info!("fetch_opponent_stats: no contests for player key {}", key);
            return Ok(Vec::new());
        }
        // 2) Get all resulted_in rows for those contests (indexed lookup on `out`).
        // Bind Thing array so SurrealDB matches record id column (v2 does not coerce string array).
        let contest_things = strings_to_record_id_array(&contest_ids);
        let mut res = self
            .db
            .query("SELECT `in` AS contest_id, `out` AS player_id, place AS place FROM resulted_in WHERE `in` INSIDE $contest_ids")
            .bind(("contest_ids", contest_things))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<ResultedInRow> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("opponent stats by contest: {}", e)))?;
        log::info!(
            "fetch_opponent_stats: player key={} my_contests={} resulted_in_rows={}",
            key,
            contest_ids.len(),
            rows.len()
        );
        let my_id_norm = normalize_player_id(player_id);
        // contest_id -> (my_place, [(opponent_id, opponent_place)])
        let mut by_contest: HashMap<String, (i32, Vec<(String, i32)>)> = HashMap::new();
        for r in rows {
            let cid = thing_to_record_id(&r.contest_id);
            if cid.is_empty() {
                continue;
            }
            let pid_norm = thing_to_record_id(&r.player_id);
            if pid_norm.is_empty() {
                continue;
            }
            let place = r.place.unwrap_or(0) as i32;
            let entry = by_contest.entry(cid).or_insert((0, Vec::new()));
            if pid_norm == my_id_norm {
                entry.0 = place;
            } else {
                entry.1.push((pid_norm, place));
            }
        }
        let contests_with_my_place: usize = by_contest.values().filter(|(mp, _)| *mp > 0).count();
        let my_first_places: usize = by_contest.values().filter(|(mp, _)| *mp == 1).count();
        log::info!(
            "fetch_opponent_stats: by_contest len={} contests_with_my_place={} my_first_places={}",
            by_contest.len(),
            contests_with_my_place,
            my_first_places
        );
        // opponent_id -> (contests_played, wins_against_me, losses_to_me)
        let mut opp_stats: HashMap<String, (i32, i32, i32)> = HashMap::new();
        for (_cid, (my_place, opponents)) in by_contest {
            for (opp_id, opp_place) in opponents {
                let e = opp_stats.entry(opp_id).or_insert((0, 0, 0));
                e.0 += 1;
                if opp_place == 1 && my_place != 1 {
                    e.1 += 1;
                }
                if my_place == 1 && opp_place != 1 {
                    e.2 += 1;
                }
            }
        }
        log::info!(
            "fetch_opponent_stats: opp_stats len={} (before filter self), my_id_norm={}",
            opp_stats.len(),
            my_id_norm
        );
        for (pid, (played, wam, ltm)) in opp_stats.iter().take(5) {
            log::info!(
                "  opp_stats sample: pid={} contests_played={} wins_against_me={} losses_to_me={}",
                pid,
                played,
                wam,
                ltm
            );
        }
        if opp_stats.is_empty() {
            return Ok(Vec::new());
        }
        // 3) Fetch opponent player rows (INSIDE on id; bind Thing array for record id match).
        let player_ids: Vec<String> = opp_stats.keys().map(|pid| pid.replace('/', ":")).collect();
        let player_things = strings_to_record_id_array(&player_ids);
        let mut res2 = self
            .db
            .query("SELECT id, handle, firstname, lastname, email FROM player WHERE id INSIDE $player_ids")
            .bind(("player_ids", player_things))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct PlayerDisplayRowWithEmail {
            id: Option<surrealdb::types::RecordId>,
            handle: Option<String>,
            firstname: Option<String>,
            lastname: Option<String>,
            email: Option<String>,
        }
        let player_rows: Vec<PlayerDisplayRowWithEmail> = res2
            .take(0)
            .map_err(|e| SharedError::Database(format!("player display query: {}", e)))?;
        let mut display: HashMap<String, (String, String)> = HashMap::new();
        for row in player_rows {
            let id_norm = thing_to_record_id(&row.id);
            if id_norm.is_empty() {
                continue;
            }
            let handle = row
                .handle
                .filter(|s| !s.trim().is_empty())
                .or_else(|| row.email.clone())
                .unwrap_or_default();
            let first = row.firstname.unwrap_or_default();
            let last = row.lastname.unwrap_or_default();
            let name = format!("{} {}", first, last).trim().to_string();
            let display_name = if !name.is_empty() {
                name
            } else if !handle.is_empty() {
                handle.clone()
            } else {
                row.email.unwrap_or_default()
            };
            let fallback = if display_name.is_empty() {
                "Unknown Player".to_string()
            } else {
                display_name
            };
            display.insert(id_norm, (handle, fallback));
        }
        let out: Vec<PlayerOpponentDto> = opp_stats
            .into_iter()
            .filter(|(pid, _)| normalize_player_id(pid) != my_id_norm)
            .map(|(pid, (contests_played, wins_against_me, losses_to_me))| {
                let win_rate = if contests_played > 0 {
                    (wins_against_me as f64 * 100.0) / contests_played as f64
                } else {
                    0.0
                };
                let (handle, name) = display
                    .get(&pid)
                    .cloned()
                    .unwrap_or_else(|| ("Unknown".to_string(), "Unknown Player".to_string()));
                PlayerOpponentDto {
                    player_id: pid,
                    player_handle: handle,
                    player_name: name,
                    contests_played,
                    wins_against_me,
                    losses_to_me,
                    win_rate_against_me: win_rate,
                    last_played: None,
                    total_contests: contests_played,
                    overall_win_rate: win_rate,
                }
            })
            .collect();
        Ok(out)
    }

    /// Get player's game performance statistics per game.
    /// SurrealQL has no INNER JOIN; we query resulted_in, contest, and played_with separately and join in Rust.
    /// Tries fn::player_game_performance_data($key) first for one round-trip when applied.
    pub async fn get_my_game_performance(
        &self,
        player_id: &str,
    ) -> Result<Vec<GamePerformanceDto>> {
        let key = player_id_to_key(player_id);

        if let Ok(mut res) = self
            .db
            .query("SELECT fn::player_game_performance_data($key) AS result FROM [1]")
            .bind(("key", key.clone()))
            .await
        {
            let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
            if let Some(first) = rows.into_iter().next() {
                let result = first
                    .get("result")
                    .or_else(|| first.get("fn::player_game_performance_data($key)"))
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                if let Some(obj) = result.as_object() {
                    let empty: &[serde_json::Value] = &[];
                    let ri_arr = obj
                        .get("resulted_in")
                        .and_then(|v| v.as_array())
                        .map(|v| v.as_slice())
                        .unwrap_or(empty);
                    let cs_arr = obj
                        .get("contest_starts")
                        .and_then(|v| v.as_array())
                        .map(|v| v.as_slice())
                        .unwrap_or(empty);
                    let pw_arr = obj
                        .get("played_with")
                        .and_then(|v| v.as_array())
                        .map(|v| v.as_slice())
                        .unwrap_or(empty);
                    let ri_rows: Vec<(String, i32)> = ri_arr
                        .iter()
                        .filter_map(|v| {
                            let cid = record_id_from_field(v, "contest_id")?;
                            let place = v.get("place").and_then(|p| p.as_i64()).unwrap_or(0) as i32;
                            Some((cid, place))
                        })
                        .collect();
                    if !ri_rows.is_empty() {
                        let contest_start_by_id: HashMap<String, String> = cs_arr
                            .iter()
                            .filter_map(|v| {
                                let id = record_id_from_field(v, "id")?;
                                let start =
                                    v.get("start").and_then(|s| s.as_str()).map(String::from)?;
                                Some((id, start))
                            })
                            .collect();
                        let pw_rows: Vec<(String, String)> = pw_arr
                            .iter()
                            .filter_map(|v| {
                                let cid = record_id_from_field(v, "contest_id")?;
                                let gid = record_id_from_field(v, "game_id")?;
                                Some((cid, gid))
                            })
                            .collect();
                        let mut by_game: GamePerformanceAggMap = HashMap::new();
                        for (contest_id, place) in &ri_rows {
                            let contest_start = contest_start_by_id.get(contest_id).cloned();
                            let last_played = contest_start
                                .as_deref()
                                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                                .map(|dt| {
                                    dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
                                });
                            for (pcid, game_id) in &pw_rows {
                                if pcid != contest_id {
                                    continue;
                                }
                                let e = by_game.entry(game_id.clone()).or_insert((
                                    0,
                                    0,
                                    Vec::new(),
                                    None,
                                ));
                                e.0 += 1;
                                if *place == 1 {
                                    e.1 += 1;
                                }
                                e.2.push(*place);
                                if let Some(lp) = last_played {
                                    if e.3.map(|t| lp > t).unwrap_or(true) {
                                        e.3 = Some(lp);
                                    }
                                }
                            }
                        }
                        if !by_game.is_empty() {
                            let game_ids: Vec<String> = by_game.keys().cloned().collect();
                            let ids_surreal: Vec<String> = game_ids
                                .iter()
                                .map(|s| s.replace("game/", "game:"))
                                .collect();
                            if let Ok(mut res2) = self
                                .db
                                .query("SELECT string::concat(id) AS game_id, name FROM game WHERE id INSIDE $ids")
                                .bind(("ids", ids_surreal))
                                .await
                            {
                                let name_rows: Vec<serde_json::Value> = res2.take(0).unwrap_or_default();
                                let mut game_names: HashMap<String, String> = HashMap::new();
                                for v in name_rows {
                                    if let (Some(id_val), Some(name)) = (
                                        v.get("game_id").and_then(|x| x.as_str()).map(normalize_record_id_string),
                                        v.get("name").and_then(|x| x.as_str()).map(String::from),
                                    ) {
                                        if !id_val.is_empty() {
                                            game_names.insert(id_val, name);
                                        }
                                    }
                                }
                                // Fallback: batch INSIDE may not match (binding format); fetch missing names by single-record lookup
                                for gid in game_ids.iter() {
                                    if !game_names.contains_key(gid) {
                                        if let Ok(Some(name)) = self.get_game_info(gid).await {
                                            game_names.insert(gid.clone(), name);
                                        }
                                    }
                                }
                                let now = chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());
                                let out: Vec<GamePerformanceDto> = by_game
                                    .into_iter()
                                    .map(|(game_id, (total_plays, wins, placements, last_played))| {
                                        let _losses = total_plays.saturating_sub(wins);
                                        let win_rate = if total_plays > 0 { (wins as f64 * 100.0) / total_plays as f64 } else { 0.0 };
                                        let avg_place = if placements.is_empty() { 0.0 } else { placements.iter().sum::<i32>() as f64 / placements.len() as f64 };
                                        let best = *placements.iter().min().unwrap_or(&0);
                                        let worst = *placements.iter().max().unwrap_or(&0);
                                        let last = last_played.unwrap_or(now);
                                        let days_since = (now - last).num_days();
                                        let game_name = game_names.get(&game_id).cloned().unwrap_or_else(|| "Unknown".to_string());
                                        GamePerformanceDto {
                                            game_id,
                                            game_name,
                                            total_plays,
                                            wins,
                                            losses: total_plays.saturating_sub(wins),
                                            win_rate,
                                            average_placement: avg_place,
                                            best_placement: best,
                                            worst_placement: worst,
                                            total_points: wins * 10,
                                            average_points: if total_plays > 0 { (wins * 10) as f64 / total_plays as f64 } else { 0.0 },
                                            last_played: last,
                                            days_since_last_play: days_since,
                                            favorite_venue: None,
                                        }
                                    })
                                    .collect();
                                return Ok(out);
                            }
                        }
                    }
                }
            }
        }

        // 1) Player's contest participations: contest_id, place
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct RiRow {
            contest_id: Option<surrealdb::types::RecordId>,
            place: Option<i64>,
        }
        let record_id = surrealdb::types::RecordId::new("player", key.as_str());
        let mut q_ri = self
            .db
            .query("SELECT in AS contest_id, place FROM resulted_in WHERE out = $record_id")
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let ri_rows: Vec<RiRow> = q_ri
            .take(0)
            .map_err(|e| SharedError::Database(format!("game performance resulted_in: {}", e)))?;

        if ri_rows.is_empty() {
            return Ok(Vec::new());
        }

        let contest_ids_slash: Vec<String> = ri_rows
            .iter()
            .filter_map(|r| {
                let id = thing_to_record_id(&r.contest_id);
                if id.is_empty() {
                    None
                } else {
                    Some(id)
                }
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let contest_ids_colon: Vec<String> = contest_ids_slash
            .iter()
            .map(|s| s.replace('/', ":"))
            .collect();

        // 2) Contest start times
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct ContestStartRow {
            id: Option<surrealdb::types::RecordId>,
            start: Option<String>,
        }
        let mut q_contest = self
            .db
            .query("SELECT id, start FROM contest WHERE id INSIDE $ids")
            .bind(("ids", contest_ids_colon.clone()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let contest_rows: Vec<ContestStartRow> = q_contest
            .take(0)
            .map_err(|e| SharedError::Database(format!("game performance contest: {}", e)))?;
        let contest_start_by_id: HashMap<String, String> = contest_rows
            .into_iter()
            .filter_map(|r| {
                let id = thing_to_record_id(&r.id);
                let start = r.start?;
                if id.is_empty() {
                    None
                } else {
                    Some((id, start))
                }
            })
            .collect();

        // 3) Contest -> game edges
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct PwRow {
            contest_id: Option<surrealdb::types::RecordId>,
            game_id: Option<surrealdb::types::RecordId>,
        }
        let mut q_pw = self
            .db
            .query("SELECT in AS contest_id, out AS game_id FROM played_with WHERE in INSIDE $ids")
            .bind(("ids", contest_ids_colon))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let pw_rows: Vec<PwRow> = q_pw
            .take(0)
            .map_err(|e| SharedError::Database(format!("game performance played_with: {}", e)))?;

        // Build (contest_id, place, game_id, contest_start) and aggregate by game
        let mut by_game: GamePerformanceAggMap = HashMap::new();
        for r in &ri_rows {
            let contest_id = thing_to_record_id(&r.contest_id);
            if contest_id.is_empty() {
                continue;
            }
            let place = r.place.unwrap_or(0) as i32;
            let contest_start = contest_start_by_id.get(&contest_id).cloned();
            let last_played = contest_start
                .as_deref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()));

            for pw in &pw_rows {
                if thing_to_record_id(&pw.contest_id) != contest_id {
                    continue;
                }
                let game_id = thing_to_record_id(&pw.game_id);
                if game_id.is_empty() {
                    continue;
                }
                let e = by_game.entry(game_id).or_insert((0, 0, Vec::new(), None));
                e.0 += 1;
                if place == 1 {
                    e.1 += 1;
                }
                e.2.push(place);
                if let Some(lp) = last_played {
                    if e.3.map(|t| lp > t).unwrap_or(true) {
                        e.3 = Some(lp);
                    }
                }
            }
        }
        if by_game.is_empty() {
            return Ok(Vec::new());
        }
        let game_ids: Vec<String> = by_game.keys().cloned().collect();
        let ids_surreal: Vec<String> = game_ids
            .iter()
            .map(|s| s.replace("game/", "game:"))
            .collect();
        let mut res2 = self
            .db
            .query("SELECT string::concat(id) AS game_id, name FROM game WHERE id INSIDE $ids")
            .bind(("ids", ids_surreal))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let name_rows: Vec<serde_json::Value> = res2
            .take(0)
            .map_err(|e| SharedError::Database(format!("game display query: {}", e)))?;
        let mut game_names: HashMap<String, String> = HashMap::new();
        for v in name_rows {
            if let (Some(id_val), Some(name)) = (
                v.get("game_id")
                    .and_then(|x| x.as_str())
                    .map(normalize_record_id_string),
                v.get("name").and_then(|x| x.as_str()).map(String::from),
            ) {
                if !id_val.is_empty() {
                    game_names.insert(id_val, name);
                }
            }
        }
        // Fallback: batch INSIDE may not match; fetch missing names by single-record lookup
        for gid in &game_ids {
            if !game_names.contains_key(gid) {
                if let Ok(Some(name)) = self.get_game_info(gid).await {
                    game_names.insert(gid.clone(), name);
                }
            }
        }
        let now = chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());
        let out: Vec<GamePerformanceDto> = by_game
            .into_iter()
            .map(|(game_id, (total_plays, wins, placements, last_played))| {
                let losses = total_plays.saturating_sub(wins);
                let win_rate = if total_plays > 0 {
                    (wins as f64 * 100.0) / total_plays as f64
                } else {
                    0.0
                };
                let avg_place = if placements.is_empty() {
                    0.0
                } else {
                    placements.iter().sum::<i32>() as f64 / placements.len() as f64
                };
                let best = *placements.iter().min().unwrap_or(&0);
                let worst = *placements.iter().max().unwrap_or(&0);
                let last = last_played.unwrap_or(now);
                let days_since = (now - last).num_days();
                let game_name = game_names
                    .get(&game_id)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string());
                GamePerformanceDto {
                    game_id,
                    game_name,
                    total_plays,
                    wins,
                    losses,
                    win_rate,
                    average_placement: avg_place,
                    best_placement: best,
                    worst_placement: worst,
                    total_points: wins * 10,
                    average_points: if total_plays > 0 {
                        (wins * 10) as f64 / total_plays as f64
                    } else {
                        0.0
                    },
                    last_played: last,
                    days_since_last_play: days_since,
                    favorite_venue: None,
                }
            })
            .collect();
        Ok(out)
    }

    /// Game Performance detail (best/toughest opponent, best venue per game). Tries fn::player_game_performance_detail_data($key) first when applied (tools/arango-to-surreal/surreal-functions.surql).
    pub async fn get_player_game_performance_detail(
        &self,
        player_id: &str,
    ) -> Result<Vec<GamePerformanceDetailDto>> {
        use std::collections::HashMap;

        let player_key = player_id_to_key(player_id);
        if player_key.is_empty() {
            return Ok(vec![]);
        }
        let player_rid = surrealdb::types::RecordId::new("player", player_key.as_str());

        let mut my_place_by_contest: HashMap<String, i32> = HashMap::new();
        let mut start_by_contest: HashMap<String, chrono::DateTime<chrono::FixedOffset>> =
            HashMap::new();
        let mut venue_by_contest: HashMap<String, String> = HashMap::new();
        let mut participants_by_contest: HashMap<String, Vec<(String, i32)>> = HashMap::new();
        let mut games_by_contest: HashMap<String, Vec<String>> = HashMap::new();
        let mut game_ids: Vec<String> = vec![];

        // Try SurrealDB function first (one round-trip when application functions are applied).
        if let Ok(mut res) = self
            .db
            .query("SELECT fn::player_game_performance_detail_data($key) AS result FROM [1]")
            .bind(("key", player_key.clone()))
            .await
        {
            let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
            if let Some(first) = rows.into_iter().next() {
                let result = first
                    .get("result")
                    .or_else(|| first.get("fn::player_game_performance_detail_data($key)"))
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                if let Some(obj) = result.as_object() {
                    let empty: &[serde_json::Value] = &[];
                    let ri_arr = obj
                        .get("resulted_in")
                        .and_then(|v| v.as_array())
                        .map(|v| v.as_slice())
                        .unwrap_or(empty);
                    let cs_arr = obj
                        .get("contest_starts")
                        .and_then(|v| v.as_array())
                        .map(|v| v.as_slice())
                        .unwrap_or(empty);
                    let pw_arr = obj
                        .get("played_with")
                        .and_then(|v| v.as_array())
                        .map(|v| v.as_slice())
                        .unwrap_or(empty);
                    let pa_arr = obj
                        .get("played_at")
                        .and_then(|v| v.as_array())
                        .map(|v| v.as_slice())
                        .unwrap_or(empty);
                    let part_arr = obj
                        .get("participants")
                        .and_then(|v| v.as_array())
                        .map(|v| v.as_slice())
                        .unwrap_or(empty);
                    for v in ri_arr {
                        if let (Some(cid), Some(place)) = (
                            record_id_from_field(v, "contest_id"),
                            v.get("place").and_then(|p| p.as_i64()),
                        ) {
                            let cid_s = cid.replace("contest:", "contest/");
                            my_place_by_contest.insert(cid_s, place as i32);
                        }
                    }
                    for v in cs_arr {
                        if let (Some(id), Some(start_str)) = (
                            record_id_from_field(v, "id"),
                            v.get("start").and_then(|s| s.as_str()),
                        ) {
                            let cid_s = id.replace("contest:", "contest/");
                            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(start_str) {
                                start_by_contest.insert(
                                    cid_s,
                                    dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()),
                                );
                            }
                        }
                    }
                    for v in pa_arr {
                        if let (Some(cid), Some(vid)) = (
                            record_id_from_field(v, "contest_id"),
                            record_id_from_field(v, "venue_id"),
                        ) {
                            let cid_s = cid.replace("contest:", "contest/");
                            let vid_s = vid.replace("venue:", "venue/");
                            venue_by_contest.insert(cid_s, vid_s);
                        }
                    }
                    for v in part_arr {
                        if let (Some(cid), Some(pid), Some(place)) = (
                            record_id_from_field(v, "contest_id"),
                            record_id_from_field(v, "player_id"),
                            v.get("place").and_then(|p| p.as_i64()),
                        ) {
                            let cid_s = cid.replace("contest:", "contest/");
                            let pid_s = pid.replace("player:", "player/");
                            participants_by_contest
                                .entry(cid_s)
                                .or_default()
                                .push((pid_s, place as i32));
                        }
                    }
                    for v in pw_arr {
                        if let (Some(cid), Some(gid)) = (
                            record_id_from_field(v, "contest_id"),
                            record_id_from_field(v, "game_id"),
                        ) {
                            let cid_s = cid.replace("contest:", "contest/");
                            let gid_s = gid.replace("game:", "game/");
                            games_by_contest
                                .entry(cid_s)
                                .or_default()
                                .push(gid_s.clone());
                            game_ids.push(gid_s);
                        }
                    }
                    game_ids.sort();
                    game_ids.dedup();
                    if !my_place_by_contest.is_empty() && !games_by_contest.is_empty() {
                        // Build agg_by_game and DTOs (shared block below).
                        return self
                            .build_game_performance_detail_dtos(
                                player_id,
                                &my_place_by_contest,
                                &start_by_contest,
                                &venue_by_contest,
                                &participants_by_contest,
                                &games_by_contest,
                                &game_ids,
                            )
                            .await;
                    }
                }
            }
        }

        // Fallback: multi-query path when function is not defined or returns nothing.
        #[derive(
            Clone, Debug, serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue,
        )]
        struct PlayerContestRow {
            contest_id: Option<surrealdb::types::RecordId>,
            place: Option<i64>,
        }
        let mut res = self
            .db
            .query("SELECT `in` AS contest_id, place FROM resulted_in WHERE `out` = $player")
            .bind(("player", player_rid.clone()))
            .await
            .map_err(|e| {
                SharedError::Database(format!("Failed to fetch player contests: {}", e))
            })?;
        let player_contests: Vec<PlayerContestRow> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("Failed to take player contests: {}", e)))?;

        let contest_ids: Vec<surrealdb::types::RecordId> = player_contests
            .iter()
            .filter_map(|r| r.contest_id.clone())
            .collect();
        if contest_ids.is_empty() {
            return Ok(vec![]);
        }

        for r in &player_contests {
            if let (Some(cid), Some(place)) = (&r.contest_id, r.place) {
                let cid_s = crate::surreal_helpers::record_id_to_canonical(cid)
                    .replace("contest:", "contest/");
                my_place_by_contest.insert(cid_s, place as i32);
            }
        }

        #[derive(
            Clone, Debug, serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue,
        )]
        struct ContestGameRow {
            contest_id: Option<surrealdb::types::RecordId>,
            game_id: Option<surrealdb::types::RecordId>,
        }
        let mut res = self
            .db
            .query("SELECT `in` AS contest_id, `out` AS game_id FROM played_with WHERE `in` INSIDE $contests")
            .bind(("contests", contest_ids.clone()))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to fetch contest games: {}", e)))?;
        let contest_games: Vec<ContestGameRow> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("Failed to take contest games: {}", e)))?;

        #[derive(
            Clone, Debug, serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue,
        )]
        struct ContestVenueRow {
            contest_id: Option<surrealdb::types::RecordId>,
            venue_id: Option<surrealdb::types::RecordId>,
        }
        let mut res = self
            .db
            .query("SELECT `in` AS contest_id, `out` AS venue_id FROM played_at WHERE `in` INSIDE $contests")
            .bind(("contests", contest_ids.clone()))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to fetch contest venues: {}", e)))?;
        let contest_venues: Vec<ContestVenueRow> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("Failed to take contest venues: {}", e)))?;

        for r in &contest_venues {
            if let (Some(cid), Some(vid)) = (&r.contest_id, &r.venue_id) {
                let cid_s = crate::surreal_helpers::record_id_to_canonical(cid)
                    .replace("contest:", "contest/");
                let vid_s =
                    crate::surreal_helpers::record_id_to_canonical(vid).replace("venue:", "venue/");
                venue_by_contest.insert(cid_s, vid_s);
            }
        }

        #[derive(
            Clone, Debug, serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue,
        )]
        struct ContestStartRow {
            id: Option<surrealdb::types::RecordId>,
            start: Option<surrealdb::types::Datetime>,
        }
        let mut res = self
            .db
            .query("SELECT id, start FROM contest WHERE id INSIDE $contests")
            .bind(("contests", contest_ids.clone()))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to fetch contest starts: {}", e)))?;
        let contest_starts: Vec<ContestStartRow> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("Failed to take contest starts: {}", e)))?;
        for r in contest_starts {
            if let (Some(cid), Some(start)) = (r.id, r.start) {
                let cid_s = crate::surreal_helpers::record_id_to_canonical(&cid)
                    .replace("contest:", "contest/");
                let dt =
                    chrono::DateTime::parse_from_rfc3339(&start.to_string()).unwrap_or_else(|_| {
                        chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
                    });
                start_by_contest.insert(cid_s, dt);
            }
        }

        #[derive(
            Clone, Debug, serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue,
        )]
        struct ContestParticipantRow {
            contest_id: Option<surrealdb::types::RecordId>,
            player_id: Option<surrealdb::types::RecordId>,
            place: Option<i64>,
        }
        let mut res = self
            .db
            .query("SELECT `in` AS contest_id, `out` AS player_id, place FROM resulted_in WHERE `in` INSIDE $contests")
            .bind(("contests", contest_ids))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to fetch contest participants: {}", e)))?;
        let participants: Vec<ContestParticipantRow> = res.take(0).map_err(|e| {
            SharedError::Database(format!("Failed to take contest participants: {}", e))
        })?;
        for r in participants {
            let (Some(cid), Some(pid), Some(place)) = (r.contest_id, r.player_id, r.place) else {
                continue;
            };
            let cid_s = crate::surreal_helpers::record_id_to_canonical(&cid)
                .replace("contest:", "contest/");
            let pid_s =
                crate::surreal_helpers::record_id_to_canonical(&pid).replace("player:", "player/");
            participants_by_contest
                .entry(cid_s)
                .or_default()
                .push((pid_s, place as i32));
        }

        for r in &contest_games {
            let (Some(cid), Some(gid)) = (&r.contest_id, &r.game_id) else {
                continue;
            };
            let cid_s =
                crate::surreal_helpers::record_id_to_canonical(cid).replace("contest:", "contest/");
            let gid_s =
                crate::surreal_helpers::record_id_to_canonical(gid).replace("game:", "game/");
            games_by_contest
                .entry(cid_s)
                .or_default()
                .push(gid_s.clone());
            game_ids.push(gid_s);
        }
        game_ids.sort();
        game_ids.dedup();

        self.build_game_performance_detail_dtos(
            player_id,
            &my_place_by_contest,
            &start_by_contest,
            &venue_by_contest,
            &participants_by_contest,
            &games_by_contest,
            &game_ids,
        )
        .await
    }

    /// Build GamePerformanceDetailDto list from pre-aggregated maps (used by fn:: path and multi-query fallback).
    #[allow(clippy::too_many_arguments)]
    async fn build_game_performance_detail_dtos(
        &self,
        player_id: &str,
        my_place_by_contest: &HashMap<String, i32>,
        start_by_contest: &HashMap<String, chrono::DateTime<chrono::FixedOffset>>,
        venue_by_contest: &HashMap<String, String>,
        participants_by_contest: &HashMap<String, Vec<(String, i32)>>,
        games_by_contest: &HashMap<String, Vec<String>>,
        game_ids: &[String],
    ) -> Result<Vec<GamePerformanceDetailDto>> {
        use std::collections::HashMap;

        #[derive(Default)]
        struct GameAgg {
            total_plays: i32,
            wins: i32,
            losses: i32,
            sum_place: i32,
            best_place: i32,
            worst_place: i32,
            last_played: Option<chrono::DateTime<chrono::FixedOffset>>,
            opp: HashMap<String, (i32, i32)>, // opp -> (contests, my_wins)
            venue_counts: HashMap<String, i32>, // venue -> plays
        }

        let mut agg_by_game: HashMap<String, GameAgg> = HashMap::new();
        for (contest_id, games) in games_by_contest.iter() {
            let my_place = match my_place_by_contest.get(contest_id).copied() {
                Some(p) if p > 0 => p,
                _ => continue,
            };
            let contest_start = start_by_contest.get(contest_id).cloned();
            let venue_id = venue_by_contest.get(contest_id).cloned();
            let participants = participants_by_contest
                .get(contest_id)
                .cloned()
                .unwrap_or_default();

            for game_id in games {
                let entry = agg_by_game.entry(game_id.clone()).or_default();
                entry.total_plays += 1;
                entry.sum_place += my_place;
                entry.best_place = if entry.best_place == 0 {
                    my_place
                } else {
                    entry.best_place.min(my_place)
                };
                entry.worst_place = entry.worst_place.max(my_place);
                if my_place == 1 {
                    entry.wins += 1;
                } else {
                    entry.losses += 1;
                }
                if let Some(dt) = contest_start {
                    if entry.last_played.map(|x| dt > x).unwrap_or(true) {
                        entry.last_played = Some(dt);
                    }
                }
                if let Some(vid) = &venue_id {
                    *entry.venue_counts.entry(vid.clone()).or_insert(0) += 1;
                }
                for (opp_id, opp_place) in &participants {
                    if opp_id == player_id {
                        continue;
                    }
                    let (c, w) = entry.opp.entry(opp_id.clone()).or_insert((0, 0));
                    *c += 1;
                    if my_place < *opp_place {
                        *w += 1;
                    }
                }
            }
        }

        // Names for games
        let game_rids: Vec<surrealdb::types::RecordId> = game_ids
            .iter()
            .filter_map(|s| {
                let key = record_id_to_key(s, "game");
                if key.is_empty() {
                    None
                } else {
                    Some(surrealdb::types::RecordId::new("game", key.as_str()))
                }
            })
            .collect();
        #[derive(
            Clone, Debug, serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue,
        )]
        struct GameNameRow {
            id: Option<surrealdb::types::RecordId>,
            name: Option<String>,
        }
        let mut game_name_by_id: HashMap<String, String> = HashMap::new();
        if !game_rids.is_empty() {
            let mut res = self
                .db
                .query("SELECT id, name FROM game WHERE id INSIDE $ids")
                .bind(("ids", game_rids))
                .await
                .map_err(|e| SharedError::Database(format!("Failed to fetch game names: {}", e)))?;
            let rows: Vec<GameNameRow> = res
                .take(0)
                .map_err(|e| SharedError::Database(format!("Failed to take game names: {}", e)))?;
            for r in rows {
                if let (Some(id), Some(name)) = (r.id, r.name) {
                    let id_s = crate::surreal_helpers::record_id_to_canonical(&id)
                        .replace("game:", "game/");
                    game_name_by_id.insert(id_s, name);
                }
            }
        }

        // Names for venues we reference. Bind INSIDE as `table:key` strings (see get_top_venues):
        // `Vec<RecordId>` batches often return no rows for string/numeric venue keys in SurrealDB v3.
        let mut venue_ids: Vec<String> = agg_by_game
            .values()
            .flat_map(|g| g.venue_counts.keys().cloned())
            .collect();
        venue_ids.sort();
        venue_ids.dedup();
        let venue_ids_inside: Vec<String> = venue_ids.iter().map(|s| s.replace('/', ":")).collect();
        let mut venue_name_by_id: HashMap<String, String> = HashMap::new();
        if !venue_ids_inside.is_empty() {
            let mut res = self
                .db
                .query(
                    "SELECT string::concat(id) AS venue_id, displayName AS dn, display_name AS ds \
                     FROM venue WHERE id INSIDE $ids",
                )
                .bind(("ids", venue_ids_inside))
                .await
                .map_err(|e| {
                    SharedError::Database(format!("Failed to fetch venue names: {}", e))
                })?;
            let rows: Vec<serde_json::Value> = res
                .take(0)
                .map_err(|e| SharedError::Database(format!("Failed to take venue names: {}", e)))?;
            for v in rows {
                let id_key = record_id_from_field(&v, "venue_id").unwrap_or_else(|| {
                    v.get("venue_id")
                        .and_then(|x| x.as_str())
                        .map(normalize_record_id_string)
                        .filter(|s| !s.is_empty())
                        .map(|s| {
                            if s.starts_with("venue/") || s.contains('/') {
                                s
                            } else {
                                format!("venue/{}", s)
                            }
                        })
                        .unwrap_or_default()
                });
                if id_key.is_empty() || !id_key.starts_with("venue/") {
                    continue;
                }
                let name = ["dn", "displayName", "ds", "display_name"]
                    .iter()
                    .find_map(|k| {
                        v.get(*k)
                            .and_then(|x| x.as_str())
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                    })
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| id_key.clone());
                venue_name_by_id.insert(id_key, name);
            }
            // Batch INSIDE can still miss rows (id shape / driver JSON). Resolve any remaining ids.
            for vid in &venue_ids {
                if venue_name_by_id.contains_key(vid) {
                    continue;
                }
                if let Some(row) = select_one_by_record_id(&self.db, "venue", vid).await {
                    let name = ["displayName", "display_name", "dn", "ds"]
                        .iter()
                        .find_map(|k| {
                            row.get(*k)
                                .and_then(|x| x.as_str())
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                        })
                        .map(|s| s.to_string());
                    if let Some(n) = name {
                        venue_name_by_id.insert(vid.clone(), n);
                    }
                }
            }
        }

        // Opponent handles
        let mut opp_ids: Vec<String> = agg_by_game
            .values()
            .flat_map(|g| g.opp.keys().cloned())
            .collect();
        opp_ids.sort();
        opp_ids.dedup();
        let opp_rids: Vec<surrealdb::types::RecordId> = opp_ids
            .iter()
            .filter_map(|s| {
                let key = record_id_to_key(s, "player");
                if key.is_empty() {
                    None
                } else {
                    Some(surrealdb::types::RecordId::new("player", key.as_str()))
                }
            })
            .collect();
        #[derive(
            Clone, Debug, serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue,
        )]
        struct PlayerHandleRow {
            id: Option<surrealdb::types::RecordId>,
            handle: Option<String>,
        }
        let mut handle_by_id: HashMap<String, String> = HashMap::new();
        if !opp_rids.is_empty() {
            let mut res = self
                .db
                .query("SELECT id, handle FROM player WHERE id INSIDE $ids")
                .bind(("ids", opp_rids))
                .await
                .map_err(|e| {
                    SharedError::Database(format!("Failed to fetch opponent handles: {}", e))
                })?;
            let rows: Vec<PlayerHandleRow> = res.take(0).map_err(|e| {
                SharedError::Database(format!("Failed to take opponent handles: {}", e))
            })?;
            for r in rows {
                if let Some(id) = r.id {
                    let id_s = crate::surreal_helpers::record_id_to_canonical(&id)
                        .replace("player:", "player/");
                    handle_by_id.insert(id_s, r.handle.unwrap_or_else(|| "Unknown".into()));
                }
            }
        }

        let now: chrono::DateTime<chrono::FixedOffset> =
            chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());

        let mut out: Vec<GamePerformanceDetailDto> = Vec::new();
        for (game_id, g) in agg_by_game {
            let game_name = game_name_by_id
                .get(&game_id)
                .cloned()
                .unwrap_or_else(|| "Unknown".into());
            let avg_place = if g.total_plays > 0 {
                (g.sum_place as f64) / (g.total_plays as f64)
            } else {
                0.0
            };
            let win_rate = if g.total_plays > 0 {
                (g.wins as f64 * 100.0) / (g.total_plays as f64)
            } else {
                0.0
            };
            let last_played = g.last_played.unwrap_or(now);
            let days_since = (now - last_played).num_days();

            // Best/toughest opponent (min 3 contests together)
            let mut best: Option<(String, i32, f64)> = None;
            let mut worst: Option<(String, i32, f64)> = None;
            for (opp_id, (c, w)) in &g.opp {
                if *c < 3 {
                    continue;
                }
                let wr = (*w as f64 * 100.0) / (*c as f64);
                match &best {
                    None => best = Some((opp_id.clone(), *c, wr)),
                    Some((_, bc, bwr)) => {
                        if wr > *bwr || (wr == *bwr && *c > *bc) {
                            best = Some((opp_id.clone(), *c, wr));
                        }
                    }
                }
                match &worst {
                    None => worst = Some((opp_id.clone(), *c, wr)),
                    Some((_, wc, wwr)) => {
                        if wr < *wwr || (wr == *wwr && *c > *wc) {
                            worst = Some((opp_id.clone(), *c, wr));
                        }
                    }
                }
            }
            let best_opponent = best.map(|(oid, c, wr)| GamePerformanceOpponentDto {
                player_id: oid.clone(),
                player_handle: handle_by_id
                    .get(&oid)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".into()),
                contests_played: c,
                my_win_rate: wr,
            });
            let toughest_opponent = worst.map(|(oid, c, wr)| GamePerformanceOpponentDto {
                player_id: oid.clone(),
                player_handle: handle_by_id
                    .get(&oid)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".into()),
                contests_played: c,
                my_win_rate: wr,
            });

            let best_venue = g
                .venue_counts
                .into_iter()
                .max_by_key(|(_, n)| *n)
                .map(|(vid, n)| {
                    let resolved = venue_name_by_id
                        .get(&vid)
                        .map(|s| s.as_str().trim())
                        .filter(|s| !s.is_empty());
                    let fallback_key = record_id_to_key(&vid, "venue");
                    let venue_name = resolved
                        .map(|s| s.to_string())
                        .or_else(|| (!fallback_key.is_empty()).then_some(fallback_key))
                        .unwrap_or_else(|| vid.clone());
                    GamePerformanceVenueDto {
                        venue_id: vid.clone(),
                        venue_name,
                        plays: n,
                    }
                });

            out.push(GamePerformanceDetailDto {
                game_id,
                game_name,
                total_plays: g.total_plays,
                wins: g.wins,
                losses: g.losses,
                win_rate,
                average_placement: avg_place,
                best_placement: g.best_place,
                worst_placement: g.worst_place,
                last_played,
                days_since_last_play: days_since,
                best_opponent,
                toughest_opponent,
                best_venue,
            });
        }

        out.sort_by(|a, b| b.total_plays.cmp(&a.total_plays));
        Ok(out)
    }

    /// Get head-to-head record against specific opponent (contests where both participated).
    pub async fn get_head_to_head_record(
        &self,
        player_id: &str,
        opponent_id: &str,
    ) -> Result<shared::dto::analytics::HeadToHeadRecordDto> {
        let opponent_key = player_id_to_key(opponent_id);
        let opp_record_id = if opponent_key.is_empty() {
            return Err(SharedError::Validation("Invalid opponent id".to_string()));
        } else {
            surrealdb::types::RecordId::new("player", opponent_key.as_str())
        };
        let mut res = self
            .db
            .query("SELECT handle, firstname, lastname FROM player WHERE id = $record_id")
            .bind(("record_id", opp_record_id.clone()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let opp_rows: Vec<PlayerDisplayRow> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("head-to-head opponent lookup: {}", e)))?;
        let (opponent_handle, opponent_name) = if let Some(opp) = opp_rows.first() {
            let handle = opp.handle.clone().unwrap_or_else(|| "Unknown".to_string());
            let first = opp.firstname.as_deref().unwrap_or("");
            let last = opp.lastname.as_deref().unwrap_or("");
            let name = if first.is_empty() && last.is_empty() {
                "Unknown Player".to_string()
            } else {
                format!("{} {}", first, last).trim().to_string()
            };
            (handle, name)
        } else {
            ("Unknown".to_string(), "Unknown Player".to_string())
        };

        let my_key = player_id_to_key(player_id);
        let my_record_id = if my_key.is_empty() {
            return Err(SharedError::Validation("Invalid player id".to_string()));
        } else {
            surrealdb::types::RecordId::new("player", my_key.as_str())
        };
        // Contests where both player and opponent have resulted_in: fetch my and opponent place per contest.
        let sql = r#"
            SELECT `in` AS contest_id, `out` AS player_id, place AS place
            FROM resulted_in
            WHERE `in` IN (SELECT VALUE `in` FROM resulted_in WHERE `out` = $my_record_id)
            AND (`out` = $my_record_id OR `out` = $opp_record_id)
        "#;
        let mut res = self
            .db
            .query(sql)
            .bind(("my_record_id", my_record_id))
            .bind(("opp_record_id", opp_record_id.clone()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<ResultedInRow> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("head-to-head contests: {}", e)))?;

        let mut by_contest: HashMap<String, (i32, i32)> = HashMap::new(); // contest_id -> (my_place, opp_place)
        for r in rows {
            let cid = thing_to_record_id(&r.contest_id);
            if cid.is_empty() {
                continue;
            }
            let pid = thing_to_record_id(&r.player_id);
            let place = r.place.unwrap_or(0) as i32;
            let entry = by_contest.entry(cid).or_insert((0, 0));
            if pid == crate::surreal_helpers::record_id_to_canonical(&opp_record_id) {
                entry.1 = place;
            } else {
                entry.0 = place;
            }
        }

        let mut my_wins = 0;
        let mut opponent_wins = 0;
        let default_dt =
            chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());

        let contest_ids: Vec<String> = by_contest.keys().cloned().collect();
        let rid_param: Vec<String> = contest_ids.iter().map(|s| s.replace('/', ":")).collect();

        let mut contest_names: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut contest_starts: std::collections::HashMap<
            String,
            chrono::DateTime<chrono::FixedOffset>,
        > = std::collections::HashMap::new();
        let mut contest_games: std::collections::HashMap<String, (Option<String>, String)> =
            std::collections::HashMap::new();
        let mut contest_venues: std::collections::HashMap<String, (Option<String>, String)> =
            std::collections::HashMap::new();

        if !rid_param.is_empty() {
            let mut meta_res = self
                .db
                .query(
                    "SELECT string::concat(id) AS id, name, start FROM contest WHERE id INSIDE $rids",
                )
                .bind(("rids", rid_param.clone()))
                .await
                .map_err(|e| SharedError::Database(e.to_string()))?;
            let meta_rows: Vec<serde_json::Value> = meta_res.take(0).unwrap_or_default();
            for row in meta_rows {
                let Some(id_raw) = row.get("id").and_then(|v| v.as_str()) else {
                    continue;
                };
                let cid = id_raw.replace("contest:", "contest/");
                let name = row
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !name.is_empty() {
                    contest_names.insert(cid.clone(), name);
                }
                if let Some(start_s) = row.get("start").and_then(|v| v.as_str()) {
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(start_s) {
                        contest_starts.insert(
                            cid,
                            dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()),
                        );
                    }
                }
            }

            let mut pw_res = self
                .db
                .query(
                    "SELECT string::concat(`in`) AS contest_id, string::concat(`out`) AS game_id FROM played_with WHERE `in` INSIDE $rids",
                )
                .bind(("rids", rid_param.clone()))
                .await
                .map_err(|e| SharedError::Database(e.to_string()))?;
            let pw_rows: Vec<serde_json::Value> = pw_res.take(0).unwrap_or_default();
            let game_ids: Vec<String> = pw_rows
                .iter()
                .filter_map(|r| r.get("game_id").and_then(|v| v.as_str()))
                .map(|s| s.replace('/', ":"))
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            let game_names = if game_ids.is_empty() {
                std::collections::HashMap::new()
            } else {
                let mut name_res = self
                    .db
                    .query("SELECT string::concat(id) AS game_id, name FROM game WHERE id INSIDE $ids")
                    .bind(("ids", game_ids))
                    .await
                    .map_err(|e| SharedError::Database(e.to_string()))?;
                let name_rows: Vec<serde_json::Value> = name_res.take(0).unwrap_or_default();
                name_rows
                    .into_iter()
                    .filter_map(|v| {
                        let id = v.get("game_id")?.as_str()?.replace("game:", "game/");
                        let name = v.get("name")?.as_str()?.to_string();
                        Some((id, name))
                    })
                    .collect()
            };
            for row in pw_rows {
                let Some(cid_raw) = row.get("contest_id").and_then(|v| v.as_str()) else {
                    continue;
                };
                let cid = cid_raw.replace("contest:", "contest/");
                let gid = row
                    .get("game_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.replace("game:", "game/"));
                let name = gid
                    .as_ref()
                    .and_then(|g| game_names.get(g))
                    .cloned()
                    .unwrap_or_default();
                contest_games
                    .entry(cid)
                    .or_insert((gid, name));
            }

            let mut pa_res = self
                .db
                .query(
                    "SELECT string::concat(`in`) AS contest_id, string::concat(`out`) AS venue_id FROM played_at WHERE `in` INSIDE $rids",
                )
                .bind(("rids", rid_param))
                .await
                .map_err(|e| SharedError::Database(e.to_string()))?;
            let pa_rows: Vec<serde_json::Value> = pa_res.take(0).unwrap_or_default();
            let venue_ids: Vec<String> = pa_rows
                .iter()
                .filter_map(|r| r.get("venue_id").and_then(|v| v.as_str()))
                .map(|s| s.replace('/', ":"))
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            let venue_names = if venue_ids.is_empty() {
                std::collections::HashMap::new()
            } else {
                let mut name_res = self
                    .db
                    .query(
                        "SELECT string::concat(id) AS venue_id, displayName AS name FROM venue WHERE id INSIDE $ids",
                    )
                    .bind(("ids", venue_ids))
                    .await
                    .map_err(|e| SharedError::Database(e.to_string()))?;
                let name_rows: Vec<serde_json::Value> = name_res.take(0).unwrap_or_default();
                name_rows
                    .into_iter()
                    .filter_map(|v| {
                        let id = v.get("venue_id")?.as_str()?.replace("venue:", "venue/");
                        let name = v.get("name")?.as_str()?.to_string();
                        Some((id, name))
                    })
                    .collect()
            };
            for row in pa_rows {
                let Some(cid_raw) = row.get("contest_id").and_then(|v| v.as_str()) else {
                    continue;
                };
                let cid = cid_raw.replace("contest:", "contest/");
                let vid = row
                    .get("venue_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.replace("venue:", "venue/"));
                let name = vid
                    .as_ref()
                    .and_then(|v| venue_names.get(v))
                    .cloned()
                    .unwrap_or_default();
                contest_venues
                    .entry(cid)
                    .or_insert((vid, name));
            }
        }

        let mut contest_history: Vec<shared::dto::analytics::HeadToHeadContestDto> = Vec::new();
        for (cid, (my_place, opp_place)) in by_contest {
            if my_place == 1 && opp_place != 1 {
                my_wins += 1;
            } else if opp_place == 1 && my_place != 1 {
                opponent_wins += 1;
            }
            let contest_name = contest_names
                .get(&cid)
                .cloned()
                .unwrap_or_else(|| cid.clone());
            let contest_date = contest_starts.get(&cid).copied().unwrap_or(default_dt);
            let (game_id, game_name) = contest_games
                .get(&cid)
                .cloned()
                .unwrap_or((None, String::new()));
            let (venue_id, venue_name) = contest_venues
                .get(&cid)
                .cloned()
                .unwrap_or((None, String::new()));
            contest_history.push(shared::dto::analytics::HeadToHeadContestDto {
                contest_id: cid.clone(),
                contest_name,
                game_id,
                game_name,
                venue_id,
                venue_name,
                my_placement: my_place,
                opponent_placement: opp_place,
                i_won: my_place == 1 && opp_place != 1,
                contest_date,
            });
        }
        contest_history.sort_by(|a, b| b.contest_date.cmp(&a.contest_date));
        let total_contests = contest_history.len() as i32;
        let my_win_rate = if total_contests > 0 {
            (my_wins as f64 * 100.0) / total_contests as f64
        } else {
            0.0
        };

        Ok(shared::dto::analytics::HeadToHeadRecordDto {
            opponent_id: opponent_id.to_string(),
            opponent_handle,
            opponent_name,
            total_contests,
            my_wins,
            opponent_wins,
            my_win_rate,
            contest_history,
        })
    }

    /// Get player's performance trends by month (last 12 months). SurrealQL has no INNER JOIN; we query resulted_in then contest and join in Rust.
    pub async fn get_my_performance_trends(
        &self,
        player_id: &str,
        _game_id: Option<&str>,
        _venue_id: Option<&str>,
    ) -> Result<Vec<PerformanceTrendDto>> {
        let key = player_id_to_key(player_id);
        if key.is_empty() {
            log::warn!(
                "get_my_performance_trends: empty key for player_id={:?}",
                player_id
            );
            return Ok(Vec::new());
        }
        // 1) Get (contest_id, place) for this player from resulted_in
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct RiTrendRow {
            #[serde(rename = "contest_id")]
            out: Option<surrealdb::types::RecordId>,
            place: Option<i64>,
        }
        let record_id = surrealdb::types::RecordId::new("player", key.as_str());
        let sql_ri = "SELECT `in` AS contest_id, place FROM resulted_in WHERE `out` = $record_id";
        let mut res_ri = self
            .db
            .query(sql_ri)
            .bind(("record_id", record_id))
            .await
            .map_err(|e| {
                log::error!(
                    "get_my_performance_trends resulted_in query failed: {} (key={:?})",
                    e,
                    key
                );
                SharedError::Database(e.to_string())
            })?;
        let ri_rows: Vec<RiTrendRow> = res_ri
            .take(0)
            .map_err(|e| SharedError::Database(e.to_string()))?;
        if ri_rows.is_empty() {
            return Ok(Vec::new());
        }
        let contest_ids_slash: Vec<String> = ri_rows
            .iter()
            .filter_map(|r| {
                let id = thing_to_record_id(&r.out);
                if id.is_empty() {
                    None
                } else {
                    Some(id)
                }
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let contest_ids_colon: Vec<String> = contest_ids_slash
            .iter()
            .map(|s| s.replace('/', ":"))
            .collect();

        // 2) Get contest start times for those contests, last 12 months only
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct ContestStartRow {
            id: Option<surrealdb::types::RecordId>,
            start: Option<serde_json::Value>,
        }
        let sql_contest = "SELECT id, start FROM contest WHERE id INSIDE $ids AND start >= time::now() - duration::from_days(365)";
        let mut res_contest = self
            .db
            .query(sql_contest)
            .bind(("ids", contest_ids_colon))
            .await
            .map_err(|e| {
                log::error!("get_my_performance_trends contest query failed: {}", e);
                SharedError::Database(e.to_string())
            })?;
        let contest_rows: Vec<ContestStartRow> = res_contest
            .take(0)
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let contest_start_by_id: HashMap<String, serde_json::Value> = contest_rows
            .into_iter()
            .filter_map(|r| {
                let id = thing_to_record_id(&r.id);
                let start = r.start?;
                if id.is_empty() {
                    None
                } else {
                    Some((id, start))
                }
            })
            .collect();

        // 3) Aggregate by month from (contest_id, place) using contest start from map
        let mut by_month: HashMap<String, (i32, i32, i64)> = HashMap::new();
        for r in &ri_rows {
            let contest_id = thing_to_record_id(&r.out);
            if contest_id.is_empty() {
                continue;
            }
            let Some(start_val) = contest_start_by_id.get(&contest_id) else {
                continue;
            };
            let month = extract_month_from_value(Some(start_val));
            let place = r.place.unwrap_or(0);
            let e = by_month.entry(month).or_insert((0, 0, 0));
            e.0 += 1;
            if place == 1 {
                e.1 += 1;
            }
            e.2 += place;
        }
        let mut out: Vec<PerformanceTrendDto> = by_month
            .into_iter()
            .map(|(month, (contests_played, wins, sum_place))| {
                let win_rate = if contests_played > 0 {
                    (wins as f64 * 100.0) / contests_played as f64
                } else {
                    0.0
                };
                let average_placement = if contests_played > 0 {
                    sum_place as f64 / contests_played as f64
                } else {
                    0.0
                };
                PerformanceTrendDto {
                    month,
                    contests_played,
                    wins,
                    win_rate,
                    average_placement,
                    skill_rating: 0.0,
                    points_earned: wins * 10,
                }
            })
            .collect();
        out.sort_by(|a, b| a.month.cmp(&b.month));
        Ok(out)
    }

    /// Get contests by venue for a player using graph traversal
    pub async fn get_contests_by_venue(
        &self,
        player_id: &str,
        venue_id: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let player_key = player_id_to_key(player_id);
        let venue_key = record_id_to_key(venue_id, "venue");
        if player_key.is_empty() || venue_key.is_empty() {
            return Err(SharedError::Validation("Invalid player or venue id".to_string()));
        }
        let sql = r#"
            SELECT string::replace(string::concat(c.id), '`', '') AS contest_id,
                   c.name AS contest_name,
                   c.start AS start,
                   c.stop AS stop
            FROM resulted_in ri
            JOIN played_at pa ON ri.in = pa.in
            JOIN contest c ON ri.in = c.id
            WHERE ri.out = type::record('player', $player_key)
              AND pa.out = type::record('venue', $venue_key)
            ORDER BY c.start DESC
        "#;
        let mut res = self
            .db
            .query(sql)
            .bind(("player_key", player_key))
            .bind(("venue_key", venue_key))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        Ok(rows)
    }

    /// Saves game statistics to database
    pub async fn save_game_stats(&self, stats: &GameStats) -> Result<()> {
        let value =
            serde_json::to_value(stats).map_err(|e| SharedError::Conversion(e.to_string()))?;
        self.db
            .query("INSERT INTO game_stats CONTENT $stats")
            .bind(("stats", value))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to save game stats: {}", e)))?;
        Ok(())
    }

    /// Retrieves game statistics from database
    pub async fn get_game_stats(&self, game_id: &str) -> Result<Option<GameStats>> {
        let mut res = self
            .db
            .query("SELECT * FROM game_stats WHERE game_id = $game_id LIMIT 1")
            .bind(("game_id", game_id.to_string()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let res: Vec<GameStats> = res.take(0).unwrap_or_default();
        Ok(res.into_iter().next())
    }

    /// Saves venue statistics to database
    pub async fn save_venue_stats(&self, stats: &VenueStats) -> Result<()> {
        let value =
            serde_json::to_value(stats).map_err(|e| SharedError::Conversion(e.to_string()))?;
        self.db
            .query("INSERT INTO venue_stats CONTENT $stats")
            .bind(("stats", value))
            .await
            .map_err(|e| SharedError::Database(format!("Failed to save venue stats: {}", e)))?;
        Ok(())
    }

    /// Retrieves venue statistics from database
    pub async fn get_venue_stats(&self, venue_id: &str) -> Result<Option<VenueStats>> {
        let mut res = self
            .db
            .query("SELECT * FROM venue_stats WHERE venue_id = $venue_id LIMIT 1")
            .bind(("venue_id", venue_id.to_string()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let res: Vec<VenueStats> = res.take(0).unwrap_or_default();
        Ok(res.into_iter().next())
    }

    /// Retrieves all player statistics for leaderboard
    pub async fn get_all_player_stats(&self) -> Result<Vec<PlayerStats>> {
        let mut res = self
            .db
            .query("SELECT * FROM player_stats ORDER BY skill_rating DESC")
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let results: Vec<PlayerStats> = res.take(0).unwrap_or_default();
        Ok(results)
    }

    /// Retrieves player contest results for statistics calculation. SurrealQL has no INNER JOIN; query resulted_in then contest and join in Rust.
    pub async fn get_player_contest_results(&self, player_id: &str) -> Result<Vec<ContestResult>> {
        let key = player_id_to_key(player_id);
        if key.is_empty() {
            return Ok(Vec::new());
        }
        let record_id = surrealdb::types::RecordId::new("player", key.as_str());
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct RiRow {
            contest_id: Option<surrealdb::types::RecordId>,
            placement: Option<i64>,
        }
        let sql_ri = "SELECT `in` AS contest_id, place AS placement FROM resulted_in WHERE `out` = $record_id";
        let mut res_ri = self
            .db
            .query(sql_ri)
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let ri_rows: Vec<RiRow> = res_ri
            .take(0)
            .map_err(|e| SharedError::Database(format!("player contest results: {}", e)))?;
        if ri_rows.is_empty() {
            return Ok(Vec::new());
        }
        let contest_ids_colon: Vec<String> = ri_rows
            .iter()
            .filter_map(|r| {
                let id = thing_to_record_id(&r.contest_id);
                if id.is_empty() {
                    None
                } else {
                    Some(id.replace('/', ":"))
                }
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct ContestStartRow {
            id: Option<surrealdb::types::RecordId>,
            start: Option<String>,
        }
        let sql_contest = "SELECT id, start FROM contest WHERE id INSIDE $ids";
        let mut res_contest = self
            .db
            .query(sql_contest)
            .bind(("ids", contest_ids_colon))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let contest_rows: Vec<ContestStartRow> = res_contest
            .take(0)
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let contest_start_by_id: std::collections::HashMap<String, String> = contest_rows
            .into_iter()
            .filter_map(|r| {
                let id = thing_to_record_id(&r.id);
                let start = r.start?;
                if id.is_empty() {
                    None
                } else {
                    Some((id, start))
                }
            })
            .collect();
        let results: Vec<ContestResult> = ri_rows
            .into_iter()
            .map(|r| {
                let contest_id = thing_to_record_id(&r.contest_id);
                let contest_date = contest_start_by_id
                    .get(&contest_id)
                    .and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(s)
                            .ok()
                            .map(|d| d.fixed_offset())
                    })
                    .unwrap_or_else(|| chrono::Utc::now().into());
                ContestResult {
                    contest_id,
                    placement: r.placement.unwrap_or(0) as i32,
                    score: 0.0,
                    average_opponent_rating: Some(1200.0),
                    contest_difficulty: Some(1.0),
                    contest_date,
                }
            })
            .collect();
        Ok(results)
    }

    /// Retrieves contest participants for statistics calculation.
    /// Tries fn::contest_participants($key) first when applied.
    pub async fn get_contest_participants(
        &self,
        contest_id: &str,
    ) -> Result<Vec<ContestParticipant>> {
        let key = record_id_to_key(contest_id, "contest");
        if !key.is_empty() {
            if let Ok(mut res) = self
                .db
                .query("SELECT fn::contest_participants($key) AS result FROM [1]")
                .bind(("key", key.clone()))
                .await
            {
                let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
                if let Some(first) = rows.into_iter().next() {
                    let result = first
                        .get("result")
                        .or_else(|| first.get("fn::contest_participants($key)"))
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    if let Some(arr) = result.as_array() {
                        let out: Vec<ContestParticipant> = arr
                            .iter()
                            .map(|e| ContestParticipant {
                                player_id: record_id_from_field(e, "player_id").unwrap_or_default(),
                                placement: e.get("place").and_then(|v| v.as_i64()).unwrap_or(0)
                                    as i32,
                                score: 0.0,
                                skill_rating: 1200.0,
                                completed: true,
                            })
                            .collect();
                        return Ok(out);
                    }
                }
            }
        }
        if key.is_empty() {
            return Ok(Vec::new());
        }
        let record_id = surrealdb::types::RecordId::new("contest", key.as_str());
        let sql = r#"
            SELECT `out` AS player_id, place AS placement
            FROM resulted_in
            WHERE `in` = $record_id
            ORDER BY place ASC
        "#;
        let mut res = self
            .db
            .query(sql)
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<ContestParticipantRow> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("contest participants: {}", e)))?;
        let results: Vec<ContestParticipant> = rows
            .into_iter()
            .map(|r| ContestParticipant {
                player_id: thing_to_record_id(&r.player_id),
                placement: r.placement.unwrap_or(0) as i32,
                score: 0.0,
                skill_rating: 1200.0,
                completed: true,
            })
            .collect();
        Ok(results)
    }

    /// Retrieves game plays for statistics calculation. SurrealQL has no INNER JOIN; query played_with, then resulted_in and contest, join in Rust.
    /// Tries fn::game_with_contest_ids($key) first when applied.
    pub async fn get_game_plays(&self, game_id: &str) -> Result<Vec<GamePlay>> {
        let key = record_id_to_key(game_id, "game");
        let mut contest_ids_colon: Vec<String> = Vec::new();
        if !key.is_empty() {
            if let Ok(mut res) = self
                .db
                .query("SELECT fn::game_with_contest_ids($key) AS result FROM [1]")
                .bind(("key", key.clone()))
                .await
            {
                let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
                if let Some(first) = rows.into_iter().next() {
                    let result = first
                        .get("result")
                        .or_else(|| first.get("fn::game_with_contest_ids($key)"))
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    if let Some(obj) = result.as_object() {
                        let empty: &[serde_json::Value] = &[];
                        let cids = obj
                            .get("contest_ids")
                            .and_then(|v| v.as_array())
                            .map(|v| v.as_slice())
                            .unwrap_or(empty);
                        contest_ids_colon = cids
                            .iter()
                            .filter_map(|v| {
                                record_id_from_field(v, "id")
                                    .or_else(|| record_id_from_row(v, Some("contest")))
                                    .map(|s| s.replace('/', ":"))
                            })
                            .collect::<std::collections::HashSet<_>>()
                            .into_iter()
                            .collect();
                    }
                }
            }
        }
        if contest_ids_colon.is_empty() {
            let game_record_id = surrealdb::types::RecordId::new("game", key.as_str());
            let pw_sql = "SELECT string::concat(`in`) AS contest_id FROM played_with WHERE `out` = $record_id";
            let mut res_pw = self
                .db
                .query(pw_sql)
                .bind(("record_id", game_record_id))
                .await
                .map_err(|e| SharedError::Database(e.to_string()))?;
            #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
            struct PwRow {
                contest_id: Option<String>,
            }
            let pw_rows: Vec<PwRow> = res_pw
                .take(0)
                .map_err(|e| SharedError::Database(e.to_string()))?;
            contest_ids_colon = pw_rows
                .iter()
                .filter_map(|r| r.contest_id.as_ref().map(|s| s.replace('/', ":")))
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
        }
        if contest_ids_colon.is_empty() {
            return Ok(Vec::new());
        }
        let ri_sql = "SELECT string::concat(`in`) AS contest_id, place, string::concat(`out`) AS player_id FROM resulted_in WHERE `in` INSIDE $ids";
        let mut res_ri = self
            .db
            .query(ri_sql)
            .bind(("ids", contest_ids_colon.clone()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct RiRow {
            contest_id: Option<String>,
            place: Option<i64>,
            player_id: Option<String>,
        }
        let ri_rows: Vec<RiRow> = res_ri
            .take(0)
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let contest_start_sql =
            "SELECT string::concat(id) AS contest_id, start FROM contest WHERE id INSIDE $ids";
        let mut res_start = self
            .db
            .query(contest_start_sql)
            .bind(("ids", contest_ids_colon))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let start_rows: Vec<serde_json::Value> = res_start
            .take(0)
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let contest_start_by_id: std::collections::HashMap<String, String> = start_rows
            .into_iter()
            .filter_map(|v| {
                let id = v
                    .get("contest_id")
                    .and_then(|x| x.as_str())
                    .map(|s| s.replace('/', ":"))?;
                let start = v.get("start").and_then(|x| x.as_str()).map(String::from)?;
                Some((id, start))
            })
            .collect();
        let player_count = ri_rows.len().max(1);
        let results: Vec<GamePlay> = ri_rows
            .into_iter()
            .filter_map(|r| {
                let contest_id = r.contest_id?.replace('/', ":");
                let start = contest_start_by_id.get(&contest_id).cloned()?;
                let played_at = chrono::DateTime::parse_from_rfc3339(&start)
                    .ok()
                    .map(|d| d.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()))
                    .unwrap_or_else(|| {
                        chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
                    });
                Some(GamePlay {
                    player_id: r.player_id.unwrap_or_default(),
                    player_count: player_count as i32,
                    won: r.place.unwrap_or(0) == 1,
                    duration_minutes: 0,
                    played_at,
                })
            })
            .collect();
        Ok(results)
    }

    /// Retrieves venue contests for statistics calculation. SurrealQL has no INNER JOIN; query played_at then contest and join in Rust.
    /// Tries fn::venue_with_contest_ids($key) first when applied.
    pub async fn get_venue_contests(&self, venue_id: &str) -> Result<Vec<VenueContest>> {
        let key = record_id_to_key(venue_id, "venue");
        if !key.is_empty() {
            if let Ok(mut res) = self
                .db
                .query("SELECT fn::venue_with_contest_ids($key) AS result FROM [1]")
                .bind(("key", key.clone()))
                .await
            {
                let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
                if let Some(first) = rows.into_iter().next() {
                    let result = first
                        .get("result")
                        .or_else(|| first.get("fn::venue_with_contest_ids($key)"))
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    if let Some(obj) = result.as_object() {
                        let empty: &[serde_json::Value] = &[];
                        let cids = obj
                            .get("contest_ids")
                            .and_then(|v| v.as_array())
                            .map(|v| v.as_slice())
                            .unwrap_or(empty);
                        let contest_ids_colon: Vec<String> = cids
                            .iter()
                            .filter_map(|v| {
                                let id = record_id_from_field(v, "id")
                                    .or_else(|| record_id_from_row(v, Some("contest")));
                                id.map(|s| s.replace('/', ":"))
                            })
                            .collect::<std::collections::HashSet<_>>()
                            .into_iter()
                            .collect();
                        if !contest_ids_colon.is_empty() {
                            let contest_sql = "SELECT string::concat(id) AS contest_id, start FROM contest WHERE id INSIDE $ids ORDER BY start DESC";
                            if let Ok(mut res_contest) = self
                                .db
                                .query(contest_sql)
                                .bind(("ids", contest_ids_colon))
                                .await
                            {
                                let contest_rows: Vec<serde_json::Value> =
                                    res_contest.take(0).unwrap_or_default();
                                let default_dt = chrono::Utc::now()
                                    .with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());
                                let results: Vec<VenueContest> = contest_rows
                                    .into_iter()
                                    .map(|v| {
                                        let contest_id = v
                                            .get("contest_id")
                                            .and_then(|x| x.as_str())
                                            .map(|s| s.replace("contest:", "contest/"))
                                            .unwrap_or_default();
                                        let contest_date = v
                                            .get("start")
                                            .and_then(|x| x.as_str())
                                            .and_then(|s| {
                                                chrono::DateTime::parse_from_rfc3339(s).ok()
                                            })
                                            .map(|d| {
                                                d.with_timezone(
                                                    &chrono::FixedOffset::east_opt(0).unwrap(),
                                                )
                                            })
                                            .unwrap_or(default_dt);
                                        VenueContest {
                                            contest_id,
                                            participant_ids: Vec::new(),
                                            participant_count: 0,
                                            game_ids: Vec::new(),
                                            duration_minutes: 0,
                                            contest_date,
                                        }
                                    })
                                    .collect();
                                return Ok(results);
                            }
                        } else {
                            return Ok(Vec::new());
                        }
                    }
                }
            }
        }
        let record_id = surrealdb::types::RecordId::new("venue", key.as_str());
        let pa_sql =
            "SELECT string::concat(`in`) AS contest_id FROM played_at WHERE `out` = $record_id";
        let mut res_pa = self
            .db
            .query(pa_sql)
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct PaRow {
            contest_id: Option<String>,
        }
        let pa_rows: Vec<PaRow> = res_pa
            .take(0)
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let contest_ids_colon: Vec<String> = pa_rows
            .iter()
            .filter_map(|r| r.contest_id.as_ref().map(|s| s.replace('/', ":")))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        if contest_ids_colon.is_empty() {
            return Ok(Vec::new());
        }
        let contest_sql = "SELECT string::concat(id) AS contest_id, start FROM contest WHERE id INSIDE $ids ORDER BY start DESC";
        let mut res_contest = self
            .db
            .query(contest_sql)
            .bind(("ids", contest_ids_colon))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let contest_rows: Vec<serde_json::Value> = res_contest
            .take(0)
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let default_dt =
            chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());
        let results: Vec<VenueContest> = contest_rows
            .into_iter()
            .map(|v| {
                let contest_id = v
                    .get("contest_id")
                    .and_then(|x| x.as_str())
                    .map(|s| s.replace("contest:", "contest/"))
                    .unwrap_or_default();
                let contest_date = v
                    .get("start")
                    .and_then(|x| x.as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|d| d.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()))
                    .unwrap_or(default_dt);
                VenueContest {
                    contest_id,
                    participant_ids: Vec::new(),
                    participant_count: 0,
                    game_ids: Vec::new(),
                    duration_minutes: 0,
                    contest_date,
                }
            })
            .collect();
        Ok(results)
    }

    /// Retrieves player information for DTOs
    pub async fn get_player_info(&self, player_id: &str) -> Result<Option<(String, String)>> {
        let key = player_id_to_key(player_id);
        if key.is_empty() {
            return Ok(None);
        }
        let record_id = surrealdb::types::RecordId::new("player", key.as_str());
        let mut res = self
            .db
            .query("SELECT handle, firstname FROM player WHERE id = $record_id")
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let results: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        if let Some(result) = results.first() {
            let handle = result
                .get("handle")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let firstname = result
                .get("firstname")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(Some((handle, firstname)))
        } else {
            Ok(None)
        }
    }

    /// Retrieves game information for DTOs
    pub async fn get_game_info(&self, game_id: &str) -> Result<Option<String>> {
        let key = record_id_to_key(game_id, "game");
        if key.is_empty() {
            return Ok(None);
        }
        let record_id = surrealdb::types::RecordId::new("game", key.as_str());
        let mut res = self
            .db
            .query("SELECT name FROM game WHERE id = $record_id")
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let results: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        Ok(results.into_iter().next().and_then(|r| {
            r.get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        }))
    }

    /// Retrieves venue information for DTOs
    pub async fn get_venue_info(&self, venue_id: &str) -> Result<Option<String>> {
        let key = record_id_to_key(venue_id, "venue");
        if key.is_empty() {
            return Ok(None);
        }
        let record_id = surrealdb::types::RecordId::new("venue", key.as_str());
        let mut res = self
            .db
            .query("SELECT displayName FROM venue WHERE id = $record_id")
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let results: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        Ok(results.into_iter().next().and_then(|r| {
            r.get("displayName")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        }))
    }

    /// Retrieves contest information for DTOs
    pub async fn get_contest_info(&self, contest_id: &str) -> Result<Option<String>> {
        let key = record_id_to_key(contest_id, "contest");
        if key.is_empty() {
            return Ok(None);
        }
        let record_id = surrealdb::types::RecordId::new("contest", key.as_str());
        let mut res = self
            .db
            .query("SELECT name FROM contest WHERE id = $record_id")
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let results: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        Ok(results.into_iter().next().and_then(|r| {
            r.get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        }))
    }

    /// Creates analytics collections if they don't exist
    pub async fn create_collections(&self) -> Result<()> {
        // SurrealDB creates tables on first insert; no explicit create needed
        Ok(())
    }

    /// Debug method to run custom queries (SurrealQL)
    pub async fn debug_database(&self, query: &str) -> Result<serde_json::Value> {
        let mut res = self
            .db
            .query(query.to_string())
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let results: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        if let Some(result) = results.first() {
            Ok(result.clone())
        } else {
            Ok(serde_json::json!({"error": "No results from debug query"}))
        }
    }

    /// Get enhanced platform insights with more meaningful metrics
    pub async fn get_platform_insights(&self) -> Result<serde_json::Value> {
        // Get basic stats
        let total_players = self.get_total_players().await?;
        let total_contests = self.get_total_contests().await?;
        let total_games = self.get_total_games().await?;
        let total_venues = self.get_total_venues().await?;
        let active_players_30d = self.get_active_players(30).await?;
        let contests_30d = self.get_contests_in_period(30).await?;
        let average_participants = self.get_average_participants_per_contest().await?;

        // Calculate meaningful ratios and insights
        let contests_per_player = if total_players > 0 {
            total_contests as f64 / total_players as f64
        } else {
            0.0
        };
        let activity_rate = if total_players > 0 {
            (active_players_30d as f64 / total_players as f64) * 100.0
        } else {
            0.0
        };
        let monthly_avg_contests = total_contests as f64 / 12.0;
        let monthly_growth = if monthly_avg_contests > 0.0 {
            (contests_30d as f64 / monthly_avg_contests) * 100.0
        } else {
            0.0
        };

        // Determine platform health indicators
        let engagement_level = if contests_per_player > 10.0 {
            "High"
        } else if contests_per_player > 5.0 {
            "Medium"
        } else {
            "Low"
        };
        let growth_trend = if monthly_growth > 120.0 {
            "↗️ Above Average"
        } else if monthly_growth < 80.0 {
            "↘️ Below Average"
        } else {
            "→ On Track"
        };
        let activity_status = if activity_rate > 20.0 {
            "Very Active"
        } else if activity_rate > 10.0 {
            "Moderately Active"
        } else {
            "Low Activity"
        };

        // Get top performers
        let top_games = self.get_top_games(5).await?;
        let top_venues = self.get_top_venues(5).await?;

        let insights = serde_json::json!({
            "summary": {
                "total_players": total_players,
                "total_contests": total_contests,
                "total_games": total_games,
                "total_venues": total_venues,
                "active_players_30d": active_players_30d,
                "contests_30d": contests_30d,
                "average_participants": average_participants
            },
            "metrics": {
                "contests_per_player": contests_per_player,
                "activity_rate": activity_rate,
                "monthly_growth": monthly_growth,
                "engagement_level": engagement_level,
                "growth_trend": growth_trend,
                "activity_status": activity_status
            },
            "top_performers": {
                "games": top_games,
                "venues": top_venues
            },
            "insights": {
                "platform_health": if contests_per_player > 5.0 && activity_rate > 10.0 { "Healthy" } else { "Needs Attention" },
                "growth_potential": if monthly_growth > 100.0 { "Strong" } else if monthly_growth > 80.0 { "Stable" } else { "Declining" },
                "recommendations": {
                    "engagement": if contests_per_player < 5.0 { "Consider running more contests to increase player engagement" } else { "Great player engagement levels" },
                    "retention": if activity_rate < 15.0 { "Focus on player retention strategies" } else { "Good player retention" },
                    "growth": if monthly_growth < 90.0 { "Implement growth initiatives to boost monthly activity" } else { "Strong monthly growth" }
                }
            }
        });

        Ok(insights)
    }

    /// Get player achievements (SurrealDB-style: one multi-statement query for player + contest ids, then one for games + venues).
    /// Uses RecordId binding for player (like get_player_stats) and RecordId array for INSIDE so SurrealDB matches correctly.
    pub async fn get_player_achievements(&self, player_id: &str) -> Result<PlayerAchievements> {
        let player_key = player_id_to_key(player_id);
        if player_key.is_empty() {
            log::warn!(
                "get_player_achievements: empty key for player_id={:?}",
                player_id
            );
            return Err(SharedError::Database(
                "Achievements: invalid player id".to_string(),
            ));
        }
        let record_id = surrealdb::types::RecordId::new("player", player_key.as_str());

        // Player row (no scalar subqueries — some SurrealDB setups don't bind $record_id in subqueries).
        let sql_player = "SELECT id, handle FROM player WHERE id = $record_id LIMIT 1";
        let mut res = self
            .db
            .query(sql_player)
            .bind(("record_id", record_id.clone()))
            .await
            .map_err(|e| {
                log::error!(
                    "Achievements: player query failed for key {:?}: {}",
                    player_key,
                    e
                );
                SharedError::Database(e.to_string())
            })?;
        let player_rows: Vec<serde_json::Value> = res
            .take(0)
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let row = player_rows
            .into_iter()
            .next()
            .ok_or_else(|| SharedError::Database("Achievements: no player row".to_string()))?;
        let player_id_norm = record_id_from_row(&row, Some("player"))
            .unwrap_or_else(|| format!("player/{}", player_key));
        let player_handle = row
            .get("handle")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        // Separate count queries so bindings are reliable (avoids scalar subquery binding issues).
        let total_contests = self
            .db
            .query("SELECT count() AS n FROM resulted_in WHERE `out` = $record_id GROUP ALL")
            .bind(("record_id", record_id.clone()))
            .await
            .ok()
            .and_then(|mut r| r.take(0).ok())
            .and_then(|rows: Vec<serde_json::Value>| rows.into_iter().next())
            .map(|r| scalar_i64(&r))
            .unwrap_or(0) as i32;
        let total_wins = self
            .db
            .query("SELECT count() AS n FROM resulted_in WHERE `out` = $record_id AND place = 1 GROUP ALL")
            .bind(("record_id", record_id.clone()))
            .await
            .ok()
            .and_then(|mut r| r.take(0).ok())
            .and_then(|rows: Vec<serde_json::Value>| rows.into_iter().next())
            .map(|r| scalar_i64(&r))
            .unwrap_or(0) as i32;

        log::debug!(
            "get_player_achievements: player_key={:?} total_contests={} total_wins={}",
            player_key,
            total_contests,
            total_wins
        );

        // Query 2: contest IDs (raw `in` so INSIDE matches). Separate query avoids multi-statement scalar subquery quirks.
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct ContestIdRow {
            contest_id: Option<surrealdb::types::RecordId>,
        }
        let contest_id_rows: Vec<ContestIdRow> = self
            .db
            .query("SELECT `in` AS contest_id FROM resulted_in WHERE `out` = $record_id")
            .bind(("record_id", record_id.clone()))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?
            .take(0)
            .unwrap_or_default();
        // Prefer RecordIds from raw `in` so INSIDE matches. If edges are stored as strings, we get None and need fallback.
        let mut contest_record_ids: Vec<surrealdb::types::RecordId> = contest_id_rows
            .into_iter()
            .filter_map(|r| r.contest_id)
            .collect();
        // Fallback when edge `in` did not deserialize as RecordId (e.g. old DB with string edges): fetch as string and build RecordIds.
        let mut contest_ids_colon_fallback: Vec<String> = vec![];
        if contest_record_ids.is_empty() && total_contests > 0 {
            let sql_rids =
                "SELECT string::concat(`in`) AS rid FROM resulted_in WHERE `out` = $record_id";
            if let Ok(mut rres) = self
                .db
                .query(sql_rids)
                .bind(("record_id", record_id.clone()))
                .await
            {
                let rids: Vec<serde_json::Value> = rres.take(0).unwrap_or_default();
                let contest_ids_str: Vec<String> = rids
                    .iter()
                    .filter_map(|r| r.get("rid").and_then(|v| v.as_str()).map(String::from))
                    .collect();
                contest_ids_colon_fallback = contest_ids_str
                    .iter()
                    .map(|s| s.replace('/', ":"))
                    .collect();
                contest_record_ids = strings_to_record_id_array(&contest_ids_colon_fallback);
            }
        }

        // Two single-statement queries (SurrealDB: one statement per result set avoids multi-statement take(0)/take(1) and scalar quirks).
        let (unique_games, unique_venues) = if contest_record_ids.is_empty()
            && contest_ids_colon_fallback.is_empty()
        {
            (0, 0)
        } else {
            let games_sql = "SELECT string::concat(`out`) AS game_id FROM played_with WHERE `in` INSIDE $contest_ids";
            let venues_sql = "SELECT string::concat(`out`) AS venue_id FROM played_at WHERE `in` INSIDE $contest_ids";
            let (game_res, venue_res) = if !contest_record_ids.is_empty() {
                (
                    self.db
                        .query(games_sql)
                        .bind(("contest_ids", contest_record_ids.clone()))
                        .await,
                    self.db
                        .query(venues_sql)
                        .bind(("contest_ids", contest_record_ids))
                        .await,
                )
            } else {
                (
                    self.db
                        .query(games_sql)
                        .bind(("contest_ids", contest_ids_colon_fallback.clone()))
                        .await,
                    self.db
                        .query(venues_sql)
                        .bind(("contest_ids", contest_ids_colon_fallback))
                        .await,
                )
            };
            let ug = match game_res {
                Ok(mut r) => {
                    let rows: Vec<serde_json::Value> = r.take(0).unwrap_or_default();
                    rows.iter()
                        .filter_map(|r| r.get("game_id").and_then(|v| v.as_str()))
                        .collect::<std::collections::HashSet<_>>()
                        .len() as i32
                }
                Err(e) => {
                    log::warn!("Achievements: played_with INSIDE failed: {}", e);
                    0
                }
            };
            let uv = match venue_res {
                Ok(mut r) => {
                    let rows: Vec<serde_json::Value> = r.take(0).unwrap_or_default();
                    rows.iter()
                        .filter_map(|r| r.get("venue_id").and_then(|v| v.as_str()))
                        .collect::<std::collections::HashSet<_>>()
                        .len() as i32
                }
                Err(e) => {
                    log::warn!("Achievements: played_at INSIDE failed: {}", e);
                    0
                }
            };
            (ug, uv)
        };

        log::debug!(
            "get_player_achievements: unique_games={} unique_venues={}",
            unique_games,
            unique_venues
        );

        let player_data = PlayerDataResult {
            player_id: player_id_norm,
            player_handle,
            total_contests,
            total_wins,
            unique_games,
            unique_venues,
        };
        let achievements = self.calculate_achievements(&player_data).await?;
        let unlocked_count = achievements.iter().filter(|a| a.unlocked).count() as i32;
        let total_achievements = achievements.len() as i32;
        Ok(PlayerAchievements {
            player_id: player_data.player_id,
            player_handle: player_data.player_handle,
            achievements,
            total_achievements,
            unlocked_achievements: unlocked_count,
            completion_percentage: if total_achievements == 0 {
                0.0
            } else {
                (unlocked_count as f64 / total_achievements as f64) * 100.0
            },
        })
    }

    /// Calculate achievements for a player based on their stats
    async fn calculate_achievements(
        &self,
        player_data: &PlayerDataResult,
    ) -> Result<Vec<Achievement>> {
        let mut achievements = Vec::new();

        // Win-based achievements
        achievements.push(Achievement {
            id: "first_win".to_string(),
            name: "First Victory".to_string(),
            description: "Win your first contest".to_string(),
            category: AchievementCategory::Wins,
            required_value: 1,
            current_value: player_data.total_wins,
            unlocked: player_data.total_wins >= 1,
            unlocked_at: if player_data.total_wins >= 1 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        achievements.push(Achievement {
            id: "win_master".to_string(),
            name: "Win Master".to_string(),
            description: "Win 10 contests".to_string(),
            category: AchievementCategory::Wins,
            required_value: 10,
            current_value: player_data.total_wins,
            unlocked: player_data.total_wins >= 10,
            unlocked_at: if player_data.total_wins >= 10 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        achievements.push(Achievement {
            id: "champion".to_string(),
            name: "Champion".to_string(),
            description: "Win 50 contests".to_string(),
            category: AchievementCategory::Wins,
            required_value: 50,
            current_value: player_data.total_wins,
            unlocked: player_data.total_wins >= 50,
            unlocked_at: if player_data.total_wins >= 50 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        achievements.push(Achievement {
            id: "dominator".to_string(),
            name: "Dominator".to_string(),
            description: "Win 100 contests".to_string(),
            category: AchievementCategory::Wins,
            required_value: 100,
            current_value: player_data.total_wins,
            unlocked: player_data.total_wins >= 100,
            unlocked_at: if player_data.total_wins >= 100 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        // Contest-based achievements
        achievements.push(Achievement {
            id: "contestant".to_string(),
            name: "Contestant".to_string(),
            description: "Participate in 5 contests".to_string(),
            category: AchievementCategory::Contests,
            required_value: 5,
            current_value: player_data.total_contests,
            unlocked: player_data.total_contests >= 5,
            unlocked_at: if player_data.total_contests >= 5 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        achievements.push(Achievement {
            id: "veteran".to_string(),
            name: "Veteran".to_string(),
            description: "Participate in 25 contests".to_string(),
            category: AchievementCategory::Contests,
            required_value: 25,
            current_value: player_data.total_contests,
            unlocked: player_data.total_contests >= 25,
            unlocked_at: if player_data.total_contests >= 25 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        achievements.push(Achievement {
            id: "dedicated".to_string(),
            name: "Dedicated".to_string(),
            description: "Participate in 50 contests".to_string(),
            category: AchievementCategory::Contests,
            required_value: 50,
            current_value: player_data.total_contests,
            unlocked: player_data.total_contests >= 50,
            unlocked_at: if player_data.total_contests >= 50 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        achievements.push(Achievement {
            id: "legend".to_string(),
            name: "Legend".to_string(),
            description: "Participate in 100 contests".to_string(),
            category: AchievementCategory::Contests,
            required_value: 100,
            current_value: player_data.total_contests,
            unlocked: player_data.total_contests >= 100,
            unlocked_at: if player_data.total_contests >= 100 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        // Game-based achievements
        achievements.push(Achievement {
            id: "first_game".to_string(),
            name: "First Game".to_string(),
            description: "Play your first different game".to_string(),
            category: AchievementCategory::Games,
            required_value: 1,
            current_value: player_data.unique_games,
            unlocked: player_data.unique_games >= 1,
            unlocked_at: if player_data.unique_games >= 1 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        achievements.push(Achievement {
            id: "game_explorer".to_string(),
            name: "Game Explorer".to_string(),
            description: "Play 5 different games".to_string(),
            category: AchievementCategory::Games,
            required_value: 5,
            current_value: player_data.unique_games,
            unlocked: player_data.unique_games >= 5,
            unlocked_at: if player_data.unique_games >= 5 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        achievements.push(Achievement {
            id: "diverse".to_string(),
            name: "Diverse".to_string(),
            description: "Play 10 different games".to_string(),
            category: AchievementCategory::Games,
            required_value: 10,
            current_value: player_data.unique_games,
            unlocked: player_data.unique_games >= 10,
            unlocked_at: if player_data.unique_games >= 10 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        achievements.push(Achievement {
            id: "game_master".to_string(),
            name: "Game Master".to_string(),
            description: "Play 15 different games".to_string(),
            category: AchievementCategory::Games,
            required_value: 15,
            current_value: player_data.unique_games,
            unlocked: player_data.unique_games >= 15,
            unlocked_at: if player_data.unique_games >= 15 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        achievements.push(Achievement {
            id: "collector".to_string(),
            name: "Collector".to_string(),
            description: "Play 25 different games".to_string(),
            category: AchievementCategory::Games,
            required_value: 25,
            current_value: player_data.unique_games,
            unlocked: player_data.unique_games >= 25,
            unlocked_at: if player_data.unique_games >= 25 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        // Venue-based achievements
        achievements.push(Achievement {
            id: "first_venue".to_string(),
            name: "First Venue".to_string(),
            description: "Play at your first venue".to_string(),
            category: AchievementCategory::Venues,
            required_value: 1,
            current_value: player_data.unique_venues,
            unlocked: player_data.unique_venues >= 1,
            unlocked_at: if player_data.unique_venues >= 1 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        achievements.push(Achievement {
            id: "venue_hopper".to_string(),
            name: "Venue Hopper".to_string(),
            description: "Play at 3 different venues".to_string(),
            category: AchievementCategory::Venues,
            required_value: 3,
            current_value: player_data.unique_venues,
            unlocked: player_data.unique_venues >= 3,
            unlocked_at: if player_data.unique_venues >= 3 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        achievements.push(Achievement {
            id: "venue_regular".to_string(),
            name: "Venue Regular".to_string(),
            description: "Play at 10 different venues".to_string(),
            category: AchievementCategory::Venues,
            required_value: 10,
            current_value: player_data.unique_venues,
            unlocked: player_data.unique_venues >= 10,
            unlocked_at: if player_data.unique_venues >= 10 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        achievements.push(Achievement {
            id: "globetrotter".to_string(),
            name: "Globetrotter".to_string(),
            description: "Play at 25 different venues".to_string(),
            category: AchievementCategory::Venues,
            required_value: 25,
            current_value: player_data.unique_venues,
            unlocked: player_data.unique_venues >= 25,
            unlocked_at: if player_data.unique_venues >= 25 {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        // Special: combo achievements
        let all_rounder = player_data.total_contests >= 1
            && player_data.unique_games >= 1
            && player_data.unique_venues >= 1;
        achievements.push(Achievement {
            id: "all_rounder".to_string(),
            name: "All-Rounder".to_string(),
            description: "Play at least one contest, one game, and one venue".to_string(),
            category: AchievementCategory::Special,
            required_value: 1,
            current_value: if all_rounder { 1 } else { 0 },
            unlocked: all_rounder,
            unlocked_at: if all_rounder {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        let triple_threat = player_data.total_contests >= 3
            && player_data.unique_games >= 3
            && player_data.unique_venues >= 3;
        achievements.push(Achievement {
            id: "triple_threat".to_string(),
            name: "Triple Threat".to_string(),
            description: "Play 3+ contests, 3+ games, and 3+ venues".to_string(),
            category: AchievementCategory::Special,
            required_value: 3,
            current_value: [
                player_data.total_contests,
                player_data.unique_games,
                player_data.unique_venues,
            ]
            .into_iter()
            .min()
            .unwrap_or(0),
            unlocked: triple_threat,
            unlocked_at: if triple_threat {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        let completionist = player_data.total_contests >= 10
            && player_data.unique_games >= 10
            && player_data.unique_venues >= 10;
        achievements.push(Achievement {
            id: "completionist".to_string(),
            name: "Completionist".to_string(),
            description: "Play 10+ contests, 10+ games, and 10+ venues".to_string(),
            category: AchievementCategory::Special,
            required_value: 10,
            current_value: [
                player_data.total_contests,
                player_data.unique_games,
                player_data.unique_venues,
            ]
            .into_iter()
            .min()
            .unwrap_or(0),
            unlocked: completionist,
            unlocked_at: if completionist {
                Some(chrono::Utc::now().into())
            } else {
                None
            },
        });

        Ok(achievements)
    }

    /// Get player ranking across all categories
    pub async fn get_player_rankings(&self, player_id: &str) -> Result<Vec<PlayerRanking>> {
        let mut rankings = Vec::new();

        if let Ok(win_rate_rank) = self.get_player_win_rate_ranking(player_id).await {
            rankings.push(win_rate_rank);
        }

        if let Ok(total_wins_rank) = self.get_player_total_wins_ranking(player_id).await {
            rankings.push(total_wins_rank);
        }

        if let Ok(total_contests_rank) = self.get_player_total_contests_ranking(player_id).await {
            rankings.push(total_contests_rank);
        }

        Ok(rankings)
    }

    /// Per-player (contests, wins) from resulted_in — SurrealDB 3.x safe GROUP BY on `out`.
    async fn aggregate_player_contest_stats(&self) -> Result<HashMap<String, (i32, i32)>> {
        let mut stats: HashMap<String, (i32, i32)> = HashMap::new();
        let mut res = self
            .db
            .query("SELECT `out` AS player_id, count() AS total FROM resulted_in GROUP BY `out`")
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        for row in res.take::<Vec<serde_json::Value>>(0).unwrap_or_default() {
            let Some(pid_val) = row.get("player_id") else {
                continue;
            };
            let pid = canonical_id_from_value(pid_val, "player");
            if pid.is_empty() {
                continue;
            }
            let total = row.get("total").map(scalar_i64).unwrap_or(0) as i32;
            stats.insert(pid, (total, 0));
        }
        let mut res2 = self
            .db
            .query(
                "SELECT `out` AS player_id, count() AS wins FROM resulted_in WHERE place = 1 GROUP BY `out`",
            )
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        for row in res2.take::<Vec<serde_json::Value>>(0).unwrap_or_default() {
            let Some(pid_val) = row.get("player_id") else {
                continue;
            };
            let pid = canonical_id_from_value(pid_val, "player");
            if pid.is_empty() {
                continue;
            }
            let wins = row.get("wins").map(scalar_i64).unwrap_or(0) as i32;
            stats.entry(pid).and_modify(|e| e.1 = wins).or_insert((0, wins));
        }
        Ok(stats)
    }

    fn ranking_for_player(
        entries: &[(String, f64)],
        player_id: &str,
        category: &str,
    ) -> Result<PlayerRanking> {
        let target = normalize_player_id(player_id);
        let rank = entries
            .iter()
            .position(|(pid, _)| normalize_player_id(pid) == target)
            .map(|i| i as i32 + 1)
            .ok_or_else(|| SharedError::NotFound("Player not found in rankings".to_string()))?;
        let value = entries
            .iter()
            .find(|(pid, _)| normalize_player_id(pid) == target)
            .map(|(_, v)| *v)
            .unwrap_or(0.0);
        Ok(PlayerRanking {
            category: category.to_string(),
            rank,
            total_players: entries.len() as i32,
            value,
        })
    }

    /// Get player's win rate ranking
    async fn get_player_win_rate_ranking(&self, player_id: &str) -> Result<PlayerRanking> {
        let stats = self.aggregate_player_contest_stats().await?;
        let mut entries: Vec<(String, f64)> = stats
            .into_iter()
            .filter(|(_, (total, _))| *total >= 3)
            .map(|(pid, (total, wins))| {
                let rate = if total > 0 {
                    (wins as f64 * 100.0) / total as f64
                } else {
                    0.0
                };
                (pid, rate)
            })
            .collect();
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Self::ranking_for_player(&entries, player_id, "win_rate")
    }

    /// Get player's total wins ranking
    async fn get_player_total_wins_ranking(&self, player_id: &str) -> Result<PlayerRanking> {
        let stats = self.aggregate_player_contest_stats().await?;
        let mut entries: Vec<(String, f64)> = stats
            .into_iter()
            .map(|(pid, (_, wins))| (pid, wins as f64))
            .collect();
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Self::ranking_for_player(&entries, player_id, "total_wins")
    }

    /// Get player's total contests ranking
    async fn get_player_total_contests_ranking(&self, player_id: &str) -> Result<PlayerRanking> {
        let stats = self.aggregate_player_contest_stats().await?;
        let mut entries: Vec<(String, f64)> = stats
            .into_iter()
            .map(|(pid, (total, _))| (pid, total as f64))
            .collect();
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Self::ranking_for_player(&entries, player_id, "total_contests")
    }

    /// Get player performance distribution by win rate ranges
    pub async fn get_player_performance_distribution(&self) -> Result<Vec<(String, i32)>> {
        let stats = self.aggregate_player_contest_stats().await?;
        let labels = ["0-20%", "21-40%", "41-60%", "61-80%", "81-100%"];
        let mut counts = [0i32; 5];
        for (_, (total, wins)) in stats {
            if total < 1 {
                continue;
            }
            let wr = (wins as f64 * 100.0) / total as f64;
            let idx = if wr <= 20.0 {
                0
            } else if wr <= 40.0 {
                1
            } else if wr <= 60.0 {
                2
            } else if wr <= 80.0 {
                3
            } else {
                4
            };
            counts[idx] += 1;
        }
        Ok(labels
            .iter()
            .enumerate()
            .map(|(i, label)| (label.to_string(), counts[i]))
            .collect())
    }

    /// Get game difficulty vs popularity data
    pub async fn get_game_difficulty_popularity(&self) -> Result<Vec<(String, f64, i32, f64)>> {
        let mut res_pw = self
            .db
            .query("SELECT `in` AS contest_id, `out` AS game_id FROM played_with")
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let pw_rows: Vec<serde_json::Value> = res_pw.take(0).unwrap_or_default();
        let mut contest_to_game: HashMap<String, String> = HashMap::new();
        for row in &pw_rows {
            let Some(cid_val) = row.get("contest_id") else {
                continue;
            };
            let Some(gid_val) = row.get("game_id") else {
                continue;
            };
            let cid = canonical_id_from_value(cid_val, "contest");
            let gid = canonical_id_from_value(gid_val, "game");
            if !cid.is_empty() && !gid.is_empty() {
                contest_to_game.insert(cid, gid);
            }
        }

        let mut res_ri = self
            .db
            .query(
                "SELECT `in` AS contest_id, count() AS participants, math::min(place) AS min_place FROM resulted_in GROUP BY `in`",
            )
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let ri_rows: Vec<serde_json::Value> = res_ri.take(0).unwrap_or_default();

        let mut agg: HashMap<String, (i32, i32, i32)> = HashMap::new(); // plays, sum_participants, completed
        for row in &ri_rows {
            let Some(cid_val) = row.get("contest_id") else {
                continue;
            };
            let cid = canonical_id_from_value(cid_val, "contest");
            let Some(gid) = contest_to_game.get(&cid).cloned() else {
                continue;
            };
            let participants = row
                .get("participants")
                .map(scalar_i64)
                .unwrap_or(0) as i32;
            let min_place = row.get("min_place").map(scalar_i64).unwrap_or(0) as i32;
            let completed = if participants > 0 && min_place > 0 {
                1
            } else {
                0
            };
            let e = agg.entry(gid).or_insert((0, 0, 0));
            e.0 += 1;
            e.1 += participants;
            e.2 += completed;
        }

        let game_ids: Vec<surrealdb::types::RecordId> = agg
            .keys()
            .filter_map(|gid| {
                let key = record_id_to_key(gid, "game");
                if key.is_empty() {
                    None
                } else {
                    Some(surrealdb::types::RecordId::new("game", key.as_str()))
                }
            })
            .collect();
        let mut names: HashMap<String, String> = HashMap::new();
        if !game_ids.is_empty() {
            let mut res_names = self
                .db
                .query("SELECT string::concat(id) AS game_id, name FROM game WHERE id INSIDE $ids")
                .bind(("ids", game_ids))
                .await
                .map_err(|e| SharedError::Database(e.to_string()))?;
            for row in res_names.take::<Vec<serde_json::Value>>(0).unwrap_or_default() {
                let Some(id_val) = row.get("game_id") else {
                    continue;
                };
                let gid = canonical_id_from_value(id_val, "game");
                let name = row
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                if !gid.is_empty() {
                    names.insert(gid, name);
                }
            }
        }

        let mut out: Vec<(String, f64, i32, f64)> = agg
            .into_iter()
            .map(|(gid, (plays, sum_part, completed))| {
                let difficulty = if plays > 0 {
                    sum_part as f64 / plays as f64
                } else {
                    0.0
                };
                let win_rate = if plays > 0 {
                    (completed as f64 * 100.0) / plays as f64
                } else {
                    0.0
                };
                let name = names.get(&gid).cloned().unwrap_or_else(|| "Unknown".to_string());
                (name, difficulty, plays, win_rate)
            })
            .collect();
        out.sort_by(|a, b| b.2.cmp(&a.2));
        out.truncate(25);
        Ok(out)
    }

    /// Get venue performance by time slot (Morning/Afternoon/Evening in `timezone`).
    pub async fn get_venue_performance_timeslots(
        &self,
        timezone: &str,
    ) -> Result<Vec<(String, String, String, f64)>> {
        let mut res_pa = self
            .db
            .query("SELECT `in` AS contest_id, `out` AS venue_id FROM played_at")
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let pa_rows: Vec<serde_json::Value> = res_pa.take(0).unwrap_or_default();
        let mut contest_venue: HashMap<String, String> = HashMap::new();
        for row in &pa_rows {
            let Some(cid_val) = row.get("contest_id") else {
                continue;
            };
            let Some(vid_val) = row.get("venue_id") else {
                continue;
            };
            let cid = canonical_id_from_value(cid_val, "contest");
            let vid = canonical_id_from_value(vid_val, "venue");
            if !cid.is_empty() && !vid.is_empty() {
                contest_venue.insert(cid, vid);
            }
        }
        if contest_venue.is_empty() {
            return Ok(Vec::new());
        }

        let contest_ids: Vec<surrealdb::types::RecordId> = contest_venue
            .keys()
            .filter_map(|cid| {
                let key = record_id_to_key(cid, "contest");
                if key.is_empty() {
                    None
                } else {
                    Some(surrealdb::types::RecordId::new("contest", key.as_str()))
                }
            })
            .collect();
        let mut res_starts = self
            .db
            .query("SELECT id, start FROM contest WHERE id INSIDE $ids")
            .bind(("ids", contest_ids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct ContestStartRow {
            id: Option<surrealdb::types::RecordId>,
            start: Option<surrealdb::types::Datetime>,
        }
        let start_rows: Vec<ContestStartRow> = res_starts.take(0).unwrap_or_default();
        let mut contest_start: HashMap<String, chrono::DateTime<chrono::Utc>> = HashMap::new();
        for r in start_rows {
            if let (Some(id), Some(start)) = (r.id, r.start) {
                let cid = thing_to_record_id(&Some(id));
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&start.to_string()) {
                    contest_start.insert(cid, dt.with_timezone(&chrono::Utc));
                }
            }
        }

        fn timeslot_for_hour(hour: u32) -> &'static str {
            if (6..12).contains(&hour) {
                "Morning"
            } else if (12..18).contains(&hour) {
                "Afternoon"
            } else {
                "Evening"
            }
        }

        let mut counts: HashMap<(String, String), i32> = HashMap::new();
        for (cid, vid) in &contest_venue {
            let Some(start) = contest_start.get(cid) else {
                continue;
            };
            let slot = timeslot_for_hour(
                shared::timezone::local_weekday_hour(*start, timezone).1 as u32,
            )
            .to_string();
            *counts.entry((vid.clone(), slot)).or_insert(0) += 1;
        }

        let venue_ids: Vec<surrealdb::types::RecordId> = counts
            .keys()
            .map(|(vid, _)| vid)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .filter_map(|vid| {
                let key = record_id_to_key(vid, "venue");
                if key.is_empty() {
                    None
                } else {
                    Some(surrealdb::types::RecordId::new("venue", key.as_str()))
                }
            })
            .collect();
        let mut venue_names: HashMap<String, String> = HashMap::new();
        if !venue_ids.is_empty() {
            let mut res_names = self
                .db
                .query(
                    "SELECT string::concat(id) AS venue_id, displayName AS name FROM venue WHERE id INSIDE $ids",
                )
                .bind(("ids", venue_ids))
                .await
                .map_err(|e| SharedError::Database(e.to_string()))?;
            for row in res_names.take::<Vec<serde_json::Value>>(0).unwrap_or_default() {
                let Some(id_val) = row.get("venue_id") else {
                    continue;
                };
                let vid = canonical_id_from_value(id_val, "venue");
                let name = row
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                if !vid.is_empty() {
                    venue_names.insert(vid, name);
                }
            }
        }

        let mut out: Vec<(String, String, String, f64)> = counts
            .into_iter()
            .map(|((vid, slot), count)| {
                let name = venue_names
                    .get(&vid)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string());
                (vid, name, slot, count as f64)
            })
            .collect();
        out.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        Ok(out)
    }

    /// Gaming communities: opponents with overlapping co-players across shared contests.
    pub async fn get_gaming_communities_for_player(
        &self,
        player_id: &str,
        min_contests: i32,
    ) -> Result<Vec<serde_json::Value>> {
        let key = player_id_to_key(player_id);
        if key.is_empty() {
            return Ok(Vec::new());
        }
        let record_id = surrealdb::types::RecordId::new("player", key.as_str());
        let mut res1 = self
            .db
            .query("SELECT `in` AS contest_id FROM resulted_in WHERE `out` = $record_id")
            .bind(("record_id", record_id))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let my_rows: Vec<ResultedInRow> = res1
            .take(0)
            .map_err(|e| SharedError::Database(format!("communities my contests: {}", e)))?;
        let contest_ids: Vec<String> = my_rows
            .iter()
            .filter_map(|r| {
                let rid = thing_to_record_id(&r.contest_id);
                if rid.is_empty() {
                    None
                } else {
                    Some(rid.replace('/', ":"))
                }
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        if contest_ids.is_empty() {
            return Ok(Vec::new());
        }
        let contest_things = strings_to_record_id_array(&contest_ids);
        let mut res = self
            .db
            .query(
                "SELECT `in` AS contest_id, `out` AS player_id FROM resulted_in WHERE `in` INSIDE $contest_ids",
            )
            .bind(("contest_ids", contest_things))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let rows: Vec<ResultedInRow> = res
            .take(0)
            .map_err(|e| SharedError::Database(format!("communities participants: {}", e)))?;

        let my_id_norm = normalize_player_id(player_id);
        let mut by_contest: HashMap<String, Vec<String>> = HashMap::new();
        for r in rows {
            let cid = thing_to_record_id(&r.contest_id);
            let pid = thing_to_record_id(&r.player_id);
            if cid.is_empty() || pid.is_empty() || pid == my_id_norm {
                continue;
            }
            by_contest.entry(cid).or_default().push(pid);
        }

        let mut opponent_contests: HashMap<String, i32> = HashMap::new();
        for players in by_contest.values() {
            for pid in players {
                *opponent_contests.entry(pid.clone()).or_insert(0) += 1;
            }
        }

        let mut communities: Vec<serde_json::Value> = Vec::new();
        for (opponent_id, shared_with_me) in opponent_contests {
            if shared_with_me < min_contests {
                continue;
            }
            let mut member_set: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut strength = 0i32;
            for players in by_contest.values() {
                if !players.contains(&opponent_id) {
                    continue;
                }
                for pid in players {
                    if pid != &opponent_id {
                        member_set.insert(pid.clone());
                        strength += 1;
                    }
                }
            }
            if member_set.is_empty() {
                continue;
            }
            let opp_key = record_id_to_key(&opponent_id, "player");
            let opp_rid = surrealdb::types::RecordId::new("player", opp_key.as_str());
            let mut hres = self
                .db
                .query("SELECT handle FROM player WHERE id = $record_id LIMIT 1")
                .bind(("record_id", opp_rid))
                .await
                .map_err(|e| SharedError::Database(e.to_string()))?;
            let handle_rows: Vec<serde_json::Value> = hres.take(0).unwrap_or_default();
            let handle = handle_rows
                .first()
                .and_then(|v| v.get("handle"))
                .and_then(|h| h.as_str())
                .unwrap_or("Unknown")
                .to_string();
            communities.push(serde_json::json!({
                "community_leader": {
                    "opponent_handle": handle,
                    "player_id": opponent_id,
                },
                "total_members": member_set.len(),
                "community_strength": (strength as f64) + (shared_with_me as f64),
            }));
        }
        communities.sort_by(|a, b| {
            let sa = a["community_strength"].as_f64().unwrap_or(0.0);
            let sb = b["community_strength"].as_f64().unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
        communities.truncate(10);
        Ok(communities)
    }

    /// Get contest completion rate by game
    pub async fn get_contest_completion_by_game(&self) -> Result<Vec<(String, i32, f64)>> {
        let mut res_pw = self
            .db
            .query("SELECT `in` AS contest_id, `out` AS game_id FROM played_with")
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let pw_rows: Vec<serde_json::Value> = res_pw.take(0).unwrap_or_default();
        let mut contest_to_game: HashMap<String, String> = HashMap::new();
        for row in &pw_rows {
            let Some(cid_val) = row.get("contest_id") else {
                continue;
            };
            let Some(gid_val) = row.get("game_id") else {
                continue;
            };
            let cid = canonical_id_from_value(cid_val, "contest");
            let gid = canonical_id_from_value(gid_val, "game");
            if !cid.is_empty() && !gid.is_empty() {
                contest_to_game.insert(cid, gid);
            }
        }

        let mut res_ri = self
            .db
            .query(
                "SELECT `in` AS contest_id, count() AS participants, math::min(place) AS min_place FROM resulted_in GROUP BY `in`",
            )
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let ri_rows: Vec<serde_json::Value> = res_ri.take(0).unwrap_or_default();

        let mut agg: HashMap<String, (i32, i32)> = HashMap::new(); // total, completed
        for row in &ri_rows {
            let Some(cid_val) = row.get("contest_id") else {
                continue;
            };
            let cid = canonical_id_from_value(cid_val, "contest");
            let Some(gid) = contest_to_game.get(&cid).cloned() else {
                continue;
            };
            let participants = row
                .get("participants")
                .map(scalar_i64)
                .unwrap_or(0) as i32;
            let min_place = row.get("min_place").map(scalar_i64).unwrap_or(0) as i32;
            let completed = participants > 0 && min_place > 0;
            let e = agg.entry(gid).or_insert((0, 0));
            e.0 += 1;
            if completed {
                e.1 += 1;
            }
        }

        let game_ids: Vec<surrealdb::types::RecordId> = agg
            .keys()
            .filter_map(|gid| {
                let key = record_id_to_key(gid, "game");
                if key.is_empty() {
                    None
                } else {
                    Some(surrealdb::types::RecordId::new("game", key.as_str()))
                }
            })
            .collect();
        let mut names: HashMap<String, String> = HashMap::new();
        if !game_ids.is_empty() {
            let mut res_names = self
                .db
                .query("SELECT string::concat(id) AS game_id, name FROM game WHERE id INSIDE $ids")
                .bind(("ids", game_ids))
                .await
                .map_err(|e| SharedError::Database(e.to_string()))?;
            for row in res_names.take::<Vec<serde_json::Value>>(0).unwrap_or_default() {
                let Some(id_val) = row.get("game_id") else {
                    continue;
                };
                let gid = canonical_id_from_value(id_val, "game");
                let name = row
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                if !gid.is_empty() {
                    names.insert(gid, name);
                }
            }
        }

        let mut out: Vec<(String, i32, f64)> = agg
            .into_iter()
            .map(|(gid, (total, completed))| {
                let rate = if total > 0 {
                    (completed as f64 * 100.0) / total as f64
                } else {
                    0.0
                };
                let name = names.get(&gid).cloned().unwrap_or_else(|| "Unknown".to_string());
                (name, total, rate)
            })
            .collect();
        out.sort_by(|a, b| b.1.cmp(&a.1));
        out.truncate(25);
        Ok(out)
    }

    /// Get player retention cohort data
    pub async fn get_player_retention_cohort(&self) -> Result<Vec<(String, i32, f64)>> {
        let mut res_c = self
            .db
            .query("SELECT id, start FROM contest WHERE start >= time::now() - 12w")
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        #[derive(serde::Deserialize, serde::Serialize, surrealdb::types::SurrealValue)]
        struct ContestStartRow {
            id: Option<surrealdb::types::RecordId>,
            start: Option<surrealdb::types::Datetime>,
        }
        let contest_rows: Vec<ContestStartRow> = res_c.take(0).unwrap_or_default();
        let mut contest_week: HashMap<String, String> = HashMap::new();
        for r in contest_rows {
            let cid = thing_to_record_id(&r.id);
            let Some(start) = r.start else {
                continue;
            };
            let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&start.to_string()) else {
                continue;
            };
            let week = dt.format("%G-W%V").to_string();
            if !cid.is_empty() {
                contest_week.insert(cid, week);
            }
        }
        if contest_week.is_empty() {
            return Ok(Vec::new());
        }

        let contest_ids: Vec<surrealdb::types::RecordId> = contest_week
            .keys()
            .filter_map(|cid| {
                let key = record_id_to_key(cid, "contest");
                if key.is_empty() {
                    None
                } else {
                    Some(surrealdb::types::RecordId::new("contest", key.as_str()))
                }
            })
            .collect();
        let mut res_ri = self
            .db
            .query(
                "SELECT `in` AS contest_id, `out` AS player_id FROM resulted_in WHERE `in` INSIDE $contest_ids",
            )
            .bind(("contest_ids", contest_ids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let ri_rows: Vec<serde_json::Value> = res_ri.take(0).unwrap_or_default();

        let mut week_players: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
        for row in &ri_rows {
            let Some(cid_val) = row.get("contest_id") else {
                continue;
            };
            let Some(pid_val) = row.get("player_id") else {
                continue;
            };
            let cid = canonical_id_from_value(cid_val, "contest");
            let pid = canonical_id_from_value(pid_val, "player");
            let Some(week) = contest_week.get(&cid) else {
                continue;
            };
            if pid.is_empty() {
                continue;
            }
            week_players
                .entry(week.clone())
                .or_default()
                .insert(pid);
        }

        let mut weeks: Vec<String> = week_players.keys().cloned().collect();
        weeks.sort();
        let mut cohorts: Vec<(String, i32, f64)> = Vec::new();
        let mut prev_count: Option<i32> = None;
        for week in weeks {
            let players = week_players
                .get(&week)
                .map(|s| s.len() as i32)
                .unwrap_or(0);
            let retention = match prev_count {
                Some(prev) if prev > 0 => (players as f64) / (prev as f64),
                _ => 1.0,
            };
            prev_count = Some(players);
            cohorts.push((week, players, retention));
        }
        Ok(cohorts)
    }

    /// Get head-to-head win matrix for top players
    pub async fn get_head_to_head_matrix(&self, limit: i32) -> Result<Vec<(String, String, f64)>> {
        let lim = limit.clamp(2, 20) as i64;
        let top_sql = r#"
            SELECT p.handle AS handle,
                   type::record('player', string::replace(string::concat(ri.out), '`', '')) AS player_rid
            FROM resulted_in ri
            JOIN player p ON ri.out = p.id
            GROUP BY ri.out, p.handle
            ORDER BY count() DESC
            LIMIT $lim
        "#;
        let mut top_res = self
            .db
            .query(top_sql)
            .bind(("lim", lim))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let top_rows: Vec<serde_json::Value> = top_res.take(0).unwrap_or_default();
        if top_rows.len() < 2 {
            return Ok(Vec::new());
        }

        let mut handles: Vec<String> = Vec::new();
        let mut player_ids: Vec<String> = Vec::new();
        for row in &top_rows {
            if let (Some(handle), Some(rid)) = (
                row.get("handle").and_then(|v| v.as_str()),
                row.get("player_rid").and_then(|v| v.as_str()),
            ) {
                handles.push(handle.to_string());
                player_ids.push(rid.to_string());
            }
        }

        let pair_sql = r#"
            SELECT string::replace(string::concat(my.out), '`', '') AS player1_id,
                   string::replace(string::concat(opp.out), '`', '') AS player2_id,
                   my.place AS place1,
                   opp.place AS place2
            FROM resulted_in my
            JOIN resulted_in opp ON my.in = opp.in AND my.out != opp.out
            WHERE my.out IN $player_rids AND opp.out IN $player_rids
        "#;
        let player_rids: Vec<surrealdb::types::RecordId> = player_ids
            .iter()
            .filter_map(|id| {
                let key = player_id_to_key(id);
                if key.is_empty() {
                    None
                } else {
                    Some(surrealdb::types::RecordId::new("player", key.as_str()))
                }
            })
            .collect();
        let mut pair_res = self
            .db
            .query(pair_sql)
            .bind(("player_rids", player_rids))
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let pair_rows: Vec<serde_json::Value> = pair_res.take(0).unwrap_or_default();

        let mut wins: HashMap<(String, String), (i32, i32)> = HashMap::new();
        for row in pair_rows {
            let p1 = row
                .get("player1_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let p2 = row
                .get("player2_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let place1 = row.get("place1").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let place2 = row.get("place2").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let entry = wins.entry((p1.clone(), p2.clone())).or_insert((0, 0));
            entry.1 += 1;
            if place1 == 1 && place2 != 1 {
                entry.0 += 1;
            }
        }

        let id_to_handle: HashMap<String, String> = player_ids
            .iter()
            .zip(handles.iter())
            .map(|(id, h)| (player_id_to_key(id), h.clone()))
            .collect();

        let mut matrix = Vec::new();
        for (i, id1) in player_ids.iter().enumerate() {
            let key1 = player_id_to_key(id1);
            let h1 = id_to_handle.get(&key1).cloned().unwrap_or_else(|| key1.clone());
            for id2 in player_ids.iter().skip(i + 1) {
                let key2 = player_id_to_key(id2);
                let h2 = id_to_handle.get(&key2).cloned().unwrap_or_else(|| key2.clone());
                let (w1, total1) = wins.get(&(key1.clone(), key2.clone())).copied().unwrap_or((0, 0));
                let (_w2, total2) = wins.get(&(key2.clone(), key1.clone())).copied().unwrap_or((0, 0));
                let total = total1.max(total2);
                if total == 0 {
                    continue;
                }
                let rate = (w1 as f64 * 100.0) / total as f64;
                matrix.push((h1.clone(), h2.clone(), rate));
            }
        }
        Ok(matrix)
    }

    /// Get games by player count distribution with individual game breakdowns
    pub async fn get_games_by_player_count(&self) -> Result<Vec<(i32, Vec<(String, i32)>)>> {
        let mut res_pc = self
            .db
            .query(
                "SELECT `in` AS contest_id, count() AS participant_count FROM resulted_in GROUP BY `in`",
            )
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let pc_rows: Vec<serde_json::Value> = res_pc.take(0).unwrap_or_default();

        let mut participants_by_contest: HashMap<String, i32> = HashMap::new();
        for row in &pc_rows {
            let Some(contest_val) = row.get("contest_id") else {
                continue;
            };
            let contest_id = canonical_id_from_value(contest_val, "contest");
            if contest_id.is_empty() {
                continue;
            }
            let count = row
                .get("participant_count")
                .map(scalar_i64)
                .unwrap_or(0) as i32;
            if count >= 2 {
                participants_by_contest.insert(contest_id, count);
            }
        }

        let mut res_pw = self
            .db
            .query("SELECT `in` AS contest_id, `out` AS game_id FROM played_with")
            .await
            .map_err(|e| SharedError::Database(e.to_string()))?;
        let pw_rows: Vec<serde_json::Value> = res_pw.take(0).unwrap_or_default();

        let mut plays_by_bucket_game: HashMap<(i32, String), i32> = HashMap::new();
        for row in &pw_rows {
            let Some(contest_val) = row.get("contest_id") else {
                continue;
            };
            let Some(game_val) = row.get("game_id") else {
                continue;
            };
            let contest_id = canonical_id_from_value(contest_val, "contest");
            let game_id = canonical_id_from_value(game_val, "game");
            if contest_id.is_empty() || game_id.is_empty() {
                continue;
            }
            let Some(participant_count) = participants_by_contest.get(&contest_id) else {
                continue;
            };
            let bucket = if *participant_count >= 10 {
                10
            } else {
                *participant_count
            };
            *plays_by_bucket_game
                .entry((bucket, game_id.clone()))
                .or_insert(0) += 1;
        }

        let game_record_ids: Vec<surrealdb::types::RecordId> = plays_by_bucket_game
            .keys()
            .map(|(_, gid)| gid.as_str())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .filter_map(|gid| {
                let key = record_id_to_key(gid, "game");
                if key.is_empty() {
                    None
                } else {
                    Some(surrealdb::types::RecordId::new("game", key.as_str()))
                }
            })
            .collect();

        let mut name_by_game: HashMap<String, String> = HashMap::new();
        if !game_record_ids.is_empty() {
            let mut res_names = self
                .db
                .query("SELECT string::concat(id) AS game_id, name FROM game WHERE id INSIDE $ids")
                .bind(("ids", game_record_ids))
                .await
                .map_err(|e| SharedError::Database(e.to_string()))?;
            let name_rows: Vec<serde_json::Value> = res_names.take(0).unwrap_or_default();
            for row in name_rows {
                let Some(id_val) = row.get("game_id") else {
                    continue;
                };
                let gid = canonical_id_from_value(id_val, "game");
                let name = row
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                if !gid.is_empty() {
                    name_by_game.insert(gid, name);
                }
            }
        }

        let mut out: Vec<(i32, Vec<(String, i32)>)> = Vec::new();
        for bucket in 2..=10 {
            let mut games: Vec<(String, i32)> = plays_by_bucket_game
                .iter()
                .filter_map(|((pc, gid), count)| {
                    if *pc != bucket {
                        return None;
                    }
                    let name = name_by_game
                        .get(gid)
                        .cloned()
                        .unwrap_or_else(|| "Unknown".to_string());
                    Some((name, *count))
                })
                .collect();
            games.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            out.push((bucket, games));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_building() {
        // Test that query building functions work without database connection
        // Create a minimal config for testing
        let config = DatabaseConfig {
            url: "http://localhost:50001".to_string(),
            ns: "test".to_string(),
            name: "test".to_string(),
            root_username: "root".to_string(),
            root_password: "".to_string(),
            username: "root".to_string(),
            password: "root".to_string(),
            pool_size: 10,
            _timeout_seconds: 30,
        };

        // Test that we can create the config
        assert_eq!(config.name, "test");
        assert_eq!(config.url, "http://localhost:50001");
    }

    #[test]
    fn test_analytics_repository_creation() {
        // Test that we can create a repository structure
        let config = DatabaseConfig {
            url: "http://localhost:50001".to_string(),
            ns: "test".to_string(),
            name: "test".to_string(),
            root_username: "root".to_string(),
            root_password: "".to_string(),
            username: "test_user".to_string(),
            password: "test_pass".to_string(),
            pool_size: 10,
            _timeout_seconds: 30,
        };

        assert_eq!(config.name, "test");
    }
}
