-- One-time migration: convert edge table columns out/in from string to record id (Thing).
-- Use when your DB was imported from an older .surql that stored out/in as strings.
-- Run in Surrealist (Query tab) or: surreal sql --endpoint <url> --namespace <ns> --database <db> --username root --password <pass> -f docs/surreal-migrate-edge-strings-to-things.surql
-- See docs/SURREALDB_EDGES.md for the convention and how to verify.

-- 1. played_at (out = contest, in = venue)
UPDATE played_at SET
  out = type::record('contest', string::replace(string::concat(out), 'contest:', '')),
  in  = type::record('venue',  string::replace(string::concat(in),  'venue:',  ''));

-- 2. played_with (out = contest, in = game)
UPDATE played_with SET
  out = type::record('contest', string::replace(string::concat(out), 'contest:', '')),
  in  = type::record('game',    string::replace(string::concat(in),  'game:',    ''));

-- 3. resulted_in (out = contest, in = player)
UPDATE resulted_in SET
  out = type::record('contest', string::replace(string::concat(out), 'contest:', '')),
  in  = type::record('player',  string::replace(string::concat(in),  'player:',  ''));
