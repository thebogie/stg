//! Tab-specific platform analytics aggregations for the analytics dashboard.

use super::repository::{
    canonical_id_from_value, scalar_f64, scalar_i64, AnalyticsRepository, HeatRow,
};
use shared::Result;
use std::collections::{HashMap, HashSet};

impl AnalyticsRepository {
    /// Players whose first contest falls in the last N days vs returning actives in that window.
    pub async fn get_new_vs_returning_players(&self, days: i32) -> Result<(i32, i32)> {
        let mut res_ri = self
            .db()
            .query(
                "SELECT `out` AS player_id, `in` AS contest_id FROM resulted_in",
            )
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let ri_rows: Vec<serde_json::Value> = res_ri.take(0).unwrap_or_default();

        let mut res_c = self
            .db()
            .query("SELECT string::concat(id) AS contest_id, start FROM contest")
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let c_rows: Vec<serde_json::Value> = res_c.take(0).unwrap_or_default();

        let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
        let mut contest_start: HashMap<String, chrono::DateTime<chrono::Utc>> = HashMap::new();
        for row in &c_rows {
            let Some(cid_val) = row.get("contest_id") else {
                continue;
            };
            let cid = canonical_id_from_value(cid_val, "contest");
            let Some(start_str) = row.get("start").and_then(|v| v.as_str()) else {
                continue;
            };
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(start_str) {
                contest_start.insert(cid, dt.with_timezone(&chrono::Utc));
            }
        }

        let mut first_contest: HashMap<String, chrono::DateTime<chrono::Utc>> = HashMap::new();
        let mut active_in_window: HashSet<String> = HashSet::new();

        for row in &ri_rows {
            let Some(pid_val) = row.get("player_id") else {
                continue;
            };
            let Some(cid_val) = row.get("contest_id") else {
                continue;
            };
            let pid = canonical_id_from_value(pid_val, "player");
            let cid = canonical_id_from_value(cid_val, "contest");
            if pid.is_empty() || cid.is_empty() {
                continue;
            }
            let Some(start) = contest_start.get(&cid) else {
                continue;
            };
            first_contest
                .entry(pid.clone())
                .and_modify(|e| {
                    if start < e {
                        *e = *start;
                    }
                })
                .or_insert(*start);
            if *start >= cutoff {
                active_in_window.insert(pid);
            }
        }

        let mut new_count = 0i32;
        let mut returning_count = 0i32;
        for pid in active_in_window {
            if let Some(first) = first_contest.get(&pid) {
                if *first >= cutoff {
                    new_count += 1;
                } else {
                    returning_count += 1;
                }
            }
        }
        Ok((new_count, returning_count))
    }

    pub async fn get_platform_completion_rate(&self) -> Result<f64> {
        let mut res = self
            .db()
            .query(
                "SELECT `in` AS contest_id, count() AS participants, math::min(place) AS min_place FROM resulted_in GROUP BY `in`",
            )
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let mut total = 0i32;
        let mut completed = 0i32;
        for row in &rows {
            let participants = row
                .get("participants")
                .map(scalar_i64)
                .unwrap_or(0) as i32;
            if participants == 0 {
                continue;
            }
            total += 1;
            let min_place = row.get("min_place").map(scalar_i64).unwrap_or(0);
            if min_place > 0 {
                completed += 1;
            }
        }
        Ok(if total > 0 {
            (completed as f64 * 100.0) / total as f64
        } else {
            0.0
        })
    }

    pub async fn get_week_over_week_contests(&self, timezone: &str) -> Result<(i32, i32)> {
        let (this_week, last_week) = shared::timezone::current_and_previous_iso_weeks(timezone);
        let mut res = self
            .db()
            .query(
                "SELECT start FROM contest WHERE start >= time::now() - duration::from_days(21)",
            )
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let mut this_count = 0i32;
        let mut last_count = 0i32;
        for row in &rows {
            let Some(start_s) = row.get("start").and_then(|v| v.as_str()) else {
                continue;
            };
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(start_s) {
                let week =
                    shared::timezone::iso_week_label(dt.with_timezone(&chrono::Utc), timezone);
                if week == this_week {
                    this_count += 1;
                } else if week == last_week {
                    last_count += 1;
                }
            }
        }
        Ok((this_count, last_count))
    }

