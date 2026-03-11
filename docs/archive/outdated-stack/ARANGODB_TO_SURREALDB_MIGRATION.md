# ArangoDB → SurrealDB Migration Plan

This document outlines migrating from ArangoDB to SurrealDB, including conversion of production data from `~/work/_backups/smacktalk.zip`.

## Strategy: Dev first, one-time production cutover

- **Dev**: Get everything running on your dev machine with SurrealDB (backend, converter, import, tests). No dual-write or phased migration.
- **Production**: When ready, shut down smacktalkgaming.com, run the converter on the latest prod backup, import into SurrealDB, switch the app to SurrealDB, then bring the site back up.

### Converter (done)

The **arango-to-surreal** tool turns a `smacktalk.zip` (ArangoDB dump) into a single `.surql` file for SurrealDB:

```bash
cargo run -p arango-to-surreal -- ~/work/_backups/smacktalk.zip -o _build/smacktalk.surql
surreal import --conn http://localhost:50001 --user root --pass <pass> --ns <ns> --db <db> _build/smacktalk.surql
```

See `tools/arango-to-surreal/README.md` for usage and import details.

---

## Current State Summary

### ArangoDB usage
- **Backend**: Rust (Actix), `arangors` 0.6; connection via `DatabaseConfig` (url, name, root_username, root_password).
- **Collections** (from backup and code):
  - **Document**: `player`, `contest`, `game`, `venue`, `rating_latest`, `rating_history`, `schema_migrations`, `migration_lock`
  - **Edge** (from/to): `played_at` (contest→venue), `played_with` (contest→game), `resulted_in` (contest→player; fields: place, result).
- **Backup format**: ArangoDB dump (arangodump). Zip contains:
  - `dump.json` (manifest: database name `smacktalk`)
  - Per collection: `{name}_{hash}.structure.json`, `{name}_{hash}.data.json.gz`
- **ID format**: `collection/key` (e.g. `player/2025041711441880300680500`). Edges use `_from`, `_to` with same format.

### Backend touchpoints
| Area | Files | Notes |
|------|--------|------|
| Connection / config | `main.rs`, `config.rs` | Arango URL, db name, auth |
| Analytics | `analytics/repository.rs`, `controller.rs`, `usecase.rs` | Heavy AQL (heatmap, leaderboard, trends, head-to-head, etc.) |
| Contest | `contest/repository.rs` | CRUD contest, create edges: played_at, played_with, resulted_in |
| Player | `player/repository.rs` | `collection("player")` |
| Game | `game/repository.rs` | `collection("game")`, AQL with edges |
| Venue | `venue/repository.rs` | `collection("venue")`, AQL with played_at/resulted_in |
| Ratings | `ratings/repository.rs`, `scheduler.rs` | rating_latest, rating_history, AQL |
| Health | `health.rs` | DB health check |
| Error | `error.rs` | `From<arangors::ClientError>` |
| Deploy / scripts | `deploy/docker-compose.production.yml`, `scripts/load-prod-data.sh`, `test.sh`, etc. | Arango container, env vars, restore |

---

## Phase 1: Discovery & Mapping (1–2 days)

**Goal**: Lock schema and query mapping so conversion and backend work are unambiguous.

### 1.1 Schema mapping
- [ ] List every Arango collection and edge with field list (from backup `.structure.json` and code).
- [ ] Define SurrealDB tables and relations:
  - **Tables**: `player`, `contest`, `game`, `venue`, `rating_latest`, `rating_history`, `schema_migrations`, `migration_lock`.
  - **Edges → relations**: SurrealDB uses record links and/or relation tables. Map:
    - `played_at` → e.g. `contest.venue` link or `played_at(contest, venue)` table.
    - `played_with` → `contest.game` or `played_with(contest, game)`.
    - `resulted_in` → relation table with `contest`, `player`, `place`, `result` (and any rating fields).
- [ ] Decide ID strategy: keep Arango `_key` as Surreal record id (e.g. `player:2025041711441880300680500`) for simpler 1:1 conversion and minimal backend ID handling changes.

### 1.2 Query mapping (AQL → SurrealQL)
- [ ] Catalog every AQL usage (grep `AqlQuery`, `aql_query`, `FOR .* IN`).
- [ ] For each, write equivalent SurrealQL (or multi-query + app logic) and document in a single mapping file (e.g. `docs/surrealql_mapping.md`). Focus on:
  - Graph traversals (e.g. `FOR c IN 1..1 OUTBOUND @player_id resulted_in`).
  - `DOCUMENT(_id)` → Surreal record links / `SELECT * FROM type WHERE id = ...`.
  - Aggregations (COLLECT, LENGTH, etc.) → SurrealQL `GROUP BY`, `count`, etc.
  - Date handling (DATE_NOW(), DATE_SUBTRACT, etc.) → SurrealQL time functions.

