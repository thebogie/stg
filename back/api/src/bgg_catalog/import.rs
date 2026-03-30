//! Stream `boardgames_ranks.csv` into `bgg_catalog` (UPSERT by record id `bgg_catalog:<bgg_id>`).

use std::cmp::Ordering;
use std::path::Path;

use anyhow::Context;
use csv::StringRecord;
use surrealdb::types::RecordId;

use crate::db::Db;

/// Result of a full-file import.
#[derive(Debug, Clone, Default)]
pub struct ImportStats {
    pub rows_read: usize,
    pub rows_upserted: usize,
    pub rows_skipped: usize,
}

#[derive(Debug, Clone)]
struct ParsedRow {
    bgg_id: i32,
    name: String,
    year_published: Option<i32>,
    rank: Option<i32>,
}

fn parse_bgg_csv_row(
    raw: &StringRecord,
    id_i: usize,
    name_i: usize,
    year_i: Option<usize>,
    rank_i: Option<usize>,
) -> Option<ParsedRow> {
    let id: i32 = raw.get(id_i).and_then(|s| s.parse().ok())?;
    let name = raw.get(name_i).unwrap_or("").trim();
    if name.is_empty() {
        return None;
    }

    let year_published: Option<i32> = year_i
        .and_then(|j| raw.get(j))
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse().ok());
    let rank: Option<i32> = rank_i
        .and_then(|j| raw.get(j))
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse().ok());

    Some(ParsedRow {
        bgg_id: id,
        name: name.to_string(),
        year_published,
        rank,
    })
}

/// Newest publication year first; missing year last. Tie-break: better BGG rank (lower number), then id.
fn sort_newest_first(a: &ParsedRow, b: &ParsedRow) -> Ordering {
    let year_ord = match (a.year_published, b.year_published) {
        (Some(ya), Some(yb)) => yb.cmp(&ya),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    };
    if year_ord != Ordering::Equal {
        return year_ord;
    }
    match (a.rank, b.rank) {
        (Some(ra), Some(rb)) => ra.cmp(&rb),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.bgg_id.cmp(&b.bgg_id),
    }
}

async fn upsert_parsed_row(db: &Db, row: &ParsedRow, import_batch: &str) -> anyhow::Result<()> {
    let key = row.bgg_id.to_string();
    let rid = RecordId::new("bgg_catalog", key.as_str());

    db.query(
        "UPSERT $rid SET \
         bgg_id = $bgg_id, \
         name = $name, \
         year_published = $year, \
         rank = $rank, \
         imported_at = time::now(), \
         import_batch = $batch",
    )
    .bind(("rid", rid))
    .bind(("bgg_id", row.bgg_id))
    .bind(("name", row.name.clone()))
    .bind(("year", row.year_published))
    .bind(("rank", row.rank))
    .bind(("batch", import_batch.to_string()))
    .await
    .with_context(|| format!("upsert bgg_catalog id={}", row.bgg_id))?;

    Ok(())
}