    pub async fn get_week_over_week_active_players(&self, timezone: &str) -> Result<(i32, i32)> {
        let (this_week, last_week) = shared::timezone::current_and_previous_iso_weeks(timezone);
        let mut res_c = self
            .db()
            .query(
                "SELECT string::concat(id) AS contest_id, start FROM contest WHERE start >= time::now() - duration::from_days(21)",
            )
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let c_rows: Vec<serde_json::Value> = res_c.take(0).unwrap_or_default();
        let mut contest_week: HashMap<String, String> = HashMap::new();
        for row in &c_rows {
            let Some(cid_val) = row.get("contest_id") else {
                continue;
            };
            let cid = canonical_id_from_value(cid_val, "contest");
            let Some(start_s) = row.get("start").and_then(|v| v.as_str()) else {
                continue;
            };
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(start_s) {
                let week =
                    shared::timezone::iso_week_label(dt.with_timezone(&chrono::Utc), timezone);
                contest_week.insert(cid, week);
            }
        }
        if contest_week.is_empty() {
            return Ok((0, 0));
        }
        let rid_param: Vec<String> = contest_week
            .keys()
            .map(|s| s.replace('/', ":"))
            .collect();
        let mut res_ri = self
            .db()
            .query("SELECT `in` AS contest_id, `out` AS player_id FROM resulted_in WHERE `in` INSIDE $rids")
            .bind(("rids", rid_param))
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let ri_rows: Vec<serde_json::Value> = res_ri.take(0).unwrap_or_default();
        let mut this_players: HashSet<String> = HashSet::new();
        let mut last_players: HashSet<String> = HashSet::new();
        for row in &ri_rows {
            let Some(cid_val) = row.get("contest_id") else {
                continue;
            };
            let cid = canonical_id_from_value(cid_val, "contest");
            let Some(week) = contest_week.get(&cid) else {
                continue;
            };
            let Some(pid_val) = row.get("player_id") else {
                continue;
            };
            let pid = canonical_id_from_value(pid_val, "player");
            if week == &this_week {
                this_players.insert(pid);
            } else if week == &last_week {
                last_players.insert(pid);
            }
        }
        Ok((this_players.len() as i32, last_players.len() as i32))
    }

