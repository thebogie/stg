# arango-to-surreal

Converts an ArangoDB smacktalk dump (zip from `arangodump`) into a single `.surql` file that SurrealDB can import. Designed for a **one-time production migration**: run once on ArangoDB production data to produce SurrealDB data and schema for the next version of STG.

**Conventions:** The generated `.surql` follows the project’s SurrealDB conventions and is suitable for re-import and backend use. See:

- **docs/SURREALDB_ID_CONVENTIONS.md** — Record IDs as `type::thing("table", "key")` with **raw key** only (no backticks/angle brackets); canonical app format `"table/key"`; backend uses `type::thing('table', $key)` with raw key.
- **docs/SURREALDB_EDGES.md** — All record refs as Thing; edges: `out` = source, `in` = target; document and edge ids and refs emitted as `type::thing("table", "key")`.

## One-shot production migration (recommended)

```bash
# From repo root: full schema (DEFINE TABLE/FIELD/INDEX) + data
cargo run -p arango-to-surreal -- path/to/smacktalk.zip -o output.surql --production

# Or use the import script (converts then imports; uses --production by default)
./scripts/arango-to-surreal-import.sh path/to/smacktalk.zip --fresh
```

- **`--production`**: Emit full production schema then data: `DEFINE TABLE ... SCHEMAFULL`, `DEFINE FIELD ... TYPE ...` for every table/field, and `DEFINE INDEX` where the backend expects lookups. Use this for the single run that cuts over from ArangoDB to SurrealDB in production. Import into an **empty** namespace/database (e.g. use `--fresh` with the script, or create ns/db before import).

## Other options

```bash
# Minimal schema only (DEFINE TABLE <name> SCHEMAFULL)
cargo run -p arango-to-surreal -- path/to/smacktalk.zip -o output.surql --schema

# Data only (no DEFINE; tables must already exist or be schemaless)
cargo run -p arango-to-surreal -- path/to/smacktalk.zip -o output.surql

# Remap player IDs only, or all IDs (optional; see "Record IDs: format vs remap" below)
cargo run -p arango-to-surreal -- path/to/smacktalk.zip -o output.surql --production --remap-player-ids
cargo run -p arango-to-surreal -- path/to/smacktalk.zip -o output.surql --production --remap-all-ids

# Example with production backup
cargo run -p arango-to-surreal -- ~/work/_backups/smacktalk.zip -o _build/smacktalk.surql --production
```

- **Input**: Path to a zip containing an ArangoDB dump (e.g. `smacktalk/dump.json` and `smacktalk/<collection>_<hash>.data.json.gz`).
- **Output**: One `.surql` file. Default: next to the zip with `.surql` extension, or use `-o path/to/file.surql`.
- **`--schema`**: Emit only `DEFINE TABLE <name> SCHEMAFULL;` for each table (no field types or indexes).

## Import into SurrealDB

Start SurrealDB (e.g. `docker compose -f deploy/docker-compose.deps.yml up -d surrealdb`). Then:

```bash
surreal import --conn http://localhost:50001 --user root --pass <password> --ns <namespace> --db <database> output.surql
```

For **production cutover**: take a final ArangoDB backup, stop the app, run the converter with `--production`, import the `.surql` into an empty SurrealDB namespace/database, point the next version of STG at SurrealDB, then bring the app back up. Use `./scripts/arango-to-surreal-import.sh ... --fresh` to reset the target ns/db before import so the production schema applies cleanly.

**One run is enough.** The converter emits edge `out`/`in` as `type::thing(...)` record ids, so leaderboard, achievements, contest list, and analytics work after import. You do **not** need to run a separate edge migration (e.g. `docs/surreal-migrate-edge-strings-to-things.surql`) when the data comes from this tool.

For running SurrealDB in Docker and using a UI (Surrealist) to run queries and reset data, see **docs/setup/SURREALDB_UI.md**.

Example (namespace and database matching your app):

```bash
surreal import --conn http://localhost:50001 --user root --pass root --ns stg_rd --db stg_rd _build/smacktalk.surql
```

## Record IDs: format vs remap

Arango and Surreal use different **formats** for record IDs (Arango: `collection/key`, Surreal: `table:key`). The tool always **translates format** (slash → colon, table name lowercased) and **preserves the key** so every reference stays valid.

**Optional remaps (relationships are preserved):**

- **`--remap-player-ids`** – New UUID per player; rewrites only player record ids and every reference to a player (`contest.creator_id`, `rating_*.*.player_id`, edge `resulted_in.in`, etc.). Use when you want players to match backend-created players (UUID).

- **`--remap-all-ids`** – New UUID for **every** document and edge. All record ids and all references (out/in, creator_id, player_id, scope_id when it’s a record ref) are rewritten so the graph is unchanged but uses Surreal-style opaque ids. Use when you want a clean break from Arango and don’t rely on old keys anywhere.

```bash
# Remap only players
cargo run -p arango-to-surreal -- path/to/smacktalk.zip -o output.surql --production --remap-player-ids

# Remap every document and edge (nodes and edges keep their relationships)
cargo run -p arango-to-surreal -- path/to/smacktalk.zip -o output.surql --production --remap-all-ids
```

