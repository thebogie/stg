use chrono::{DateTime, FixedOffset, Utc};
use std::collections::HashMap;

use crate::timezone::convert_to_timezone;

/// Primary game label for a contest title (first game, or "+N more").
pub fn primary_game_label(game_names: &[&str]) -> String {
    let names: Vec<&str> = game_names
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    match names.as_slice() {
        [] => "Contest".to_string(),
        [one] => (*one).to_string(),
        [first, ..] => format!("{} (+{} more)", first, names.len() - 1),
    }
}

/// Local calendar label for contest start, e.g. `Thu May 28`.
pub fn format_contest_date_label(start: DateTime<FixedOffset>, timezone: &str) -> String {
    let utc = start.with_timezone(&Utc);
    if let Some(local) = convert_to_timezone(utc, timezone) {
        format!(
            "{} {} {}",
            local.format("%a"),
            local.format("%b"),
            local.format("%d")
        )
    } else {
        format!(
            "{} {} {}",
            start.format("%a"),
            start.format("%b"),
            start.format("%d")
        )
    }
}

/// Default contest title: `{Game} — {Weekday Mon D}` in the venue timezone.
pub fn default_contest_name(
    game_names: &[&str],
    start: DateTime<FixedOffset>,
    timezone: &str,
) -> String {
    format!(
        "{} — {}",
        primary_game_label(game_names),
        format_contest_date_label(start, timezone)
    )
}

pub fn disambiguate_with_venue(base: &str, venue_display: &str) -> String {
    let venue = venue_display.trim();
    if venue.is_empty() {
        base.to_string()
    } else {
        format!("{} @ {}", base, venue)
    }
}

pub fn disambiguate_with_time(
    base: &str,
    start: DateTime<FixedOffset>,
    timezone: &str,
) -> String {
    let utc = start.with_timezone(&Utc);
    if let Some(local) = convert_to_timezone(utc, timezone) {
        format!("{} · {}", base, local.format("%I:%M %p"))
    } else {
        base.to_string()
    }
}

/// One contest row used when batch-renaming (backfill).
#[derive(Debug, Clone)]
pub struct ContestNameRow {
    pub contest_id: String,
    pub base_name: String,
    pub venue_display: String,
    pub start: DateTime<FixedOffset>,
    pub timezone: String,
}

/// Resolve unique display names when several contests share the same base title.
pub fn resolve_unique_contest_names(rows: &[ContestNameRow]) -> HashMap<String, String> {
    let mut by_base: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, row) in rows.iter().enumerate() {
        by_base.entry(row.base_name.clone()).or_default().push(i);
    }

    let mut out = HashMap::new();
    for (_base, mut indices) in by_base {
        indices.sort_by_key(|&i| rows[i].start);
        if indices.len() == 1 {
            let i = indices[0];
            out.insert(rows[i].contest_id.clone(), rows[i].base_name.clone());
            continue;
        }
        let mut used: Vec<String> = Vec::new();
        for i in indices {
            let row = &rows[i];
            let with_venue = disambiguate_with_venue(&row.base_name, &row.venue_display);
            let candidate = if used.contains(&with_venue) {
                disambiguate_with_time(&with_venue, row.start, &row.timezone)
            } else {
                with_venue
            };
            used.push(candidate.clone());
            out.insert(row.contest_id.clone(), candidate);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample_start() -> DateTime<FixedOffset> {
        FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2024, 5, 28, 18, 0, 0)
            .unwrap()
    }

    #[test]
    fn default_name_single_game() {
        let name = default_contest_name(&["Azul"], sample_start(), "America/Chicago");
        assert!(name.starts_with("Azul — "));
        assert!(name.contains("May"));
    }

    #[test]
    fn default_name_multiple_games() {
        let name = default_contest_name(
            &["Azul", "Catan"],
            sample_start(),
            "America/Chicago",
        );
        assert!(name.starts_with("Azul (+1 more) — "));
    }

    #[test]
    fn resolve_unique_adds_venue_suffix() {
        let start = sample_start();
        let rows = vec![
            ContestNameRow {
                contest_id: "contest/a".into(),
                base_name: "Azul — Thu May 28".into(),
                venue_display: "Joe's Table".into(),
                start,
                timezone: "America/Chicago".into(),
            },
            ContestNameRow {
                contest_id: "contest/b".into(),
                base_name: "Azul — Thu May 28".into(),
                venue_display: "Main Library".into(),
                start,
                timezone: "America/Chicago".into(),
            },
        ];
        let names = resolve_unique_contest_names(&rows);
        assert_eq!(
            names.get("contest/a").map(String::as_str),
            Some("Azul — Thu May 28 @ Joe's Table")
        );
        assert_eq!(
            names.get("contest/b").map(String::as_str),
            Some("Azul — Thu May 28 @ Main Library")
        );
    }
}