    /// Monthly contest counts bucketed in the player's timezone.
    pub async fn get_monthly_contest_trends(
        &self,
        months: i32,
        timezone: &str,
    ) -> Result<Vec<(String, String, i32)>> {
        let days = months.saturating_mul(31);
        let mut res = self
            .db()
            .query(
                "SELECT start FROM contest WHERE start >= time::now() - duration::from_days($days)",
            )
            .bind(("days", days))
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let mut by_month: HashMap<String, (String, i32)> = HashMap::new();
        for row in &rows {
            let Some(start_s) = row.get("start").and_then(|v| v.as_str()) else {
                continue;
            };
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(start_s) {
                let utc = dt.with_timezone(&chrono::Utc);
                let key = shared::timezone::month_bucket_key(utc, timezone);
                let label = shared::timezone::month_label(utc, timezone);
                by_month
                    .entry(key)
                    .and_modify(|(_, c)| *c += 1)
                    .or_insert((label, 1));
            }
        }
        let mut out: Vec<(String, String, i32)> = by_month
            .into_iter()
            .map(|(key, (label, count))| (key, label, count))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    /// Unique active players per calendar month in the player's timezone.
    pub async fn get_monthly_active_players(
        &self,
        months: i32,
        timezone: &str,
    ) -> Result<Vec<(String, String, i32)>> {
        let days = months.saturating_mul(31);
        let mut res_c = self
            .db()
            .query(
                "SELECT string::concat(id) AS contest_id, start FROM contest WHERE start >= time::now() - duration::from_days($days)",
            )
            .bind(("days", days))
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let c_rows: Vec<serde_json::Value> = res_c.take(0).unwrap_or_default();
        let mut contest_month: HashMap<String, String> = HashMap::new();
        for row in &c_rows {
            let Some(cid_val) = row.get("contest_id") else {
                continue;
            };
            let cid = canonical_id_from_value(cid_val, "contest");
            let Some(start_s) = row.get("start").and_then(|v| v.as_str()) else {
                continue;
            };
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(start_s) {
                let key =
                    shared::timezone::month_bucket_key(dt.with_timezone(&chrono::Utc), timezone);
                contest_month.insert(cid, key);
            }
        }
        if contest_month.is_empty() {
            return Ok(Vec::new());
        }
        let rid_param: Vec<String> = contest_month
            .keys()
            .map(|s| s.replace('/', ":"))
            .collect();
        let mut res_ri = self
            .db()
            .query("SELECT `in` AS contest_id, `out` AS player_id FROM resulted_in WHERE `in` INSIDE $rids")
            .bind(("rids", rid_param))
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let ri_rows: Vec<serde_json::Value> = res_ri.take(0).unwrap_or_default();
        let mut by_month: HashMap<String, HashSet<String>> = HashMap::new();
        for row in &ri_rows {
            let Some(cid_val) = row.get("contest_id") else {
                continue;
            };
            let cid = canonical_id_from_value(cid_val, "contest");
            let Some(month_key) = contest_month.get(&cid) else {
                continue;
            };
            let Some(pid_val) = row.get("player_id") else {
                continue;
            };
            let pid = canonical_id_from_value(pid_val, "player");
            by_month.entry(month_key.clone()).or_default().insert(pid);
        }
        let mut out: Vec<(String, String, i32)> = by_month
            .into_iter()
            .map(|(key, players)| {
                let label = if let Ok(dt) = chrono::NaiveDate::parse_from_str(
                    &format!("{}-01", key),
                    "%Y-%m-%d",
                ) {
                    shared::timezone::month_label(
                        dt.and_hms_opt(12, 0, 0)
                            .unwrap()
                            .and_utc(),
                        timezone,
                    )
                } else {
                    key.clone()
                };
                (key, label, players.len() as i32)
            })
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    pub async fn get_weekly_contest_sparkline(
        &self,
        weeks: i32,
        timezone: &str,
    ) -> Result<Vec<(String, i32)>> {
        let days = weeks * 7;
        let mut res = self
            .db()
            .query(
                "SELECT start FROM contest WHERE start >= time::now() - duration::from_days($days)",
            )
            .bind(("days", days))
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let mut by_week: HashMap<String, i32> = HashMap::new();
        for row in &rows {
            let Some(start_s) = row.get("start").and_then(|v| v.as_str()) else {
                continue;
            };
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(start_s) {
                let week =
                    shared::timezone::iso_week_label(dt.with_timezone(&chrono::Utc), timezone);
                *by_week.entry(week).or_insert(0) += 1;
            }
        }
        let mut out: Vec<(String, i32)> = by_week.into_iter().collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    pub async fn get_contest_duration_stats(&self) -> Result<f64> {
        let mut res = self
            .db()
            .query(
                "SELECT start, stop FROM contest WHERE start IS NOT NONE AND stop IS NOT NONE",
            )
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let mut total_minutes = 0.0;
        let mut count = 0i32;
        for row in &rows {
            let Some(start_s) = row.get("start").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(stop_s) = row.get("stop").and_then(|v| v.as_str()) else {
                continue;
            };
            let Ok(start) = chrono::DateTime::parse_from_rfc3339(start_s) else {
                continue;
            };
            let Ok(stop) = chrono::DateTime::parse_from_rfc3339(stop_s) else {
                continue;
            };
            let mins = (stop - start).num_minutes() as f64;
            if mins > 0.0 && mins < 24.0 * 60.0 {
                total_minutes += mins;
                count += 1;
            }
        }
        Ok(if count > 0 {
            total_minutes / count as f64
        } else {
            0.0
        })
    }

    pub async fn get_time_to_fill_hours(&self) -> Result<f64> {
        let mut res = self
            .db()
            .query(
                "SELECT created_at, start FROM contest WHERE created_at IS NOT NONE AND start IS NOT NONE",
            )
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let mut total_hours = 0.0;
        let mut count = 0i32;
        for row in &rows {
            let Some(created_s) = row.get("created_at").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(start_s) = row.get("start").and_then(|v| v.as_str()) else {
                continue;
            };
            let Ok(created) = chrono::DateTime::parse_from_rfc3339(created_s) else {
                continue;
            };
            let Ok(start) = chrono::DateTime::parse_from_rfc3339(start_s) else {
                continue;
            };
            let hours = (start - created).num_minutes() as f64 / 60.0;
            if hours >= 0.0 && hours < 24.0 * 30.0 {
                total_hours += hours;
                count += 1;
            }
        }
        Ok(if count > 0 {
            total_hours / count as f64
        } else {
            0.0
        })
    }

    pub async fn get_contest_size_distribution(&self) -> Result<Vec<(String, i32)>> {
        let mut res = self
            .db()
            .query(
                "SELECT `in` AS contest_id, count() AS participant_count FROM resulted_in GROUP BY `in`",
            )
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let mut buckets: HashMap<String, i32> = HashMap::new();
        for row in &rows {
            let n = row
                .get("participant_count")
                .map(scalar_i64)
                .unwrap_or(0) as i32;
            if n < 2 {
                continue;
            }
            let label = if n >= 6 {
                "6+ players".to_string()
            } else {
                format!("{} players", n)
            };
            *buckets.entry(label).or_insert(0) += 1;
        }
        let mut out: Vec<(String, i32)> = buckets.into_iter().collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    /// Average participants per weekday/hour slot (capacity proxy) in `timezone`.
    pub async fn get_peak_participants_heatmap(
        &self,
        weeks: i32,
        timezone: &str,
    ) -> Result<Vec<HeatRow>> {
        let heatmap = self
            .get_contest_heatmap(weeks, None, timezone)
            .await?;
        let mut res_pc = self
            .db()
            .query(
                "SELECT `in` AS contest_id, count() AS participant_count FROM resulted_in GROUP BY `in`",
            )
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let pc_rows: Vec<serde_json::Value> = res_pc.take(0).unwrap_or_default();
        let mut participants: HashMap<String, i64> = HashMap::new();
        for row in &pc_rows {
            let Some(cid_val) = row.get("contest_id") else {
                continue;
            };
            let cid = canonical_id_from_value(cid_val, "contest");
            let count = row
                .get("participant_count")
                .map(scalar_i64)
                .unwrap_or(0);
            participants.insert(cid, count);
        }

        let days = weeks * 7;
        let mut res_c = self
            .db()
            .query("SELECT string::concat(id) AS contest_id, start FROM contest WHERE start >= time::now() - duration::from_days($days)")
            .bind(("days", days))
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let c_rows: Vec<serde_json::Value> = res_c.take(0).unwrap_or_default();

        let mut slot_sum: HashMap<(i32, i32), (f64, i64)> = HashMap::new();
        for row in &c_rows {
            let Some(cid_val) = row.get("contest_id") else {
                continue;
            };
            let cid = canonical_id_from_value(cid_val, "contest");
            let Some(start_s) = row.get("start").and_then(|v| v.as_str()) else {
                continue;
            };
            let Ok(dt) = chrono::DateTime::parse_from_rfc3339(start_s) else {
                continue;
            };
            let dt_utc = dt.with_timezone(&chrono::Utc);
            let (day, hour) = shared::timezone::local_weekday_hour(dt_utc, timezone);
            let pc = participants.get(&cid).copied().unwrap_or(0) as f64;
            let e = slot_sum.entry((day, hour)).or_insert((0.0, 0));
            e.0 += pc;
            e.1 += 1;
        }

        // Fall back to play counts if no participant data
        if slot_sum.is_empty() {
            return Ok(heatmap
                .into_iter()
                .map(|h| HeatRow {
                    day: h.day,
                    hour: h.hour,
                    plays: h.plays,
                })
                .collect());
        }

        Ok(slot_sum
            .into_iter()
            .map(|((day, hour), (sum, n))| HeatRow {
                day,
                hour,
                plays: if n > 0 {
                    (sum / n as f64).round() as i64
                } else {
                    0
                },
            })
            .collect())
    }

    pub async fn get_venue_utilization_rates(&self, limit: i32) -> Result<Vec<(String, String, i32)>> {
        let top = self.get_top_venues(limit).await?;
        Ok(top)
    }

    pub async fn get_venue_game_diversity(&self, limit: i32) -> Result<Vec<(String, String, i32, i32)>> {
        let mut res = self
            .db()
            .query(
                "SELECT `in` AS contest_id, `out` AS venue_id FROM played_at",
            )
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let pa_rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();

        let mut res_pw = self
            .db()
            .query("SELECT `in` AS contest_id, `out` AS game_id FROM played_with")
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let pw_rows: Vec<serde_json::Value> = res_pw.take(0).unwrap_or_default();

        let mut contest_venue: HashMap<String, String> = HashMap::new();
        for row in &pa_rows {
            let cid = row
                .get("contest_id")
                .map(|v| canonical_id_from_value(v, "contest"))
                .unwrap_or_default();
            let vid = row
                .get("venue_id")
                .map(|v| canonical_id_from_value(v, "venue"))
                .unwrap_or_default();
            if !cid.is_empty() && !vid.is_empty() {
                contest_venue.insert(cid, vid);
            }
        }

        let mut venue_games: HashMap<String, HashSet<String>> = HashMap::new();
        let mut venue_contests: HashMap<String, i32> = HashMap::new();
        for row in &pw_rows {
            let cid = row
                .get("contest_id")
                .map(|v| canonical_id_from_value(v, "contest"))
                .unwrap_or_default();
            let gid = row
                .get("game_id")
                .map(|v| canonical_id_from_value(v, "game"))
                .unwrap_or_default();
            let Some(vid) = contest_venue.get(&cid) else {
                continue;
            };
            *venue_contests.entry(vid.clone()).or_insert(0) += 1;
            venue_games
                .entry(vid.clone())
                .or_default()
                .insert(gid);
        }

        let venue_ids: Vec<String> = venue_games.keys().cloned().collect();
        let mut names: HashMap<String, String> = HashMap::new();
        if !venue_ids.is_empty() {
            let record_ids: Vec<surrealdb::types::RecordId> = venue_ids
                .iter()
                .filter_map(|vid| {
                    let key = crate::surreal_helpers::record_id_to_key(vid, "venue");
                    if key.is_empty() {
                        None
                    } else {
                        Some(surrealdb::types::RecordId::new("venue", key.as_str()))
                    }
                })
                .collect();
            let mut res_names = self
                .db()
                .query("SELECT string::concat(id) AS venue_id, displayName AS name FROM venue WHERE id INSIDE $ids")
                .bind(("ids", record_ids))
                .await
                .map_err(|e| shared::SharedError::Database(e.to_string()))?;
            for row in res_names.take::<Vec<serde_json::Value>>(0).unwrap_or_default() {
                let vid = row
                    .get("venue_id")
                    .map(|v| canonical_id_from_value(v, "venue"))
                    .unwrap_or_default();
                let name = row
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                names.insert(vid, name);
            }
        }

        let mut out: Vec<(String, String, i32, i32)> = venue_games
            .into_iter()
            .map(|(vid, games)| {
                let name = names.get(&vid).cloned().unwrap_or_else(|| vid.clone());
                let contests = venue_contests.get(&vid).copied().unwrap_or(0);
                (vid, name, games.len() as i32, contests)
            })
            .collect();
        out.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| b.3.cmp(&a.3)));
        out.truncate(limit as usize);
        Ok(out)
    }

    pub async fn get_venue_retention_rate(&self) -> Result<f64> {
        let mut res = self
            .db()
            .query(
                "SELECT `out` AS player_id, `in` AS contest_id FROM resulted_in",
            )
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let ri_rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();

        let mut res_pa = self
            .db()
            .query("SELECT `in` AS contest_id, `out` AS venue_id FROM played_at")
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let pa_rows: Vec<serde_json::Value> = res_pa.take(0).unwrap_or_default();

        let mut contest_venue: HashMap<String, String> = HashMap::new();
        for row in &pa_rows {
            let cid = row
                .get("contest_id")
                .map(|v| canonical_id_from_value(v, "contest"))
                .unwrap_or_default();
            let vid = row
                .get("venue_id")
                .map(|v| canonical_id_from_value(v, "venue"))
                .unwrap_or_default();
            if !cid.is_empty() && !vid.is_empty() {
                contest_venue.insert(cid, vid);
            }
        }

        let mut player_venue_counts: HashMap<(String, String), i32> = HashMap::new();
        for row in &ri_rows {
            let pid = row
                .get("player_id")
                .map(|v| canonical_id_from_value(v, "player"))
                .unwrap_or_default();
            let cid = row
                .get("contest_id")
                .map(|v| canonical_id_from_value(v, "contest"))
                .unwrap_or_default();
            let Some(vid) = contest_venue.get(&cid) else {
                continue;
            };
            if pid.is_empty() {
                continue;
            }
            *player_venue_counts
                .entry((pid, vid.clone()))
                .or_insert(0) += 1;
        }

        let total_pairs = player_venue_counts.len() as f64;
        if total_pairs == 0.0 {
            return Ok(0.0);
        }
        let returning = player_venue_counts
            .values()
            .filter(|&&c| c >= 2)
            .count() as f64;
        Ok((returning / total_pairs) * 100.0)
    }

    pub async fn get_game_monthly_longevity(&self, limit: i32) -> Result<Vec<(String, String, Vec<(String, i32)>)>> {
        let top_games = self.get_top_games(limit).await?;
        let mut res = self
            .db()
            .query(
                "SELECT `in` AS contest_id, `out` AS game_id FROM played_with",
            )
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let pw_rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();

        let mut res_c = self
            .db()
            .query("SELECT string::concat(id) AS contest_id, start FROM contest")
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let c_rows: Vec<serde_json::Value> = res_c.take(0).unwrap_or_default();

        let mut contest_month: HashMap<String, String> = HashMap::new();
        for row in &c_rows {
            let cid = row
                .get("contest_id")
                .map(|v| canonical_id_from_value(v, "contest"))
                .unwrap_or_default();
            let Some(start_s) = row.get("start").and_then(|v| v.as_str()) else {
                continue;
            };
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(start_s) {
                contest_month.insert(cid, dt.format("%Y-%m").to_string());
            }
        }

        let top_names: HashSet<String> = top_games.iter().map(|(_, n, _)| n.clone()).collect();
        let top_ids: HashSet<String> = top_games.iter().map(|(id, _, _)| id.clone()).collect();
        let mut game_monthly: HashMap<String, HashMap<String, i32>> = HashMap::new();
        for row in &pw_rows {
            let cid = row
                .get("contest_id")
                .map(|v| canonical_id_from_value(v, "contest"))
                .unwrap_or_default();
            let gid = row
                .get("game_id")
                .map(|v| canonical_id_from_value(v, "game"))
                .unwrap_or_default();
            let Some(month) = contest_month.get(&cid) else {
                continue;
            };
            // Resolve game name via top_games list is by name only - aggregate by game id then map
            *game_monthly
                .entry(gid)
                .or_default()
                .entry(month.clone())
                .or_insert(0) += 1;
        }

        // Map game ids to names
        let game_ids: Vec<String> = game_monthly.keys().cloned().collect();
        let mut id_to_name: HashMap<String, String> = HashMap::new();
        if !game_ids.is_empty() {
            let record_ids: Vec<surrealdb::types::RecordId> = game_ids
                .iter()
                .filter_map(|gid| {
                    let key = crate::surreal_helpers::record_id_to_key(gid, "game");
                    if key.is_empty() {
                        None
                    } else {
                        Some(surrealdb::types::RecordId::new("game", key.as_str()))
                    }
                })
                .collect();
            let mut res_names = self
                .db()
                .query("SELECT string::concat(id) AS game_id, name FROM game WHERE id INSIDE $ids")
                .bind(("ids", record_ids))
                .await
                .map_err(|e| shared::SharedError::Database(e.to_string()))?;
            for row in res_names.take::<Vec<serde_json::Value>>(0).unwrap_or_default() {
                let gid = row
                    .get("game_id")
                    .map(|v| canonical_id_from_value(v, "game"))
                    .unwrap_or_default();
                let name = row
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                id_to_name.insert(gid, name);
            }
        }

        let mut out: Vec<(String, String, Vec<(String, i32)>)> = game_monthly
            .into_iter()
            .filter_map(|(gid, months)| {
                let name = id_to_name.get(&gid).cloned().unwrap_or(gid.clone());
                if !top_ids.is_empty() && !top_ids.contains(&gid) {
                    return None;
                }
                if !top_names.is_empty() && !top_names.contains(&name) {
                    return None;
                }
                let mut series: Vec<(String, i32)> = months.into_iter().collect();
                series.sort_by(|a, b| a.0.cmp(&b.0));
                Some((gid, name, series))
            })
            .collect();
        out.sort_by(|a, b| {
            let sum_a: i32 = a.2.iter().map(|(_, c)| c).sum();
            let sum_b: i32 = b.2.iter().map(|(_, c)| c).sum();
            sum_b.cmp(&sum_a)
        });
        out.truncate(limit as usize);
        Ok(out)
    }

    pub async fn get_player_count_fit_score(&self) -> Result<f64> {
        let distribution = self.get_games_by_player_count().await?;
        let mut total_plays = 0i32;
        let mut modal_plays = 0i32;
        let mut bucket_totals: HashMap<i32, i32> = HashMap::new();
        for (bucket, games) in &distribution {
            let bucket_sum: i32 = games.iter().map(|(_, c)| c).sum();
            *bucket_totals.entry(*bucket).or_insert(0) += bucket_sum;
            total_plays += bucket_sum;
        }
        if let Some((_, best)) = bucket_totals.iter().max_by_key(|(_, v)| *v) {
            modal_plays = *best;
        }
        Ok(if total_plays > 0 {
            (modal_plays as f64 / total_plays as f64) * 100.0
        } else {
            0.0
        })
    }

    pub async fn get_cross_venue_game_popularity(
        &self,
        limit: i32,
    ) -> Result<Vec<(String, String, i32, i32)>> {
        let mut res_pa = self
            .db()
            .query("SELECT `in` AS contest_id, `out` AS venue_id FROM played_at")
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let pa_rows: Vec<serde_json::Value> = res_pa.take(0).unwrap_or_default();

        let mut res_pw = self
            .db()
            .query("SELECT `in` AS contest_id, `out` AS game_id FROM played_with")
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let pw_rows: Vec<serde_json::Value> = res_pw.take(0).unwrap_or_default();

        let mut contest_venues: HashMap<String, HashSet<String>> = HashMap::new();
        for row in &pa_rows {
            let cid = row
                .get("contest_id")
                .map(|v| canonical_id_from_value(v, "contest"))
                .unwrap_or_default();
            let vid = row
                .get("venue_id")
                .map(|v| canonical_id_from_value(v, "venue"))
                .unwrap_or_default();
            if !cid.is_empty() && !vid.is_empty() {
                contest_venues.entry(cid).or_default().insert(vid);
            }
        }

        let mut game_venues: HashMap<String, HashSet<String>> = HashMap::new();
        let mut game_plays: HashMap<String, i32> = HashMap::new();
        for row in &pw_rows {
            let cid = row
                .get("contest_id")
                .map(|v| canonical_id_from_value(v, "contest"))
                .unwrap_or_default();
            let gid = row
                .get("game_id")
                .map(|v| canonical_id_from_value(v, "game"))
                .unwrap_or_default();
            if gid.is_empty() {
                continue;
            }
            *game_plays.entry(gid.clone()).or_insert(0) += 1;
            if let Some(venues) = contest_venues.get(&cid) {
                let e = game_venues.entry(gid).or_default();
                for v in venues {
                    e.insert(v.clone());
                }
            }
        }

        let game_ids: Vec<String> = game_plays.keys().cloned().collect();
        let mut names: HashMap<String, String> = HashMap::new();
        if !game_ids.is_empty() {
            let record_ids: Vec<surrealdb::types::RecordId> = game_ids
                .iter()
                .filter_map(|gid| {
                    let key = crate::surreal_helpers::record_id_to_key(gid, "game");
                    if key.is_empty() {
                        None
                    } else {
                        Some(surrealdb::types::RecordId::new("game", key.as_str()))
                    }
                })
                .collect();
            let mut res_names = self
                .db()
                .query("SELECT string::concat(id) AS game_id, name FROM game WHERE id INSIDE $ids")
                .bind(("ids", record_ids))
                .await
                .map_err(|e| shared::SharedError::Database(e.to_string()))?;
            for row in res_names.take::<Vec<serde_json::Value>>(0).unwrap_or_default() {
                let gid = row
                    .get("game_id")
                    .map(|v| canonical_id_from_value(v, "game"))
                    .unwrap_or_default();
                let name = row
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                names.insert(gid, name);
            }
        }

        let mut out: Vec<(String, String, i32, i32)> = game_plays
            .into_iter()
            .map(|(gid, plays)| {
                let venue_count = game_venues.get(&gid).map(|s| s.len() as i32).unwrap_or(0);
                let name = names.get(&gid).cloned().unwrap_or(gid.clone());
                (gid, name, venue_count, plays)
            })
            .collect();
        out.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| b.3.cmp(&a.3)));
        out.truncate(limit as usize);
        Ok(out)
    }

