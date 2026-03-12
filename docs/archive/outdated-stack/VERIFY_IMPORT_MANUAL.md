# Verify Arango→Surreal import (run by hand)

**Goal:** Confirm the data came in correctly so we only do the migration once.

**Where to run:** Surrealist (or any SurrealDB client) → connect to `http://localhost:50001`, namespace `stg_rd`, database `stg_rd`, user `root`, password `root`.

**Quick conversion check:** Run the queries in **docs/verify-conversion-queries.surql** (or `./scripts/run-surreal-script.sh docs/verify-conversion-queries.surql`). They cover row counts, contest dates, edge out/in as record ids, leaderboard-style time-window query, and contest-by-edge lookups. Share the outputs if something fails or looks wrong.

---

## 1. Copy-paste this block and run it

Paste the whole thing into the query box and run. (Use a real player key if you know one; otherwise keep the one below.) If your client runs one statement at a time, run each line in order.

```sql
SELECT string::concat(id) AS contest_id_str FROM contest LIMIT 1;
SELECT string::concat(out) AS edge_out_str FROM resulted_in LIMIT 1;
SELECT in, string::concat(in) AS in_str FROM resulted_in LIMIT 3;
SELECT out AS contest_rid FROM resulted_in WHERE in = type::record("player", "2025041711441879938520500");
SELECT * FROM contest WHERE id IN (SELECT VALUE out FROM resulted_in LIMIT 10);
SELECT * FROM contest WHERE id IN (SELECT VALUE out FROM resulted_in WHERE in = type::record("player", "2025041711441879938520500"));
SELECT count() AS players FROM player GROUP ALL;
SELECT count() AS contests FROM contest GROUP ALL;
SELECT count() AS resulted_in_edges FROM resulted_in GROUP ALL;
```

---

## 2. Paste the output back

Copy **all** the result sets (or a screenshot) and paste it here. From that we’ll confirm:

| Check | We want to see |
|-------|----------------|
| Contest id | `contest_id_str` like `contest:\`20250417...\`` (key only, **not** `contest:\`contest:2025...\``) |
| Edge out | `edge_out_str` same style as contest_id_str |
| Edge in | `in` as record id, `in_str` like `player:\`20250417...\`` |
| Player’s contests (query 4) | At least one row (contest_rid) |
| Contest list (query 5) | At least one contest row |
| Contest list for player (query 6) | At least one contest row |
| Counts (7–9) | Non-zero players, contests, resulted_in_edges |

If anything doesn’t match, we’ll fix the converter or re-import and you run this again until it’s right.

**If queries 5 and 6 return []:** Some SurrealDB setups don’t match `id IN (SELECT VALUE out FROM ...)`. The backend now uses a two-query approach: (1) `SELECT out FROM resulted_in WHERE in = type::record('player', $player_key)` to get contest ids, (2) `SELECT * FROM contest WHERE id INSIDE $contest_ids`, so the app still gets the contest list. To confirm the first step works in Surrealist, run only query 4 above; if it returns rows, the backend will work.

---

## 3. Diagnostic queries (when app shows 0 contests/games/venues)

Run these in Surrealist and paste the **raw results** (or export JSON). They show how data is stored so we can match backend expectations.

**A. Table row shapes (id format and key fields)**

```sql
-- Contest: one row, see id type (string vs object) and name/start/stop
SELECT * FROM contest LIMIT 1;

-- Game: one row, see id and name
SELECT * FROM game LIMIT 1;

-- Venue: one row, see id and displayName
SELECT * FROM venue LIMIT 1;
```

**B. Edge tables (what the contest search subqueries use)**

```sql
-- played_at: out = contest, in = venue. Backend uses: id IN (SELECT VALUE out FROM played_at)
SELECT out, in FROM played_at LIMIT 3;

-- played_with: out = contest, in = game. Backend uses: id IN (SELECT VALUE out FROM played_with)
SELECT out, in FROM played_with LIMIT 3;

-- Result of the two subqueries (contest ids that have played_at and played_with)
SELECT VALUE out FROM played_at LIMIT 5;
SELECT VALUE out FROM played_with LIMIT 5;
```

