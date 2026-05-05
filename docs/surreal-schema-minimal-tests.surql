OPTION IMPORT;

-- Minimal schema for integration tests. No production data required.
-- Goal: allow integration tests to CREATE/UPDATE/SELECT core entities and contest edges on an empty NS/DB.
-- SurrealDB v3 note: DEFINE TABLE syntax is `DEFINE TABLE IF NOT EXISTS <name> SCHEMAFULL;`
-- Idempotent: safe to re-run (IF NOT EXISTS on table, fields, indexes).
-- Usage: source scripts/load-env.sh prod && ./scripts/apply-surreal-schema-minimal.sh

DEFINE TABLE IF NOT EXISTS player SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS firstname ON player TYPE option<string>;
DEFINE FIELD IF NOT EXISTS lastname ON player TYPE option<string>;
DEFINE FIELD IF NOT EXISTS handle ON player TYPE option<string>;
DEFINE FIELD IF NOT EXISTS email ON player TYPE option<string>;
DEFINE FIELD IF NOT EXISTS password ON player TYPE option<string>;
DEFINE FIELD IF NOT EXISTS createdAt ON player TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS created_at ON player TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS isAdmin ON player TYPE option<bool>;
DEFINE FIELD IF NOT EXISTS is_admin ON player TYPE option<bool>;
DEFINE FIELD IF NOT EXISTS accessToken ON player TYPE option<string>;
DEFINE INDEX IF NOT EXISTS player_email ON TABLE player COLUMNS email;

DEFINE TABLE IF NOT EXISTS game SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS name ON game TYPE option<string>;
DEFINE FIELD IF NOT EXISTS year_published ON game TYPE option<int>;
DEFINE FIELD IF NOT EXISTS bgg_id ON game TYPE option<int>;
DEFINE FIELD IF NOT EXISTS description ON game TYPE option<string>;
DEFINE FIELD IF NOT EXISTS source ON game TYPE option<string>;
DEFINE FIELD IF NOT EXISTS createdAt ON game TYPE option<datetime>;

DEFINE TABLE IF NOT EXISTS bgg_catalog SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS bgg_id ON bgg_catalog TYPE int;
DEFINE FIELD IF NOT EXISTS name ON bgg_catalog TYPE string;
DEFINE FIELD IF NOT EXISTS year_published ON bgg_catalog TYPE option<int>;
DEFINE FIELD IF NOT EXISTS rank ON bgg_catalog TYPE option<int>;
DEFINE FIELD IF NOT EXISTS imported_at ON bgg_catalog TYPE datetime;
DEFINE FIELD IF NOT EXISTS import_batch ON bgg_catalog TYPE string;
DEFINE INDEX IF NOT EXISTS bgg_catalog_bgg_id ON TABLE bgg_catalog COLUMNS bgg_id UNIQUE;

DEFINE TABLE IF NOT EXISTS venue SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS display_name ON venue TYPE option<string>;
DEFINE FIELD IF NOT EXISTS displayName ON venue TYPE option<string>;
DEFINE FIELD IF NOT EXISTS formatted_address ON venue TYPE option<string>;
DEFINE FIELD IF NOT EXISTS formattedAddress ON venue TYPE option<string>;
DEFINE FIELD IF NOT EXISTS place_id ON venue TYPE option<string>;
DEFINE FIELD IF NOT EXISTS lat ON venue TYPE option<float>;
DEFINE FIELD IF NOT EXISTS lng ON venue TYPE option<float>;
DEFINE FIELD IF NOT EXISTS timezone ON venue TYPE option<string>;
DEFINE FIELD IF NOT EXISTS source ON venue TYPE option<string>;

DEFINE TABLE IF NOT EXISTS contest SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS name ON contest TYPE option<string>;
DEFINE FIELD IF NOT EXISTS start ON contest TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS stop ON contest TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS creator_id ON contest TYPE option<record<player>>;
DEFINE FIELD IF NOT EXISTS created_at ON contest TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS moderation_status ON contest TYPE option<string>;
DEFINE FIELD IF NOT EXISTS moderated_at ON contest TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS moderated_by ON contest TYPE option<record<player>>;
DEFINE FIELD IF NOT EXISTS moderation_note ON contest TYPE option<string>;
DEFINE FIELD IF NOT EXISTS startoffset ON contest TYPE option<string>;
DEFINE FIELD IF NOT EXISTS startOffset ON contest TYPE option<string>;
DEFINE INDEX IF NOT EXISTS contest_start ON TABLE contest COLUMNS start;
DEFINE INDEX IF NOT EXISTS contest_stop ON TABLE contest COLUMNS stop;

DEFINE TABLE IF NOT EXISTS played_at SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS `in` ON played_at TYPE record<contest>;
DEFINE FIELD IF NOT EXISTS `out` ON played_at TYPE record<venue>;
DEFINE INDEX IF NOT EXISTS played_at_in ON TABLE played_at COLUMNS `in`;
DEFINE INDEX IF NOT EXISTS played_at_out ON TABLE played_at COLUMNS `out`;

DEFINE TABLE IF NOT EXISTS played_with SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS `in` ON played_with TYPE record<contest>;
DEFINE FIELD IF NOT EXISTS `out` ON played_with TYPE record<game>;
DEFINE INDEX IF NOT EXISTS played_with_in ON TABLE played_with COLUMNS `in`;
DEFINE INDEX IF NOT EXISTS played_with_out ON TABLE played_with COLUMNS `out`;

DEFINE TABLE IF NOT EXISTS resulted_in SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS `in` ON resulted_in TYPE record<contest>;
DEFINE FIELD IF NOT EXISTS `out` ON resulted_in TYPE record<player>;
DEFINE FIELD IF NOT EXISTS place ON resulted_in TYPE option<int>;
DEFINE FIELD IF NOT EXISTS result ON resulted_in TYPE option<string>;
DEFINE FIELD IF NOT EXISTS points ON resulted_in TYPE option<int>;
DEFINE FIELD IF NOT EXISTS score ON resulted_in TYPE option<string>;
DEFINE INDEX IF NOT EXISTS resulted_in_in ON TABLE resulted_in COLUMNS `in`;
DEFINE INDEX IF NOT EXISTS resulted_in_out ON TABLE resulted_in COLUMNS `out`;
