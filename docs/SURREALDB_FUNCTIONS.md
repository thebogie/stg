# SurrealDB custom functions

Optional SurrealQL functions reduce round-trips and centralize logic in the database. **Whenever there is an option to use a SurrealDB function, we use it:** the backend tries the function first and falls back to inline/multi-query only when the function is not defined or fails. The canonical definitions live in **`tools/arango-to-surreal/surreal-functions.surql`**; apply that file (or run `./scripts/apply-surreal-functions.sh`) in your namespace/database so the backend can use them. **SurrealDB v3:** use `type::record(...)` in function bodies (v2 used `type::thing`).

## Applying the functions

**First-time conversion (ArangoDB → SurrealDB):** When you run the converter in production mode (`arango-to-surreal ... --production`), the output `.surql` file **already includes** the application functions at the end (from `tools/arango-to-surreal/surreal-functions.surql`). A single `surreal import` of that file gives you schema + data + functions. No separate step needed for the first convert. See **tools/arango-to-surreal/README.md**. **Later iterations** (schema or function changes) use migration scripts (e.g. new `.surql` files applied after import).

**Option A — Automatic (dev):** If you start deps with `./scripts/start-deps.sh` and `tools/arango-to-surreal/surreal-functions.surql` exists, the script applies it after the main data import. You should then see the functions in SurrealDB (e.g. in Surrealist under the same namespace/database).

**Option B — Manual (recommended if Option A didn’t run or failed):** With SurrealDB already running (e.g. after `start-deps.sh`):

```bash
./scripts/apply-surreal-functions.sh
```

This uses the same connection (Docker network) and NS/DB as start-deps and prints any import errors. If you see “function does not exist” in the app, run this once and check the output.

**Option C — Surrealist:** Open Surrealist, select namespace/database (e.g. `stg_rd` / `stg_rd`), then paste and run the contents of **`tools/arango-to-surreal/surreal-functions.surql`**.

After that, the functions are available in that namespace/database.

## Defined functions

### `fn::contest_row($key)`

Returns a single contest row by key (string or numeric). Handles Arango-import style numeric keys by trying both `type::record('contest', $key)` and `type::record('contest', int::parse($key))` when the key is numeric.

**Example (Surrealist / CLI):** SurrealDB v3 requires `FROM`; use a single-row source so the client gets one row:

```sql
SELECT fn::contest_row("10860886") AS result FROM [1];
```

**Backend use:** One round-trip to get the contest record instead of two (string then int fallback in Rust).

### `fn::contest_with_edges($key)`

Returns one object: `{ contest, venue_id, game_ids, outcomes }`. Contest is the full contest row; `venue_id` is a single record id (or NONE); `game_ids` is an array of record ids; `outcomes` is an array of `{ player_id, place, result }`.

**Example:**

```sql
SELECT fn::contest_with_edges("10860886") AS result FROM [1];
```

**Backend use:** One round-trip for contest + all edge ids. The backend can then batch-fetch venue, games, and players by id and build the full DTO.

## Backend integration

- **Without functions:** The contest repository uses multiple queries (contest by key, then played_at, played_with, resulted_in, then venue/game/player rows). This works without applying the `.surql` file.
- **With functions:** The backend calls `SELECT fn::contest_with_edges($key) AS result FROM [1]` and maps the result to `ContestDto`, then optionally batch-fetches venue/games/players. If the function is not defined, the backend falls back to the existing multi-query logic.

See `back/api/src/contest/repository.rs`: `find_details_by_id_impl` can be extended to try the function first when desired.

## SurrealQL version notes

- **string::is::numeric:** In SurrealDB 3.x this may be `string::is_numeric()` (underscore). If `DEFINE FUNCTION` fails, try the variant that matches your server version.
- **int::parse:** If your version doesn’t support `int::parse($key)`, the fallback in Rust (try string key then int key) remains the source of truth; you can define a string-only `contest_row` that omits the `OR (string::is::numeric(...))` branch.

## Candidates for more functions

These flows do multiple round-trips today and are good candidates for SurrealDB functions (one round-trip, same pattern as `fn::contest_with_edges`).

