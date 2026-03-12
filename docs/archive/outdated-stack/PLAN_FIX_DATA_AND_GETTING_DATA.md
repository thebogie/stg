# Plan: Fix data and “getting data” after SurrealDB migration

Login works; contest list, contest detail, player results, and analytics are empty or broken. This plan gets data and reads aligned.

---

## 1. Re-import data with the fixed converter (required)

**Cause:** Edges were likely imported with the old converter (out/in reversed). The backend now expects:

- **played_at:** `out` = contest, `in` = venue  
- **played_with:** `out` = contest, `in` = game  
- **resulted_in:** `out` = contest, `in` = player  

**Steps:**

1. From the repo root, regenerate the SurrealDB import file using the **current** arango-to-surreal (already fixed: out=_from, in=_to):
   ```bash
   cargo run -p arango-to-surreal -- path/to/smacktalk.zip -o config/smacktalk.surql
   ```
2. (Optional) In SurrealDB, clear existing data if you want a clean slate:
   - Remove or truncate tables: `contest`, `player`, `game`, `venue`, `played_at`, `played_with`, `resulted_in` (and any rating tables if you re-import those).
3. Import the new file (adjust URL/ns/db/user/pass to your env):
   ```bash
   surreal import --conn ws://localhost:8000 --ns <ns> --db <db> --user root --pass <pass> config/smacktalk.surql
   ```

**Check:** After import, in SurrealDB run:

- `SELECT * FROM contest LIMIT 1;`
- `SELECT * FROM resulted_in LIMIT 3;`  
  Confirm `out` looks like contest ids and `in` like player ids for resulted_in.

---

## 2. Pinpoint what still fails (after re-import)

If something still “doesn’t work”, map it to the right layer:

| What’s broken | Where to look | What to check |
|---------------|----------------|----------------|
| Contest list (search) empty | `contest/controller.rs` → `search_contests_handler_impl`; `contest/repository.rs` → `search_contests` | Scope: for “mine”, auth must provide email → player lookup → `player_id`. Logs: “Search query returned 0 items”. |
| Contest detail (single contest) empty / 404 | `contest/repository.rs` → `find_details_by_id` | Contest exists in `contest` table; `played_at`/`played_with`/`resulted_in` have `out` = that contest’s id (e.g. `contest:key`). |
| Player’s contests / analytics empty | `client_analytics/repository.rs`; `analytics/repository.rs` | Same edge direction: `resulted_in.out` = contest, `resulted_in.in` = player. Player id format: API uses `player/123`, DB uses `player:123`; code normalizes with `to_rid` / `split_rid_owned`. |
| Games or venues lists empty | `game/repository.rs`, `venue/repository.rs` | Document tables populated by same import; no edges required for “list all”. |

**Quick checks:**

- Backend logs: look for “Found 0 contests”, “No contest found with ID”, or SurrealDB errors.
- API: call search with `scope=all` first; if that returns data but “mine” doesn’t, the issue is auth/player_id for scope.
- SurrealDB: run the same SELECTs used in the code (see `SURREALDB_EDGES.md`) and confirm rows and `out`/`in` values.

---

## 3. Fix remaining backend issues (only if needed)

- **Record ID format:** SurrealDB uses `table:key`; API/slash form is `table/key`. The code normalizes in several places (`contest_rid`, `to_rid`, `split_rid_owned`, etc.). If a specific endpoint fails, check that it normalizes the id the same way (e.g. `type::record('contest', key)` vs `type::record(contest_rid)` in SurrealDB v3).
- **Auth → player_id for “mine”:** If “mine” is always empty but “all” works, ensure the auth middleware sets the user (e.g. email) in `req.extensions()` and that `player_repo.find_by_email(email)` returns the correct player with `id` in slash form (`player/...`).
- **Empty subqueries:** If contest list is still empty with `scope=all`, verify in DB that `played_at` and `played_with` have rows with `out` = contest ids. Search explicitly requires “contest has at least one played_at and one played_with” (see `where_parts` in `search_contests`).

---

## 4. Optional: one-off migration instead of re-import

If you cannot re-run the Arango export but already have data in SurrealDB with edges in the **wrong** direction, you can fix edges in place with SurrealQL (example for resulted_in):

```sql
-- Only if your current data has out=player, in=contest. Adjust table names if different.
UPDATE resulted_in SET out = in, in = out WHERE true;
```

Prefer re-import so the source of truth (arango-to-surreal + .surql) matches the convention; use migration only when re-import isn’t an option.

---

## Summary

1. **Re-import** with the current arango-to-surreal so edges use out=_from, in=_to.  
2. **Reproduce** failures (contest list, detail, player results, analytics) and match them to the table above.  
3. **Fix** any remaining ID or auth/scope issues after data is correct.

After re-import, contest list (with `scope=all` or `scope=mine` when player is set), contest detail, and player-scoped analytics should return data if the corresponding tables and edges are populated.
