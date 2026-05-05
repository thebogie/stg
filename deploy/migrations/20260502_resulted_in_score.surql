OPTION IMPORT;

-- Per-player free-form score on contest results (often numeric; may be empty).
DEFINE FIELD IF NOT EXISTS score ON resulted_in TYPE option<string>;
