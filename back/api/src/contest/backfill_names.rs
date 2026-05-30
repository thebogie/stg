//! Batch-rename contests to `{Game} — {Weekday Mon D}` titles.

use std::collections::HashMap;

use chrono::{DateTime, FixedOffset};
use shared::contest_name::{default_contest_name, resolve_unique_contest_names, ContestNameRow};
use surrealdb::types::RecordId;

use crate::db::Db;
use crate::surreal_helpers::record_id_to_key;

#[derive(Debug, Clone)]
pub struct BackfillPlan {
    pub contest_id: String,
    pub old_name: String,
    pub new_name: String,
}

#[derive(Debug, Default)]
pub struct BackfillSummary {
    pub total: usize,
    pub unchanged: usize,
    pub to_update: usize,
    pub updated: usize,
    pub plans: Vec<BackfillPlan>,
}

pub async fn plan_contest_name_backfill(
    db: &Db,
    limit: Option<usize>,
) -> Result<BackfillSummary, String> {
    let limit_clause = limit
        .map(|n| format!(" LIMIT {}", n))
        .unwrap_or_default();
    let sql = format!(
        "SELECT string::concat(id) AS id, name, start FROM contest ORDER BY start{}",
        limit_clause
    );
    let mut res = db.query(&sql).await.map_err(|e| e.to_string())?;
    let contests: Vec<serde_json::Value> = res.take(0).map_err(|e| e.to_string())?;
    if contests.is_empty() {
        return Ok(BackfillSummary::default());
    }

    let contest_ids: Vec<RecordId> = contests
        .iter()
        .filter_map(|c| {
            json_str(c, "id").map(|id| RecordId::new("contest", record_id_to_key(&id, "contest")))
        })
        .collect();

    let mut pw_res = db
        .query("SELECT string::concat(`in`) AS contest_id, string::concat(`out`) AS game_id FROM played_with WHERE `in` INSIDE $contests")
        .bind(("contests", contest_ids.clone()))
        .await
        .map_err(|e| e.to_string())?;
    let played_with: Vec<serde_json::Value> = pw_res.take(0).map_err(|e| e.to_string())?;

    let mut pa_res = db
        .query("SELECT string::concat(`in`) AS contest_id, string::concat(`out`) AS venue_id FROM played_at WHERE `in` INSIDE $contests")
        .bind(("contests", contest_ids.clone()))
        .await
        .map_err(|e| e.to_string())?;
    let played_at: Vec<serde_json::Value> = pa_res.take(0).map_err(|e| e.to_string())?;

    let mut games_by_contest: HashMap<String, Vec<String>> = HashMap::new();
    for row in played_with {
        if let (Some(cid), Some(gid)) = (json_str(&row, "contest_id"), json_str(&row, "game_id")) {
            games_by_contest.entry(cid).or_default().push(gid);
        }
    }

    let mut venue_by_contest: HashMap<String, String> = HashMap::new();
    for row in played_at {
        if let (Some(cid), Some(vid)) = (json_str(&row, "contest_id"), json_str(&row, "venue_id")) {
            venue_by_contest.insert(cid, vid);
        }
    }

    let game_ids: Vec<RecordId> = games_by_contest
        .values()
        .flat_map(|ids| ids.iter())
        .map(|id| RecordId::new("game", record_id_to_key(id, "game")))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let venue_ids: Vec<RecordId> = venue_by_contest
        .values()
        .map(|id| RecordId::new("venue", record_id_to_key(id, "venue")))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let mut game_names: HashMap<String, String> = HashMap::new();
    if !game_ids.is_empty() {
        let mut g_res = db
            .query("SELECT string::concat(id) AS id, name FROM game WHERE id INSIDE $ids")
            .bind(("ids", game_ids))
            .await
            .map_err(|e| e.to_string())?;
        let rows: Vec<serde_json::Value> = g_res.take(0).map_err(|e| e.to_string())?;
        for row in rows {
            if let (Some(id), Some(name)) = (json_str(&row, "id"), json_str(&row, "name")) {
                game_names.insert(id, name);
            }
        }
    }

    let mut venue_meta: HashMap<String, (String, String)> = HashMap::new();
    if !venue_ids.is_empty() {
        let mut v_res = db
            .query("SELECT string::concat(id) AS id, displayName, display_name, timezone FROM venue WHERE id INSIDE $ids")
            .bind(("ids", venue_ids))
            .await
            .map_err(|e| e.to_string())?;
        let rows: Vec<serde_json::Value> = v_res.take(0).map_err(|e| e.to_string())?;
        for row in rows {
            if let Some(id) = json_str(&row, "id") {
                let display = json_str(&row, "display_name")
                    .or_else(|| json_str(&row, "displayName"))
                    .unwrap_or_else(|| "Unknown Venue".to_string());
                let tz = json_str(&row, "timezone").unwrap_or_else(|| "UTC".to_string());
                venue_meta.insert(id, (display, tz));
            }
        }
    }

    let mut name_rows: Vec<ContestNameRow> = Vec::new();
    let mut old_names: HashMap<String, String> = HashMap::new();

    for contest in &contests {
        let Some(contest_id) = json_str(contest, "id") else {
            continue;
        };
        let old_name = json_str(contest, "name").unwrap_or_default();
        old_names.insert(contest_id.clone(), old_name.clone());

        let start = json_str(contest, "start")
            .as_deref()
            .and_then(parse_contest_start)
            .unwrap_or_else(default_start);

        let game_id_list = games_by_contest.get(&contest_id).cloned().unwrap_or_default();
        let game_name_refs: Vec<&str> = game_id_list
            .iter()
            .filter_map(|gid| game_names.get(gid).map(String::as_str))
            .collect();

        let (venue_display, timezone) = venue_by_contest
            .get(&contest_id)
            .and_then(|vid| venue_meta.get(vid).cloned())
            .unwrap_or_else(|| ("Unknown Venue".to_string(), "UTC".to_string()));

        let base_name = default_contest_name(&game_name_refs, start, &timezone);
        name_rows.push(ContestNameRow {
            contest_id,
            base_name,
            venue_display,
            start,
            timezone,
        });
    }

    let resolved = resolve_unique_contest_names(&name_rows);
    let mut summary = BackfillSummary {
        total: contests.len(),
        ..Default::default()
    };

    for (contest_id, new_name) in resolved {
        let old_name = old_names.get(&contest_id).cloned().unwrap_or_default();
        if old_name == new_name {
            summary.unchanged += 1;
            continue;
        }
        summary.to_update += 1;
        summary.plans.push(BackfillPlan {
            contest_id,
            old_name,
            new_name,
        });
    }

    Ok(summary)
}

pub async fn apply_contest_name_backfill(db: &Db, plans: &[BackfillPlan]) -> Result<usize, String> {
    let mut updated = 0usize;
    for plan in plans {
        let key = record_id_to_key(&plan.contest_id, "contest");
        let record_id = RecordId::new("contest", key.as_str());
        db.query("UPDATE $record_id SET name = $name")
            .bind(("record_id", record_id))
            .bind(("name", plan.new_name.clone()))
            .await
            .map_err(|e| format!("update {}: {}", plan.contest_id, e))?;
        updated += 1;
    }
    Ok(updated)
}

fn json_str(row: &serde_json::Value, key: &str) -> Option<String> {
    row.get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn parse_contest_start(raw: &str) -> Option<DateTime<FixedOffset>> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .ok()
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S%.fZ")
                .ok()
                .map(|ndt| ndt.and_utc().fixed_offset())
        })
        .map(|dt| dt.with_timezone(&FixedOffset::east_opt(0).unwrap()))
}

fn default_start() -> DateTime<FixedOffset> {
    chrono::Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_contest_start_accepts_rfc3339() {
        let dt = parse_contest_start("2024-05-28T18:00:00Z").expect("parse");
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-05-28");
    }
}