**C. Does any contest appear in both edge sets?**

```sql
-- Contests that have at least one played_at and one played_with (what scope=all uses)
SELECT * FROM contest
WHERE id IN (SELECT VALUE out FROM played_at)
  AND id IN (SELECT VALUE out FROM played_with)
ORDER BY start DESC
LIMIT 5;
```

**D. Backend-style search (games and venues)**

```sql
-- Game search by name (backend uses string::contains on name)
SELECT id, name FROM game WHERE string::contains(string::lowercase(name), string::lowercase("")) LIMIT 5;

-- Venue search by displayName (backend uses string::contains on displayName)
SELECT id, displayName FROM venue WHERE string::contains(string::lowercase(displayName), string::lowercase("")) LIMIT 5;
```

If the game or venue query above returns 0 rows, run these to confirm data exists: `SELECT * FROM game LIMIT 3;` and `SELECT * FROM venue LIMIT 3;`.

**What to paste back**

- For **A**: the single row for contest, game, and venue (so we see exact `id` shape: e.g. `"contest:key"` vs `{ "tb": "contest", "id": "key" }`).
- For **B**: the 3 edge rows and the two `SELECT VALUE out` result sets.
- For **C**: whether you get 0 or more contest rows.
- For **D**: whether you get any game/venue rows when the search term is empty (empty string matches everything in `string::contains`).

---

## 4. Check contest dates and times

Use this to confirm that `contest.start`, `contest.stop`, and `contest.created_at` are stored correctly (as **datetime** type). If they were imported as strings, time-window queries (e.g. leaderboard “Last 30 days”) can return no rows.

**Where to run:** Same as above — Surrealist at `http://localhost:50001`, namespace/database `stg_rd`, or save the queries to a `.surql` file and run with `./scripts/run-surreal-script.sh docs/verify-surreal-contest-dates.surql` (create that file or paste into Surrealist).

**Step 1 — See raw contest rows and date fields**

```sql
SELECT id, name, start, stop, created_at FROM contest ORDER BY start DESC LIMIT 5;
```

- **What you want:** `start`, `stop`, and `created_at` show as ISO8601-style values (e.g. `2026-03-07T23:00:22Z`). In Surrealist they may render as strings; what matters is that the next step works.
- **If you see** empty or odd-looking values, or a type that’s clearly a string in the UI, the converter or backend may have written strings instead of datetime.

**Step 2 — Confirm datetime comparison works (e.g. “last 30 days”)**

```sql
SELECT id, name, start FROM contest WHERE start >= time::now() - duration::from_days(30) ORDER BY start DESC;
```

- **What you want:** You get at least one row for any contest that actually started in the last 30 days. If you know you have such a contest (e.g. created 2026-03-07) but this returns **no rows**, then `start` is likely stored as a string and comparisons with `time::now()` are wrong — re-import with the arango-to-surreal converter (which normalizes dates to `type::datetime(...)`) or fix the backend to write datetime.
- **Optional check:** Run the same query with `duration::from_days(365)`; you should see more (or the same) rows.

**Step 3 — Optional: explicit type of `start`**

Some SurrealDB clients or versions let you inspect value types. You can try:

```sql
SELECT id, type::string(start) AS start_str, start >= time::now() - duration::from_days(30) AS in_last_30 FROM contest LIMIT 3;
```

- If `in_last_30` is `false` for a contest you know is recent, the stored type or value is wrong.

**Summary**

| If you see… | Then… |
|-------------|--------|
| Step 1: `start`/`stop`/`created_at` as sensible ISO-like values; Step 2 returns recent contests | Dates are stored correctly; leaderboard time windows should work. |
| Step 2 returns 0 rows but you have a recent contest | Dates are likely strings or wrong; re-run the converter with date normalization or fix the source that writes contest rows. |
