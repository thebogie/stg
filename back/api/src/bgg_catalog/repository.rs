//! Query `bgg_catalog` for game search (substring + rank order).

use crate::db::Db;
use shared::models::game::{Game, GameSource};
use std::collections::HashSet;

/// Search BGG catalog rows; maps to the same [`Game`] shape as the old CSV / API tiers (`bgg_*` ids).
pub async fn search_bgg_catalog(
    db: &Db,
    query: &str,
    limit: usize,
    exclude_bgg_ids: &HashSet<i32>,
) -> Vec<Game> {
    if limit == 0 {
        return Vec::new();
    }
    let q = query.trim();
    if q.len() < 2 {
        return Vec::new();
    }
    let q_owned = q.to_string();

    let res = if exclude_bgg_ids.is_empty() {
        db.query(
            "SELECT bgg_id, name, year_published, rank FROM bgg_catalog \
             WHERE string::contains(string::lowercase(name), string::lowercase($q)) \
             ORDER BY rank ASC \
             LIMIT $limit",
        )
        .bind(("q", q_owned))
        .bind(("limit", limit as i64))
        .await
    } else {
        let exclude: Vec<i32> = exclude_bgg_ids.iter().copied().collect();
        db.query(
            "SELECT bgg_id, name, year_published, rank FROM bgg_catalog \
             WHERE string::contains(string::lowercase(name), string::lowercase($q)) \
             AND bgg_id NOTINSIDE $exclude \
             ORDER BY rank ASC \
             LIMIT $limit",
        )
        .bind(("q", q_owned))
        .bind(("exclude", exclude))
        .bind(("limit", limit as i64))
        .await
    };

    let Ok(mut res) = res else {
        return Vec::new();
    };

    let rows: Vec<serde_json::Value> = match res.take(0) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    rows.into_iter().filter_map(row_to_bgg_game).collect()
}

fn row_to_bgg_game(v: serde_json::Value) -> Option<Game> {
    let bgg_id = v.get("bgg_id").and_then(|x| x.as_i64()).map(|n| n as i32)?;
    let name = v.get("name").and_then(|x| x.as_str())?.to_string();
    let year_published = v
        .get("year_published")
        .and_then(|x| x.as_i64())
        .map(|n| n as i32);
    Some(Game {
        id: format!("bgg_{}", bgg_id),
        rev: String::new(),
        name,
        year_published,
        bgg_id: Some(bgg_id),
        description: None,
        source: GameSource::BGG,
    })
}
