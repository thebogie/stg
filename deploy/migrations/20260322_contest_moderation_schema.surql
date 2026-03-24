OPTION IMPORT;

-- Contest moderation columns for SCHEMAFULL `contest` (prod DBs created before these fields existed).
-- Must run before 20260323_contest_moderation_backfill.surql. Idempotent (IF NOT EXISTS).

DEFINE FIELD IF NOT EXISTS moderation_status ON contest TYPE option<string>;
DEFINE FIELD IF NOT EXISTS moderated_at ON contest TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS moderated_by ON contest TYPE option<record<player>>;
DEFINE FIELD IF NOT EXISTS moderation_note ON contest TYPE option<string>;