| Flow | Where | Current round-trips | Suggested function |
|------|--------|---------------------|---------------------|
| **Contest → venue** | `client_analytics/repository.rs` `get_venue_for_contest` | 2 (played_at edge, then venue by id) | `fn::contest_venue($key)` → venue row or NONE |
| **Contest → game** | `client_analytics/repository.rs` `get_game_for_contest` | 2 (played_with edge, then game by id) | `fn::contest_game($key)` → game row or NONE |
| **Player → contest IDs** | `client_analytics/repository.rs` `contest_ids_for_player` | 1 (resulted_in); then callers do a 2nd query for contest rows | `fn::player_contest_ids($player_key)` → array of contest record ids (optional; already 1 query) |
| **Player → contests with edges** | `get_filtered_contests` + then per-contest venue/game/participants | 2 + N (contests, then venue/game/players per contest) | `fn::player_contests_with_edges($player_key, $start, $end)` → array of `{ contest, venue_id, game_id, outcomes }` |
| **Contest participants** | `client_analytics/repository.rs` `get_contest_participants` | 2 (resulted_in, then players by id) | `fn::contest_participants($key)` → array of `{ player_id, handle, firstname, lastname, place, result, points }` |
| **Venue + contests at venue** | Venue details page: venue by id, then contest search by venue_id | 2+ | `fn::venue_with_contest_ids($key)` → `{ venue, contest_ids }` or full contest summaries |
| **Game + contests (game history)** | Game details: game by id, then played_with + contest list | 2–3 | `fn::game_with_contest_ids($key)` → `{ game, contest_ids }` or contest rows |
| **Analytics: game performance** | `analytics/repository.rs` `get_my_game_performance` | 3 (resulted_in, contest start times, played_with) | `fn::player_game_performance($player_key)` → aggregated by game in one round-trip |
| **Analytics: daily active players** | `get_daily_active_players` | 2 (contests in period, resulted_in) | `fn::daily_active_players($days)` → array of `{ day, count }` |

Two small helpers are already added: **`fn::contest_venue($key)`** and **`fn::contest_game($key)`**. The backend uses them (and all functions below) with a try-function-first, fallback-to-existing-queries pattern.

### Implemented and wired

- **`fn::contest_venue($key)`** — `client_analytics/repository.rs` `get_venue_for_contest`
- **`fn::contest_game($key)`** — `client_analytics/repository.rs` `get_game_for_contest`
- **`fn::player_contest_ids($player_key)`** — `client_analytics/repository.rs` `contest_ids_for_player`
- **`fn::contest_participants($key)`** — `client_analytics/repository.rs` `get_contest_participants` and `analytics/repository.rs` `get_contest_participants`
- **`fn::player_contests_with_edges($player_key, $start_date, $end_date)`** — `client_analytics/repository.rs` `get_filtered_contests` (pass `""` for no date filter)
- **`fn::venue_with_contest_ids($key)`** — `analytics/repository.rs` `get_venue_contests`
- **`fn::game_with_contest_ids($key)`** — `analytics/repository.rs` `get_game_plays`
- **`fn::player_game_performance_data($player_key)`** — `analytics/repository.rs` `get_my_game_performance` (returns `{ resulted_in, contest_starts, played_with }`; backend aggregates)
- **`fn::daily_active_players_data($days)`** — `analytics/repository.rs` `get_daily_active_players` (returns `{ contest_days, resulted_in }`; backend groups by day and counts distinct players)
- **`fn::player_stats_by_id_str($id_str)`** — `analytics/repository.rs` `get_player_stats_for_me_by_email` and `get_player_stats_by_id_str` (returns dual out/in aggregates; backend picks non-zero set)
- **`fn::player_stats_by_key($key)`** — `analytics/repository.rs` `get_player_stats` (returns dual out/in aggregates by `type::record('player', $key)`; backend confirms player exists and picks non-zero set)
- **`fn::contest_row($key)`** — `contest/repository.rs` `find_by_id` and `find_details_by_id_impl` (single contest row by key; used when `contest_with_edges` is not defined or for simple lookup)
- **`fn::player_game_performance_detail_data($player_key)`** — `analytics/repository.rs` `get_player_game_performance_detail` (Game Performance tab: best/toughest opponent, best venue per game; returns raw arrays in one round-trip; backend aggregates and resolves names)

## Apply the functions

**The backend always tries these functions first and falls back to multi-query only when they are not defined.** To get one-round-trip behavior and use them, apply the script once per namespace/database:

```bash
./scripts/apply-surreal-functions.sh
```

(Or run the contents of `tools/arango-to-surreal/surreal-functions.surql` in Surrealist with your NS/DB selected.) If you skip this step, the app still works but uses the fallback multi-query path for each of the flows above.

## Related docs

- **docs/SURREALDB_ID_CONVENTIONS.md** — Record IDs, `type::record`, and edge convention (`in` = contest, `out` = venue/game/player).
- **docs/SURREALDB_EDGES.md** — Edge tables and relation semantics.
- **docs/surreal-indexes-optional.surql** — Indexes used by these queries.
