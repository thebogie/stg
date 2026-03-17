-- Minimal schema for integration tests (auth, etc.). No production data required.
-- Apply to empty NS/DB so that CREATE player works. SurrealDB v3: id TYPE string.
-- Usage: see scripts/apply-surreal-schema-minimal.sh or run via surrealdb import.

DEFINE TABLE player SCHEMAFULL;
DEFINE FIELD id ON player TYPE string;
DEFINE FIELD firstname ON player TYPE option<string>;
DEFINE FIELD lastname ON player TYPE option<string>;
DEFINE FIELD handle ON player TYPE option<string>;
DEFINE FIELD email ON player TYPE option<string>;
DEFINE FIELD password ON player TYPE option<string>;
DEFINE FIELD createdAt ON player TYPE option<datetime>;
DEFINE FIELD created_at ON player TYPE option<datetime>;
DEFINE FIELD isAdmin ON player TYPE option<bool>;
DEFINE FIELD is_admin ON player TYPE option<bool>;
DEFINE FIELD accessToken ON player TYPE option<string>;
DEFINE INDEX player_email ON player COLUMNS email;
