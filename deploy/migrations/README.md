# SurrealDB Migrations

Put versioned `.surql` migration files in this directory.

- Applied in lexical order by `deploy/run_surreal_migrations.sh`.
- Files should be idempotent (safe to re-run).
- Recommended naming: `YYYYMMDDTHHMMSS_description.surql`

Example:

`20260319T190000_n_plus_1.surql`

`20260321T210000_bgg_catalog.surql` — defines `bgg_catalog` for BGG ranks CSV import (`import_bgg_catalog` binary).

