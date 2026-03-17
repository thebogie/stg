# SurrealDB edge & ID query audit (ArangoDB legacy)

**One-time move from ArangoDB:** For a **fresh** import (restart SurrealDB → run **arango-to-surreal** on your dump → import the `.surql`), no edge migration is needed; the converter emits `type::record()` for edges. This audit is for fixing backend query patterns and for DBs that were already imported with string edges.

Queries that touch edge tables (`resulted_in`, `played_with`, `played_at`) or use record IDs from the API can fail on SurrealDB when:

1. **Edges were imported as strings** — Run the one-time migration: **docs/surreal-migrate-edge-strings-to-things.surql** (see **docs/SURREALIST_EDGE_MIGRATION.md**).
2. **`type::record('table', $key)` with string key** — Binding a string key can fail to match stored record IDs in some setups. Prefer **binding a `RecordId`** (e.g. `RecordId::new("player", key)`) and using `$record_id` in the query.
3. **`INSIDE $ids` with string array** — SurrealDB may not match a string array to a record-id column. Prefer **binding a `Vec<RecordId>`** built from the same source as the column (e.g. select raw `in`/`out` and collect `RecordId`s).

This doc lists call sites and the pattern each uses so we can fix them consistently.

---

## Safe patterns to use

| Pattern | Use when | Example |
|--------|----------|---------|
| **RecordId binding** | Single-record lookup (player, contest, etc.) | `WHERE id = $record_id` with `bind(("record_id", RecordId::new("player", key)))` |
| **Raw edge column → RecordId list** | Getting IDs for INSIDE from an edge table | `SELECT \`in\` AS contest_id FROM resulted_in WHERE \`out\` = $record_id`, deserialize as `Option<RecordId>`, collect and bind as `$contest_ids` |
| **Typed struct for edge rows** | When you need both record IDs and scalars | Struct with `contest_id: Option<RecordId>`, `place: Option<i64>`; deserialize with `.take(n)` into `Vec<Struct>` |

---

## Audit: analytics/repository.rs