### 1.3 Backup inventory
- [ ] Extract `~/work/_backups/smacktalk.zip` and confirm list of collections and approximate row counts.
- [ ] Document any indexes from `.structure.json` and replicate in SurrealDB (indexes / unique constraints).

**Deliverable**: `docs/surrealql_mapping.md` (or similar) with schema + query mapping; optional small “playground” SurrealQL script that runs key queries against sample data.

---

## Phase 2: SurrealDB Environment & Schema (1–2 days)

**Goal**: Run SurrealDB locally and define schema so Phase 3 can load data.

### 2.1 Run SurrealDB
- [ ] Add SurrealDB to dev workflow (e.g. Docker Compose service, or embedded for tests).
- [ ] Use same namespace/database name as current DB (e.g. `stg_rd` or `smacktalk`) for parity.
- [ ] Document connection (URL, namespace, database) and env vars (e.g. `SURREAL_URL`, `SURREAL_NS`, `SURREAL_DB`).

### 2.2 Define schema in SurrealDB
- [ ] Create tables with compatible types (string ids, numbers, datetimes, etc.).
- [ ] Define relations (links or relation tables) for played_at, played_with, resulted_in.
- [ ] Add indexes / unique constraints that mirror Arango (e.g. rating_latest by player_id + scope).
- [ ] Optional: use SurrealDB schema definitions (SDL) in repo for version control.

**Deliverable**: SurrealDB running locally with empty schema; script or docs to recreate schema from scratch.

---

## Phase 3: Data Conversion (smacktalk.zip → SurrealDB) — done

**Goal**: One-off conversion of production backup into a SurrealDB-importable file.

- **Done**: The `arango-to-surreal` binary reads `smacktalk.zip`, converts document and edge collections to SurrealQL `INSERT` statements, and writes a single `.surql` file. Document tables are emitted first, then edge tables (`played_at`, `played_with`, `resulted_in`) with `in`/`out` as `type::thing(...)` record links.
- **Production**: Use the latest prod backup (e.g. `~/work/_backups/smacktalk.zip` or a fresh dump before cutover). Run the converter, then `surreal import` into the target SurrealDB (staging or production).
- **Validate**: Row counts are printed by the converter; after import, run key SurrealQL queries and spot-check against expected data.
- **Achievements / leaderboard / contest list empty?** If edges were ever imported as **strings** (e.g. from an older export or a different import path), run the one-time edge migration so `out`/`in` become record ids: see **docs/SURREALIST_EDGE_MIGRATION.md** and run **docs/surreal-migrate-edge-strings-to-things.surql**. The current converter already emits `type::thing()` for edges, so a fresh import does not need this step.

---

## Phase 4: Backend Abstraction (optional, 1–2 days)

**Goal**: If you want a safer, incremental switch, introduce a DB abstraction so Surreal can be plugged in behind the same interface.

### 4.1 Trait-based repository layer
- [ ] Define traits (e.g. `AnalyticsRepo`, `ContestRepo`, `PlayerRepo`, etc.) that match current repository method signatures.
- [ ] Keep existing Arango implementations as one implementation of these traits.
- [ ] Add SurrealDB implementations that use the Surreal Rust SDK and the SurrealQL mapping from Phase 1.
- [ ] Use feature flags or config (e.g. `DB_BACKEND=arango|surrealdb`) to choose implementation at startup.

### 4.2 When to skip
- If timeline is tight, you can skip a full abstraction and **replace Arango with Surreal in place** (Phase 5 only), at the cost of a single large PR and no easy runtime switch back.

**Deliverable**: Either trait + Arango/Surreal impls and config switch, or a short note that migration will be direct replacement.

---

## Phase 5: Replace Arango with Surreal in Backend (3–5 days)

**Goal**: Backend talks only to SurrealDB; all features work with converted data.

### 5.1 Dependencies and config
- [ ] Add `surrealdb` crate; remove or gate `arangors`.
- [ ] Extend `DatabaseConfig` (or add `SurrealConfig`) with Surreal URL, namespace, database, and auth if needed.
- [ ] In `main.rs`, create SurrealDB connection and pass namespace/database (e.g. `use_ns().use_db()`); inject into repos instead of `arangors::Database`.