/// Import or refresh the BGG ranks CSV into `bgg_catalog`. Idempotent per `bgg_id` (latest row wins).
///
/// `max_rows`: when `Some(n)`, loads valid rows, sorts by **newest `yearpublished` first** (unknown years
/// last), then upserts only the first `n` after sorting. `None` streams the full file in CSV order.
pub async fn import_csv_from_path(
    db: &Db,
    path: &Path,
    import_batch: &str,
    max_rows: Option<usize>,
) -> anyhow::Result<ImportStats> {
    let f = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut rdr = csv::ReaderBuilder::new().flexible(true).from_reader(f);

    let headers = rdr.headers().context("CSV headers")?;
    let id_i = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("id"))
        .context("missing id column")?;
    let name_i = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("name"))
        .context("missing name column")?;
    let year_i = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("yearpublished"));
    let rank_i = headers.iter().position(|h| h.eq_ignore_ascii_case("rank"));

    let mut stats = ImportStats::default();

    if let Some(limit) = max_rows {
        let mut collected: Vec<ParsedRow> = Vec::new();

        for (i, result) in rdr.records().enumerate() {
            let raw = match result {
                Ok(r) => r,
                Err(e) => {
                    log::debug!("BGG CSV row {} skipped: {}", i + 2, e);
                    stats.rows_skipped += 1;
                    continue;
                }
            };

            match parse_bgg_csv_row(&raw, id_i, name_i, year_i, rank_i) {
                Some(row) => collected.push(row),
                None => stats.rows_skipped += 1,
            }
        }

        stats.rows_read = collected.len();
        collected.sort_unstable_by(sort_newest_first);

        log::info!(
            "bgg_catalog import: max_rows={limit} — sorted {} valid rows by year (newest first), upserting top {limit}",
            stats.rows_read
        );

        for row in collected.into_iter().take(limit) {
            upsert_parsed_row(db, &row, import_batch).await?;
            stats.rows_upserted += 1;

            if stats.rows_upserted > 0 && stats.rows_upserted % 5000 == 0 {
                log::info!(
                    "bgg_catalog import: {} rows upserted (batch={})",
                    stats.rows_upserted,
                    import_batch
                );
            }
        }
    } else {
        for (i, result) in rdr.records().enumerate() {
            let raw = match result {
                Ok(r) => r,
                Err(e) => {
                    log::debug!("BGG CSV row {} skipped: {}", i + 2, e);
                    stats.rows_skipped += 1;
                    continue;
                }
            };

            let row = match parse_bgg_csv_row(&raw, id_i, name_i, year_i, rank_i) {
                Some(r) => r,
                None => {
                    stats.rows_skipped += 1;
                    continue;
                }
            };

            stats.rows_read += 1;

            upsert_parsed_row(db, &row, import_batch).await?;

            stats.rows_upserted += 1;

            if stats.rows_upserted > 0 && stats.rows_upserted % 5000 == 0 {
                log::info!(
                    "bgg_catalog import: {} rows upserted (batch={})",
                    stats.rows_upserted,
                    import_batch
                );
            }
        }
    }

    log::info!(
        "bgg_catalog import done: read={} upserted={} skipped={} batch={}",
        stats.rows_read,
        stats.rows_upserted,
        stats.rows_skipped,
        import_batch
    );

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use csv::StringRecord;

    fn row(id: i32, name: &str, year: Option<i32>, rank: Option<i32>) -> ParsedRow {
        ParsedRow {
            bgg_id: id,
            name: name.to_string(),
            year_published: year,
            rank,
        }
    }

    #[test]
    fn sort_newest_first_orders_by_year_desc() {
        let mut rows = [
            row(1, "a", Some(2010), Some(5)),
            row(2, "b", Some(2020), Some(1)),
            row(3, "c", Some(2015), Some(1)),
        ];
        rows.sort_unstable_by(sort_newest_first);
        assert_eq!(rows[0].bgg_id, 2);
        assert_eq!(rows[1].bgg_id, 3);
        assert_eq!(rows[2].bgg_id, 1);
    }

    #[test]
    fn sort_newest_first_puts_missing_year_last() {
        let mut rows = [row(1, "a", None, None), row(2, "b", Some(2019), Some(10))];
        rows.sort_unstable_by(sort_newest_first);
        assert_eq!(rows[0].bgg_id, 2);
        assert_eq!(rows[1].bgg_id, 1);
    }

    #[test]
    fn sort_newest_first_tie_year_uses_rank_then_id() {
        let mut rows = [
            row(10, "a", Some(2020), Some(5)),
            row(11, "b", Some(2020), Some(1)),
            row(12, "c", Some(2020), None),
        ];
        rows.sort_unstable_by(sort_newest_first);
        assert_eq!(rows[0].bgg_id, 11);
        assert_eq!(rows[1].bgg_id, 10);
        assert_eq!(rows[2].bgg_id, 12);
    }

    #[test]
    fn parse_bgg_csv_row_reads_standard_headers() {
        let raw = StringRecord::from(vec!["42", "Test Game", "2018", "99"]);
        let p = parse_bgg_csv_row(&raw, 0, 1, Some(2), Some(3)).expect("parse");
        assert_eq!(p.bgg_id, 42);
        assert_eq!(p.name, "Test Game");
        assert_eq!(p.year_published, Some(2018));
        assert_eq!(p.rank, Some(99));
    }
}
