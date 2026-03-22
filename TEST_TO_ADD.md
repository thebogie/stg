# Test backlog and coverage notes

Use this file to track **high-value** tests that catch production bugs (Surreal record IDs, cache, search tiers, scripts). Check items off as they land in `testing/tests/`, `back/api/tests/`, or CI.

## Done

- [x] **Contest + legacy numeric `player` keys** — `testing/tests/contest_legacy_player_integration_tests.rs`  
  Seeds `CREATE type::record('player', $key)` with a digit-only key, then `ContestRepository::create_contest` + `resulted_in` path (guards `type::uuid` misuse on player keys).
- [x] **`bgg_catalog` search tier** — `testing/tests/bgg_catalog_search_integration_tests.rs`  
  Seeds one `bgg_catalog` row and asserts `search_bgg_catalog` returns it (substring + scope).
- [x] **Shell sanity for prod test script** — `just scripts-check` runs `bash -n` on `scripts/full-prod-test.sh`.
- [x] **Capped import sort order + CSV parse** — unit tests in `back/api/src/bgg_catalog/import.rs` (`sort_newest_first`, `parse_bgg_csv_row`).
- [x] **Import CLI env / placeholder parsing** — `back/api/src/bgg_catalog/import_cli.rs` (`parse_bgg_import_max_rows_from_str`, `looks_like_doc_placeholder`); binary delegates here.

## In progress / next (priority order)

1. **BGG CSV import** — `import_csv_from_path` with a tiny temp CSV + `max_rows`, assert **newest-by-year** selection end-to-end (needs scoped `Db`).
2. **Contest outcomes: mixed IDs** — UUID creator session + legacy `player_id` in outcomes (HTTP or repo).
3. **Analytics / leaderboard** — one query path with `type::record('player', $key)` using a **numeric** key (same class as contest bug).
4. **Player cache** — after `update_email` / `update_handle`, next read matches (see `back/api/tests/repository_cache_test.rs` patterns).
5. **E2E (env-gated)** — Playwright game search when `bgg_catalog` or fixture CSV exists.
6. **CI** — optional: run `just scripts-check` in pipeline; optional `shellcheck` if installed.

## Reference: record ID rules

- See `docs/SURREALDB_ID_CONVENTIONS.md`. Player keys are often **opaque strings** (including long digits); do not wrap in `type::uuid` unless the key is a real UUID.
- New registrations use UUID string keys; `PlayerRepository::create` correctly uses `type::uuid($key)` for that path only.

## Commands

```bash
# Legacy contest integration test
cargo test -p testing --test contest_legacy_player_integration_tests -- --nocapture

# bgg_catalog search integration test
cargo test -p testing --test bgg_catalog_search_integration_tests -- --nocapture

# Script syntax (no execution)
just scripts-check

# Backend unit tests (BGG import logic)
cargo test -p backend --lib bgg_catalog::
```