### 5.2 Repository and usecase migration
- [ ] **Analytics**: Replace every AQL call in `analytics/repository.rs` with SurrealQL (or SDK calls) per mapping; preserve return types and error handling.
- [ ] **Contest**: Replace collection/edge creation with Surreal inserts/relations; keep same DTOs and API behavior.
- [ ] **Player / Game / Venue**: Replace `collection("...")` and AQL with Surreal `select`/`create`/`update` and SurrealQL.
- [ ] **Ratings**: Replace rating_latest and rating_history AQL (upsert, insert, get_leaderboard, get_rating_history, etc.) with SurrealQL.
- [ ] **Health**: Replace Arango health check with Surreal (e.g. simple query or ping).
- [ ] **Error**: Replace `From<arangors::ClientError>` with Surreal error type; keep `ApiError` contract.

### 5.3 Tests and scripts
- [ ] Update integration tests that assume Arango (e.g. testcontainers or test DB) to use SurrealDB.
- [ ] Update `scripts/load-prod-data.sh` (or equivalent) to: unzip backup → run conversion tool → load into Surreal (no arangorestore).
- [ ] Update `scripts/test.sh` and any CI that starts Arango to start SurrealDB instead.
- [ ] Run full test suite and manual smoke tests against converted data.

**Deliverable**: Backend runs end-to-end on SurrealDB with data from Phase 3; tests green; load-prod-data path uses converter.

---

## Phase 6: Production cutover (one-time)

**Goal**: Shut down site, convert backup, import into SurrealDB, switch backend, bring site back up.

1. **Pre-cutover**: Final ArangoDB backup (or confirm existing `smacktalk.zip` is current). Have SurrealDB deployed in production (e.g. in `docker-compose.production.yml` with persistent volume).
2. **Maintenance window**: Shut down smacktalkgaming.com (stop backend/stack).
3. **Convert**: Run `arango-to-surreal` on the production backup zip → produces `smacktalk.surql`.
4. **Import**: Run `surreal import` to load the `.surql` file into the production SurrealDB (correct `--ns` and `--db`).
5. **Switch**: Point backend config to SurrealDB (URL, namespace, database, auth); start backend.
6. **Smoke test**: Login, create contest, view analytics, leaderboard, ratings.
7. **Go live**: Bring site back up; monitor logs.
8. **Cleanup**: Remove Arango from docker-compose and scripts; update docs (e.g. `docs/setup/HYBRID_DEVELOPMENT.md`) for SurrealDB and the converter-based load-prod-data flow.

**Rollback**: Keep last Arango backup and (if possible) Arango volume until cutover is verified. If you need to revert, restore Arango, point config back to Arango, and restore from backup.

---

## Rollback and Validation

- **Rollback**: If you kept Phase 4, flip config back to Arango and restore from last Arango backup. If not, rollback = redeploy previous backend + Arango and restore Arango data.
- **Validation**: Throughout migration, compare:
  - Row counts per collection/table.
  - Key API responses (e.g. leaderboard, contest list, player profile, trends) between Arango and Surreal using same backup.

---

## Checklist Summary

| Phase | Focus | Key deliverable |
|-------|--------|------------------|
| 1 | Discovery & mapping | Schema + AQL→SurrealQL mapping doc |
| 2 | SurrealDB env & schema | SurrealDB running, empty schema defined |
| 3 | Data conversion | Tool: smacktalk.zip → SurrealDB + validation |
| 4 | (Optional) Abstraction | Trait + Arango/Surreal impls |
| 5 | Backend replacement | All repos/health/errors on SurrealDB |
| 6 | Production cutover | Prod on SurrealDB; runbook; cleanup |

**Estimated total**: ~10–16 days depending on abstraction and test coverage. Phase 3 (converter + validation) and Phase 5 (repository rewrite) are the largest chunks.

---

## Backup reference: smacktalk.zip layout

- `dump.json` – database name `smacktalk`.
- Collections (from zip listing):
  - `contest`, `game`, `player`, `venue` (documents)
  - `rating_latest`, `rating_history` (documents)
  - `played_at`, `played_with`, `resulted_in` (edges)
  - `schema_migrations`, `migration_lock` (documents)
- Each collection: `{name}_{hash}.structure.json`, `{name}_{hash}.data.json.gz`.
- Document format: `_id`, `_key`, `_rev`, plus business fields. Edge format: `_from`, `_to`, `_id`, `_key`, `_rev`, plus fields (e.g. `place`, `result` for resulted_in).

Use this layout in Phase 1 (inventory) and Phase 3 (converter input).