| Location | Query / usage | Current pattern | Recommended |
|----------|----------------|-----------------|-------------|
| **get_player_achievements** | Player stats + contest IDs, then games/venues INSIDE | RecordId for player; single-statement queries only (player, contest IDs, games, venues each separate); INSIDE $contest_ids | ✅ Uses RecordId + raw in; games and venues are two single-statement queries (no multi-statement). |
| **get_player_stats** | resulted_in counts by player | `$record_id` binding | ✅ Safe |
| **get_player_stats_by_thing** | resulted_in counts | `$record_id` binding | ✅ Safe |
| **fetch_opponent_stats** | My contests: `in` AS contest_id; then resulted_in INSIDE | type::record('player', $key) for first query; strings_to_record_id_array(contest_ids) for INSIDE | Use RecordId for player; keep RecordId array for INSIDE (contest_ids from thing_to_record_id). |
| **get_player_stats_by_id_str** | Dual out/in string compare | string::replace(string::concat(\`out\`), …) = $id_str | Fallback when RecordId binding fails; keep as-is. |
| **get_player_stats_for_me_by_email** | Resolve player then get_player_stats_by_thing | By-thing path | ✅ Safe |
| **Leaderboard / get_leaderboard** | resulted_in GROUP BY player_id | string::concat(\`out\`) AS player_id | Returns scalars; INSIDE $ids for player lookup may use string array — prefer RecordId array if leaderboard-by-ids is added. |
| **get_contest_stats** | resulted_in by contest | type::record('contest', $key) | Prefer RecordId binding if key comes from API. |
| **get_my_performance_trends** | resulted_in by player | type::record('player', $key) | Prefer RecordId binding. |
| **get_player_game_performance** | resulted_in, played_with by player / INSIDE $ids | type::record('player', $key); INSIDE $ids (string?) | Use RecordId for player; INSIDE with RecordId array (from raw edge column). |
| **get_head_to_head_record** | resulted_in subquery | type::record('player', $my_key) | Prefer RecordId binding. |
| **get_my_game_performance** (engine) | resulted_in, played_with | type::record('player', $key); INSIDE $ids | Use RecordId + RecordId array. |
| **get_venue_contest_history** | played_at by venue | type::record('venue', $key) | Prefer RecordId binding. |
| **get_recent_contests** | resulted_in, played_with INSIDE $rids | String array $rids | Use RecordId array. |

---

## Audit: contest/repository.rs

| Location | Query / usage | Current pattern | Recommended |
|----------|----------------|-----------------|-------------|
| **create_*_relation** | INSERT into played_at, played_with, resulted_in | type::record('contest', $key) etc. | ✅ Inserts; keys from app. Keep. |
| **get_contests_for_player** | resulted_in WHERE out = player | type::record('player', $player_key) | Prefer RecordId binding. |
| **contest list (search)** | played_at / played_with / resulted_in filters | INSIDE, type::record | Prefer RecordId arrays for INSIDE. |

---

## Audit: client_analytics/repository.rs

| Location | Query / usage | Current pattern | Recommended |
|----------|----------------|-----------------|-------------|
| **Player contests, games, venues** | resulted_in, played_with, played_at INSIDE $contest_ids | RecordId array from strings_to_record_id_array | ✅ Keep; ensure contest_ids come from raw edge or thing_to_record_id. |

---

## What to do first

1. **Fresh one-time move:** Restart SurrealDB, run **arango-to-surreal**, import the `.surql` — no edge migration. **Existing DB with string edges?** Run the edge migration (fixes most “empty or wrong” issues):  
   ```bash
   surreal sql --endpoint <url> --ns <ns> --db <db> -f docs/surreal-migrate-edge-strings-to-things.surql
   ```  
   Then verify in Surrealist: `SELECT out, in FROM resulted_in LIMIT 1;` — you should see record id values (e.g. `contest:⟨...⟩`), not plain quoted strings.  
   See **docs/SURREALIST_EDGE_MIGRATION.md**.

2. **Achievements**  
   - **get_player_achievements** now: uses RecordId for player; gets contest IDs from raw `in` (or fallback `string::concat(\`in\`)` when edges are strings); binds RecordId array or string array for INSIDE.  
   - If games/venues are still 0 after code deploy, the edge migration in step 1. The fallback helps when `in` doesn’t deserialize as RecordId.

3. **Ongoing**  
   Systematically replace `type::record('player', $key)` with **RecordId** binding and use **RecordId arrays** for INSIDE on edge columns (see audit table above).

---

## Other ArangoDB-style items (converted or optional)

### Backend (all SurrealDB now)

- **Multi-statement queries:** Converted to single-statement only (see **docs/SURREALDB_QUERY_CONVENTIONS.md**). No remaining `SELECT ...; SELECT ...` in one `.query()`.
- **type::record('table', $key) in read paths:** Many SELECT/WHERE still bind a string `$key`. The audit table above marks which would benefit from **RecordId** binding (e.g. get_head_to_head_record, fetch_opponent_stats, get_my_performance_trends, get_player_game_performance, get_contest_stats, get_venue_contest_history, contest get_contests_for_player_and_game, client_analytics). Converting these is **optional** and can be done incrementally; RecordId is preferred where we had matching issues (e.g. achievements, stats).
- **string::replace(string::concat(\`out\`), …) = $id_str:** Used in get_player_stats_by_id_str and get_player_stats_for_me_by_email fallbacks so "me" and string IDs still work when RecordId binding fails. **Keep as-is** (documented fallback).
- **GROUP ALL)[0].count:** SurrealQL pattern for scalar from aggregate; not Arango. **Keep.**

### Shared crate (docs and naming)

- **Doc comments:** Several types in `shared` still say "ArangoDB document ID" or "ArangoDB will set id and rev". Update to "SurrealDB record id" or "record id (table/key)" for accuracy. The **serde(rename = "_id")** / **serde(rename = "_rev")** can stay if the API or legacy clients still use those JSON keys; only the comments need to reflect SurrealDB.
- **shared/src/models/relations.rs:** Documents edges with _id, _rev, _from, _to. SurrealDB edge tables use `in`/`out`; this model may be for a different layer or legacy. No change required unless that code path is used for SurrealDB edge rows.

### Not present

- No **arangors**, **AQL**, or **collection()** calls in the backend; the codebase is SurrealDB-only.

---

## References

- **docs/SURREALDB_EDGES.md** — Edge table convention (in/out), migration, converter.
- **docs/SURREALIST_EDGE_MIGRATION.md** — When and how to run the edge migration.
- **docs/SURREALDB_ID_CONVENTIONS.md** — RecordId, INSIDE, and query patterns.
- **docs/archive/outdated-stack/ARANGODB_TO_SURREALDB_MIGRATION.md** — Original migration plan.
