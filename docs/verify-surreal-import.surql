-- Run these in Surrealist (or: surreal sql --endpoint http://localhost:50001 --namespace stg_rd --database stg_rd --username root --password root)
-- Use namespace and database that match your import (e.g. stg_rd / stg_rd).

-- ========== 1. Document tables: should have rows ==========
SELECT count() AS n FROM player GROUP ALL;
SELECT count() AS n FROM game GROUP ALL;
SELECT count() AS n FROM venue GROUP ALL;
SELECT count() AS n FROM contest GROUP ALL;

-- ========== 2. Edge tables: should have rows; out/in in Surreal form (table:key) ==========
SELECT count() AS n FROM played_at GROUP ALL;
SELECT count() AS n FROM played_with GROUP ALL;
SELECT count() AS n FROM resulted_in GROUP ALL;

-- Sample edges: out should be contest id, in should be venue/game/player
SELECT * FROM played_at LIMIT 2;
SELECT * FROM played_with LIMIT 2;
SELECT * FROM resulted_in LIMIT 3;

-- ========== 3. Edge direction: eyeball that out = contest id, in = venue/game/player ==========
-- resulted_in: out should look like contest:xxx, in like player:xxx
-- played_at:   out = contest:xxx, in = venue:xxx
-- played_with: out = contest:xxx, in = game:xxx
SELECT out, in, place, result FROM resulted_in LIMIT 5;

-- ========== 4. One contest’s venue, games, outcomes (run after you have data) ==========
-- Pick one contest id from the previous query, then run (replace CONTEST_KEY with the key part only, e.g. 2025041711441879994340500):
-- SELECT * FROM played_at WHERE out = type::record('contest', 'CONTEST_KEY');
-- SELECT * FROM played_with WHERE out = type::record('contest', 'CONTEST_KEY');
-- SELECT `in` AS player_id, place, result FROM resulted_in WHERE out = type::record('contest', 'CONTEST_KEY') ORDER BY place ASC;

-- Simpler: get one contest id, then run the next 3 with that id (e.g. contest:abc123 → use key "abc123"):
-- SELECT id FROM contest LIMIT 1;
-- SELECT `in` AS venue_id FROM played_at WHERE out = type::record('contest', 'PASTE_KEY_HERE') LIMIT 1;
-- SELECT `in` AS game_id FROM played_with WHERE out = type::record('contest', 'PASTE_KEY_HERE');
-- SELECT `in` AS player_id, place, result FROM resulted_in WHERE out = type::record('contest', 'PASTE_KEY_HERE') ORDER BY place ASC;

-- ========== 5. One player’s contests ==========
-- First: SELECT id FROM player LIMIT 1;  then use the key in the next line:
-- SELECT `out` AS contest_id FROM resulted_in WHERE `in` = type::record('player', 'PASTE_PLAYER_KEY_HERE') LIMIT 10;

-- ========== 6. Optional: rating tables ==========
SELECT count() AS n FROM rating_latest GROUP ALL;
SELECT count() AS n FROM rating_history GROUP ALL;