With **`--remap-all-ids`**, the tool builds an old_key → new_uuid map for each document table (player, game, venue, contest, rating_latest, rating_history, schema_migrations, migration_lock), assigns a new UUID to each edge row, and rewrites every reference so no relationship is lost.

## What gets converted

- **Document collections** → Surreal tables with same name: `player`, `game`, `venue`, `contest`, `rating_latest`, `rating_history`, `schema_migrations`, `migration_lock`. Arango `_id`/`_key` become Surreal record ids (slash becomes colon).
- **Edge collections** → Surreal relation tables. All ArangoDB edges that exist in the dump are matched as follows (only these three are converted; any other edge collection in the zip is ignored):

  | Arango collection | Arango `_from` | Arango `_to` | Surreal `out` | Surreal `in` | Backend usage |
  |-------------------|----------------|--------------|---------------|--------------|---------------|
  | **played_at**     | contest        | venue        | contest       | venue        | Contest → venue |
  | **played_with**   | contest        | game         | contest       | game         | Contest → game  |
  | **resulted_in**   | contest        | player       | contest       | player       | Contest → player (place, result, points) |

  Each edge row also keeps any extra fields (e.g. `place`, `result`, `points` on resulted_in). `_from`/`_to` are coerced to string so refs always emit as `type::thing(...)` with table name lowercased to match document tables.

Insert order is document tables first, then edge tables, so referenced records exist before relations.

## SurrealDB convention (record ids as Thing)

The converter emits **record ids as Thing type** so the database stores proper record references, not plain strings. Keys are emitted as **raw** (no backticks or angle brackets), per **docs/SURREALDB_ID_CONVENTIONS.md** and **docs/SURREALDB_EDGES.md**.

| What | Output |
|------|--------|
| Document row `id` | `type::thing("table", "key")` |
| Edge row `id` | `type::thing("played_at", "key")` (etc.) |
| Edge `out` / `in` | `type::thing("contest", "key")`, `type::thing("player", "key")` (etc.) |
| **Reference fields** in documents | `player_id`, `creator_id`, `scope_id` → `type::thing("player", "key")` or `type::thing("game", "key")` when value is `"table/key"` or `"table:key"`; `null` stays `null`. |

If you have an existing DB where edges or these fields were imported as strings, run **docs/surreal-migrate-edge-strings-to-things.surql** once (see **docs/SURREALIST_EDGE_MIGRATION.md**).

## Consistency handled in the converter

- **Record ID mapping**: ArangoDB `_id` (`CollectionName/KeyName`) → SurrealDB record id `table_name:key_name`. All document and edge row `id`, and edge `out`/`in`, are emitted as `type::thing("table", "key")` (native record ids, not strings). **Document _key** is coerced to string (Arango may export numeric `_key`), so contest/player ids match. **Edge `out`/`in` and document reference fields** use a lowercased table name so `Contest/123` → `type::thing("contest", "123")` and match document table names; leaderboard and other `resulted_in.out IN (SELECT VALUE id FROM contest ...)` queries then match.
- **Edge conversion**: Arango edge collections → SurrealDB relation tables. `_from` → `out`, `_to` → `in`; emitted as `\`out\`` and `\`in\`` (backticks for reserved words) with `type::thing(...)`.
- **Reference fields**: `contest.creator_id`, `rating_latest.player_id`, `rating_history.player_id`, and `rating_*.scope_id` (when non-null) are converted from Arango `"collection/key"` strings to `type::thing("table", "key")`.
- **Field name deduplication**: If Arango has the same key with different casing (e.g. `createdat` and `createdAt`), the converter keeps a single key, preferring **camelCase** (e.g. `createdAt`).
- **Arango-only fields dropped**: `_id`, `_key`, `_rev`, `_label` (and for edges `_from`, `_to`) are never written to the `.surql` file.
- **Datetime fields**: Known date fields (e.g. `contest.start`, `contest.stop`, `player.createdAt`, `rating_latest.updated_at`, `rating_history.period_end`, `schema_migrations.appliedAt`) are **normalized** then emitted as `type::datetime("...")` so SurrealDB stores a real datetime type. String values are parsed (RFC3339, ISO8601 with space, optional fractional seconds) and re-emitted as RFC3339 UTC; numeric values are treated as Unix timestamps (seconds or milliseconds). This ensures consistent comparison with `time::now()` in SurrealQL (e.g. leaderboard time windows).
- **Invalid record refs**: Reference fields whose value is empty, `"null"`, or a string with no `:` or `/` are not converted to `type::thing`; they are left as the original value. This avoids emitting invalid `type::thing("", "")`.
- **Production schema**: With `--production`, the file includes `DEFINE TABLE ... SCHEMAFULL`, `DEFINE FIELD ... TYPE ...` (including `record<player>`, `record<contest>`, etc., and `option<datetime>` where applicable), and `DEFINE INDEX` for contest start/stop and rating_latest scope lookups. Any ambiguous or strict enforcement is documented in the generated file with `-- WARNING: Migration Note`.

Other normalizations you could add later if needed: trim whitespace on all string refs (already trimmed in `record_ref_to_surql`); normalize boolean-like strings (`"true"`/`"false"`) to `true`/`false` for known boolean fields; validate table/key format before emitting `type::thing`; or report unknown collections/fields for schema drift.