    pub async fn get_rating_distribution(&self) -> Result<Vec<(String, i32)>> {
        let mut res = self
            .db()
            .query(
                "SELECT rating FROM rating_latest WHERE scope_type = 'global'",
            )
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let buckets = [
            ("< 1200", 0.0, 1200.0),
            ("1200–1400", 1200.0, 1400.0),
            ("1400–1600", 1400.0, 1600.0),
            ("1600–1800", 1600.0, 1800.0),
            ("1800+", 1800.0, f64::MAX),
        ];
        let mut counts = vec![0i32; buckets.len()];
        for row in &rows {
            let rating = row.get("rating").map(scalar_f64).unwrap_or(0.0);
            for (i, (_, lo, hi)) in buckets.iter().enumerate() {
                if rating >= *lo && rating < *hi {
                    counts[i] += 1;
                    break;
                }
            }
        }
        Ok(buckets
            .iter()
            .zip(counts.iter())
            .map(|((label, _, _), &count)| (label.to_string(), count))
            .collect())
    }

    pub async fn get_days_since_last_contest(&self, player_id: &str) -> Result<(i32, Option<String>)> {
        let pid = if player_id.contains('/') {
            player_id.to_string()
        } else {
            format!("player/{}", player_id)
        };
        let key = crate::surreal_helpers::record_id_to_key(&pid, "player");
        let mut res = self
            .db()
            .query(
                "SELECT string::concat(`in`) AS contest_id, contest.start AS start FROM resulted_in WHERE `out` = type::record('player', $key) AND contest.start IS NOT NONE ORDER BY contest.start DESC LIMIT 1",
            )
            .bind(("key", key))
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        if let Some(row) = rows.first() {
            let contest_id = row
                .get("contest_id")
                .map(|v| canonical_id_from_value(v, "contest"))
                .filter(|id| !id.is_empty());
            if let Some(start_s) = row.get("start").and_then(|v| v.as_str()) {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(start_s) {
                    let days = (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_days();
                    return Ok((days.max(0) as i32, contest_id));
                }
            }
            return Ok((-1, contest_id));
        }
        Ok((-1, None))
    }

    pub async fn get_rating_history_points(
        &self,
        player_id: &str,
        limit: i32,
    ) -> Result<Vec<(String, f64, i32)>> {
        let pid = if player_id.contains('/') {
            player_id.to_string()
        } else {
            format!("player/{}", player_id)
        };
        let key = crate::surreal_helpers::record_id_to_key(&pid, "player");
        let mut res = self
            .db()
            .query(
                "SELECT period_end, rating, games_played FROM rating_history WHERE player_id = type::record('player', $key) AND scope_type = 'global' ORDER BY period_end ASC LIMIT $limit",
            )
            .bind(("key", key))
            .bind(("limit", limit))
            .await
            .map_err(|e| shared::SharedError::Database(e.to_string()))?;
        let rows: Vec<serde_json::Value> = res.take(0).unwrap_or_default();
        let mut out = Vec::new();
        for row in &rows {
            let period = row
                .get("period_end")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let rating = row.get("rating").map(scalar_f64).unwrap_or(1500.0);
            let gp = row
                .get("games_played")
                .map(scalar_i64)
                .unwrap_or(0) as i32;
            if !period.is_empty() {
                out.push((period, rating, gp));
            }
        }
        Ok(out)
    }
}
