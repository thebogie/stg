OPTION IMPORT;

-- One-time: existing contests were created before moderation_status existed.
-- Requires moderation fields on `contest` (see 20260322_contest_moderation_schema.surql or deploy/smacktalk.surql).

UPDATE contest SET moderation_status = 'approved' WHERE moderation_status = NONE;
