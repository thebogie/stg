OPTION IMPORT;

-- BGG reference catalog (boardgames_ranks.csv import). Idempotent: safe to re-run.
-- Search tier for /api/games/search after `game`, before live BGG API.

DEFINE TABLE IF NOT EXISTS bgg_catalog SCHEMAFULL;

DEFINE FIELD IF NOT EXISTS bgg_id ON bgg_catalog TYPE int;
DEFINE FIELD IF NOT EXISTS name ON bgg_catalog TYPE string;
DEFINE FIELD IF NOT EXISTS year_published ON bgg_catalog TYPE option<int>;
DEFINE FIELD IF NOT EXISTS rank ON bgg_catalog TYPE option<int>;
DEFINE FIELD IF NOT EXISTS imported_at ON bgg_catalog TYPE datetime;
DEFINE FIELD IF NOT EXISTS import_batch ON bgg_catalog TYPE string;

DEFINE INDEX IF NOT EXISTS bgg_catalog_bgg_id ON TABLE bgg_catalog COLUMNS bgg_id UNIQUE;
