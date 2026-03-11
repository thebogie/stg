-- Optional indexes for faster search and order (for DBs created before the conversion tool emitted them).
-- New ArangoDB→SurrealDB conversions already get these via tools/arango-to-surreal --schema.
-- Run in Surrealist (or CLI) after selecting your namespace and database.
-- Requires SurrealDB 1.3+ for IF NOT EXISTS; omit it on older versions.

-- Player: lookup by email (login, "me" resolution), search by handle/email
DEFINE INDEX IF NOT EXISTS player_email ON TABLE player COLUMNS email;

-- Edge tables: our queries filter by `in` (player/venue/game) and `out` (contest)
-- resulted_in: WHERE `in` = type::thing('player', $key) and WHERE `out` = type::thing('contest', $key)
DEFINE INDEX IF NOT EXISTS resulted_in_in ON TABLE resulted_in COLUMNS `in`;
DEFINE INDEX IF NOT EXISTS resulted_in_out ON TABLE resulted_in COLUMNS `out`;

-- played_at: WHERE `out` = contest, INSIDE $contest_ids for venue lists
DEFINE INDEX IF NOT EXISTS played_at_out ON TABLE played_at COLUMNS `out`;
DEFINE INDEX IF NOT EXISTS played_at_in ON TABLE played_at COLUMNS `in`;

-- played_with: WHERE `out` = contest, INSIDE $contest_ids for game lists
DEFINE INDEX IF NOT EXISTS played_with_out ON TABLE played_with COLUMNS `out`;
DEFINE INDEX IF NOT EXISTS played_with_in ON TABLE played_with COLUMNS `in`;

-- rating_history: lookups by player_id + scope; ordering by period_end
DEFINE INDEX IF NOT EXISTS rating_history_player_scope ON TABLE rating_history COLUMNS player_id, scope_type;
DEFINE INDEX IF NOT EXISTS rating_history_period ON TABLE rating_history COLUMNS period_end;

-- Verify with EXPLAIN (example: check resulted_in uses index)
-- EXPLAIN SELECT * FROM resulted_in WHERE `in` = type::thing('player', 'YOUR_KEY');
-- Look for "Iterate Index" in the plan.
