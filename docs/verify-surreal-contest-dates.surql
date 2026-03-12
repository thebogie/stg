-- Verify contest dates: are start/stop/created_at stored correctly (datetime)?
-- Run: ./scripts/run-surreal-script.sh docs/verify-surreal-contest-dates.surql
-- Or paste each block into Surrealist (http://localhost:50001, ns/db: stg_rd).

-- 1. Raw contest rows and date fields
SELECT id, name, start, stop, created_at FROM contest ORDER BY start DESC LIMIT 5;

-- 2. Contests in the last 30 days (if this returns 0 but you have a recent contest, dates are wrong)
SELECT id, name, start FROM contest WHERE start >= time::now() - duration::from_days(30) ORDER BY start DESC;

-- 3. Optional: type check and "in last 30" flag per row
SELECT id, type::string(start) AS start_str, start >= time::now() - duration::from_days(30) AS in_last_30 FROM contest LIMIT 3;

-- 4. If leaderboard is empty: does this contest have any resulted_in (outcomes)? Replace KEY with contest key only (e.g. 10860534).
-- SELECT * FROM resulted_in WHERE `out` = type::record('contest', '10860534');
-- Leaderboard-style query (SurrealQL uses IN (SELECT VALUE id ...), no JOIN):
-- SELECT string::concat(`in`) AS player_id, count() AS total_plays FROM resulted_in WHERE `out` IN (SELECT VALUE id FROM contest WHERE start >= time::now() - duration::from_days(30)) GROUP BY player_id;
