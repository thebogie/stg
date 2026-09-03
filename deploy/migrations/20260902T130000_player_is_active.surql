OPTION IMPORT;

-- Player account active flag (admin deactivate/reactivate). Prod snapshots predate this field.
DEFINE FIELD IF NOT EXISTS isActive ON player TYPE option<bool> DEFAULT true PERMISSIONS FULL;
