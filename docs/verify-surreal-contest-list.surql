-- Copy-paste this whole block into Surrealist and run. Use ns/db stg_rd (or your import ns/db).
-- Replace 2025041711441879938520500 with a real player key from your data if needed.

-- A. Contest id string form (should be contest:`key` not contest:`contest:key`)
SELECT string::concat(id) AS contest_id_str FROM contest LIMIT 1;

-- B. Edge out string form (should match contest_id_str)
SELECT string::concat(out) AS edge_out_str FROM resulted_in LIMIT 1;

-- C. How edge in is stored (raw + string)
SELECT in, string::concat(in) AS in_str FROM resulted_in LIMIT 3;

-- D. Subquery: contests for this player (if [] then in/type::thing don't match)
SELECT out AS contest_rid FROM resulted_in WHERE in = type::thing("player", "2025041711441879938520500");

-- E. Contest list: direct id IN subquery (Thing vs Thing; should work now that formats match)
SELECT * FROM contest WHERE id IN (SELECT out FROM resulted_in LIMIT 10);

-- F. Contest list for this player: direct id IN (preferred)
SELECT * FROM contest WHERE id IN (SELECT out FROM resulted_in WHERE in = type::thing("player", "2025041711441879938520500"));

-- E2/F2. If E/F still return [], use array form (IN with SELECT VALUE)
-- SELECT * FROM contest WHERE id IN (SELECT VALUE out FROM resulted_in LIMIT 10);
-- SELECT * FROM contest WHERE id IN (SELECT VALUE out FROM resulted_in WHERE in = type::thing("player", "2025041711441879938520500"));

-- G. Counts
SELECT count() AS players FROM player GROUP ALL;
SELECT count() AS contests FROM contest GROUP ALL;
SELECT count() AS resulted_in_edges FROM resulted_in GROUP ALL;
