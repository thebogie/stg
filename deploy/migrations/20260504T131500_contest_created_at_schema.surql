OPTION IMPORT;

-- contest.created_at is written by the backend on contest creation.
-- Older DBs (or fresh DBs without base schema applied) can be SCHEMAFULL without this field.
DEFINE FIELD IF NOT EXISTS created_at ON contest TYPE option<datetime>;

