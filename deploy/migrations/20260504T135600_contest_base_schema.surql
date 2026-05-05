OPTION IMPORT;

-- Minimal contest schema fields required by backend writes.
-- (Contest table is SCHEMAFULL in most environments; missing fields cause 500s on CREATE.)
DEFINE FIELD IF NOT EXISTS name ON contest TYPE option<string>;
DEFINE FIELD IF NOT EXISTS start ON contest TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS stop ON contest TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS creator_id ON contest TYPE option<record<player>>;
DEFINE FIELD IF NOT EXISTS created_at ON contest TYPE option<datetime>;

