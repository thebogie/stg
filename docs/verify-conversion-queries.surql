-- =============================================================================
-- ArangoDB → SurrealDB conversion check
-- Run in Surrealist (http://localhost:50001, ns/db e.g. stg_rd) or:
--   ./scripts/run-surreal-script.sh docs/verify-conversion-queries.surql
-- Run one block at a time and share the results if something looks wrong.
-- =============================================================================

-- ---------- 1. Row counts (all document + edge tables) ----------
SELECT count() AS n FROM player GROUP ALL;
SELECT count() AS n FROM game GROUP ALL;
SELECT count() AS n FROM venue GROUP ALL;
SELECT count() AS n FROM contest GROUP ALL;
SELECT count() AS n FROM played_at GROUP ALL;
SELECT count() AS n FROM played_with GROUP ALL;
SELECT count() AS n FROM resulted_in GROUP ALL;

-- ---------- 2. Contest dates: must be datetime so time filters work ----------
SELECT id, name, start, stop FROM contest ORDER BY start DESC LIMIT 3;
SELECT id, name, start FROM contest WHERE start >= time::now() - duration::from_days(30) ORDER BY start DESC;

-- ---------- 3. Edges: out/in must be record ids (not plain strings) ----------
SELECT string::concat(`out`) AS out_str, string::concat(`in`) AS in_str FROM played_at LIMIT 2;
SELECT string::concat(`out`) AS out_str, string::concat(`in`) AS in_str FROM played_with LIMIT 2;
SELECT string::concat(`out`) AS out_str, string::concat(`in`) AS in_str, place FROM resulted_in LIMIT 2;

-- ---------- 4. resulted_in.out must match contest.id (leaderboard / time window) ----------
-- Count how many resulted_in rows are in the last 30 days (if > 0, leaderboard query can return data)
SELECT count() AS resulted_in_last_30
FROM resulted_in
WHERE `out` IN (SELECT VALUE id FROM contest WHERE start >= time::now() - duration::from_days(30))
GROUP ALL;

-- ---------- 5. Contest list via edges (backend pattern) ----------
SELECT VALUE `out` FROM resulted_in LIMIT 5;
SELECT * FROM contest WHERE id IN (SELECT VALUE `out` FROM resulted_in LIMIT 10);

-- ---------- 6. One contest’s graph (pick a contest key from query 2 or 5) ----------
-- Replace CONTEST_KEY with the key only (e.g. 10860534), then run:
-- SELECT * FROM played_at WHERE `out` = type::thing('contest', 'CONTEST_KEY');
-- SELECT * FROM played_with WHERE `out` = type::thing('contest', 'CONTEST_KEY');
-- SELECT `in` AS player_id, place, result FROM resulted_in WHERE `out` = type::thing('contest', 'CONTEST_KEY') ORDER BY place;

-- ---------- 7. Achievements: player must have resulted_in with record id `in` ----------
-- If edges were imported as strings, run docs/surreal-migrate-edge-strings-to-things.surql first.
-- Sample player key from your data (use this key in the two queries below if the hardcoded one fails):
SELECT string::replace(string::concat(`in`), 'player:', '') AS player_key FROM resulted_in LIMIT 1;
SELECT count() AS total FROM resulted_in WHERE `in` = type::thing('player', '2025041711441879938520500') GROUP ALL;
SELECT count() AS wins FROM resulted_in WHERE `in` = type::thing('player', '2025041711441879938520500') AND place = 1 GROUP ALL;

-- ---------- 8. Profile overall stats by email (backend uses this pattern for "me") ----------
-- Replace YOUR_EMAIL with your player email (e.g. thebogie@live.com). Run in same ns/db as backend (see env SURREAL_*).
-- Step A: confirm player exists and get their id
SELECT id, handle, email FROM player WHERE string::lowercase(email) = string::lowercase('YOUR_EMAIL') LIMIT 1;
-- Step B: full stats (extract .count and .`math::mean`/.\`math::min\` from aggregate subquery results)
SELECT
    id AS player_id,
    handle AS player_handle,
    ((SELECT count() FROM resulted_in WHERE `in` = (SELECT VALUE id FROM player WHERE string::lowercase(email) = string::lowercase('YOUR_EMAIL') LIMIT 1)[0] GROUP ALL)[0].count) ?? 0 AS total_contests,
    ((SELECT count() FROM resulted_in WHERE `in` = (SELECT VALUE id FROM player WHERE string::lowercase(email) = string::lowercase('YOUR_EMAIL') LIMIT 1)[0] AND place = 1 GROUP ALL)[0].count) ?? 0 AS total_wins,
    ((SELECT math::mean(place) FROM resulted_in WHERE `in` = (SELECT VALUE id FROM player WHERE string::lowercase(email) = string::lowercase('YOUR_EMAIL') LIMIT 1)[0] GROUP ALL)[0].`math::mean`) ?? 0 AS average_placement,
    ((SELECT math::min(place) FROM resulted_in WHERE `in` = (SELECT VALUE id FROM player WHERE string::lowercase(email) = string::lowercase('YOUR_EMAIL') LIMIT 1)[0] GROUP ALL)[0].`math::min`) ?? 0 AS best_placement
FROM (SELECT * FROM player WHERE string::lowercase(email) = string::lowercase('YOUR_EMAIL') LIMIT 1);
