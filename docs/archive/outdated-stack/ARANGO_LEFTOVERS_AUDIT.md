# ArangoDB leftovers audit

Ways to comb the codebase for remaining ArangoDB assumptions after the SurrealDB migration.

## 1. Grep patterns (run from repo root)

```bash
# Explicit names / env
rg -i 'arangodb|arango|aql|AQL' --type-add 'code:*.{rs,js,ts,json}' -t code

# Arango default port
rg '8529' .

# Arango document/edge fields (_id, _key, _from, _to — SurrealDB uses record id and edge in/out)
rg '_key|_from|_to|\b_id\b' --type rust

# AQL-style query strings (FOR x IN y, FILTER, RETURN, COLLECT, LENGTH(...))
rg 'FOR\s+\w+\s+IN\s+|RETURN\s+\{|COLLECT\s+|LENGTH\s*\(' --type rust

# Legacy test / crate names
rg 'arangors|legacy-arangors|db_integration_fixed|arangodb_url' .
```

## 2. Known leftovers (by category)

### Config / env (intentional fallbacks or test-only)

| Location | What | Action |
|----------|------|--------|
| `back/api/src/config.rs` | `ARANGO_*` env fallbacks for `SURREAL_*`; test defaults `http://localhost:8529`, `prod-arango:8529`, `arangodb:8529` | Optional: remove fallbacks and 8529 URLs once no Arango deploy remains; tests can use Surreal URL. |
| `scripts/start-back.sh` | `ARANGO_BACKUP_ZIP`, “Arango→Surreal” import path | Keep if you still import from Arango dump; else simplify wording. |

### Dead AQL in Rust (not executed by SurrealDB path)

| Location | What | Action |
|----------|------|--------|
| `back/api/src/analytics/repository.rs` | `build_win_rate_query`, `build_total_wins_query`, `build_total_contests_query` — return AQL strings with `FOR … IN`, `_to`, `player._id` | Remove the three `#[allow(dead_code)]` functions, or reimplement as SurrealQL if you want that leaderboard path. |
| `back/api/src/analytics/repository.rs` (tests) | Test code building `DatabaseConfig` with `url: "http://localhost:8529"` | Point to Surreal URL or remove if redundant. |

### Debug endpoint (broken on SurrealDB)

| Location | What | Action |
|----------|------|--------|
| `back/api/src/analytics/usecase.rs` | `debug_database()` passes an **AQL** string (`RETURN { LENGTH(played_with), … }`) to the repo | Replace with a SurrealQL debug query or remove the debug endpoint. |
| `back/api/src/analytics/repository.rs` | `debug_database(&self, query: &str)` runs the given string | If keeping: document that it must be SurrealQL when backend is SurrealDB. |

### Comments / docs

| Location | What | Action |
|----------|------|--------|
| `back/api/src/contest/repository.rs` | Comments “ArangoDB format”, “ArangoDB _id format”, “Will be set by ArangoDB” | Reword to “record id” / “SurrealDB” or remove. |

### Tests (legacy / disabled)

| Location | What | Action |
|----------|------|--------|
| ~~analytics/usecase_tests.rs~~ | ~~legacy-arangors-tests~~ | **Removed.** |
| ~~ratings/usecase_tests.rs~~ | ~~Same~~ | **Removed.** |
| ~~ratings/scheduler_tests.rs~~ | ~~Same~~ | **Removed.** |
| ~~migration/timezone_migration_tests.rs~~ | ~~arangors mock~~ | **Removed.** |
| ~~tests/database_integration_test.rs~~ | ~~ArangoDB integration~~ | **Removed.** |
| `testing/src/env.rs` | `arangodb_url()` (alias to `surrealdb_url`) | Keep for compatibility with old integration test, or remove and update callers. |

### Docs / Justfile

| Location | What | Action |
|----------|------|--------|
| `docs/DIRECTORY_AUDIT.md`, `docs/CI_CD.md`, `docs/PROJECT_STRUCTURE.md`, `docs/ARANGODB_TO_SURREALDB_MIGRATION.md`, `docs/SURREALDB_*.md`, `Justfile` | References to Arango, arango-to-surreal, 8529, AQL | Update or archive; keep migration docs in `docs/archive/` if historical. |
| `tools/arango-to-surreal/` | One-off conversion tool | Keep if you still have Arango dumps to migrate; else archive. |

## 3. One-off “full comb” script

From repo root:

```bash
echo "=== arango/arangoDB/AQL ==="
rg -i 'arangodb|arango|\.aql\b|AQL' --type-add 'code:*.{rs,js,ts,json}' -t code -l

echo "=== Port 8529 ==="
rg '8529' -l

echo "=== _id / _key / _from / _to (Arango fields) ==="
rg '_key|_from|_to' --type rust -l
rg '\b_id\b' --type rust -l

echo "=== AQL-like strings (FOR x IN y, RETURN {) ==="
rg 'FOR\s+\w+\s+IN\s+' --type rust -l
rg 'RETURN\s+\{' --type rust -l

echo "=== arangors / legacy test features ==="
rg 'arangors|legacy-arangors|db_integration_fixed|arangodb_url' -l
```

Use this file as a checklist: run the greps, then work through “Action” in the tables (remove dead AQL, fix debug query, update comments, then tests and docs).
