-- One-time migration: convert edge table columns out/in from string to record id (Thing).
-- Use when your DB was imported from an older .surql that stored out/in as strings.
-- Run in Surrealist (Query tab) or: surreal sql --endpoint <url> --ns <ns> --db <db> -f docs/surreal-migrate-edge-strings-to-things.surql
-- See docs/SURREALDB_EDGES.md for the convention and how to verify.

-- 1. played_at (out = contest, in = venue)
UPDATE played_at SET
  out = type::thing('contest', string::replace(string::concat(out), 'contest:', '')),
  in  = type::thing('venue',  string::replace(string::concat(in),  'venue:',  ''));

-- 2. played_with (out = contest, in = game)
UPDATE played_with SET
  out = type::thing('contest', string::replace(string::concat(out), 'contest:', '')),
  in  = type::thing('game',    string::replace(string::concat(in),  'game:',    ''));

-- 3. resulted_in (out = contest, in = player)
UPDATE resulted_in SET
  out = type::thing('contest', string::replace(string::concat(out), 'contest:', '')),
  in  = type::thing('player',  string::replace(string::concat(in),  'player:',  ''));
