# SurrealDB: Running and UI

SurrealDB runs as an independent service in Docker Compose so you can use a UI to run queries, inspect data, and reset/import data without touching the app.

## Starting SurrealDB

**Dependencies only (hybrid dev):**
```bash
cd deploy
docker compose -f docker-compose.deps.yml up -d surrealdb
```

**Production stack:** SurrealDB is defined in `docker-compose.production.yml`. Start it with the rest of the stack or on its own:
```bash
docker compose -f deploy/docker-compose.production.yml --env-file config/.env.production up -d surrealdb
```

- **Port**: `50001` by default (same as ArangoDB; override with `SURREALDB_PORT`).
- **Data**: Persisted in a volume (`docker-data/surrealdb_data` for deps, or `VOLUME_PATH/surrealdb_data` in production).
- **Auth (deps)**: user `root`, password `root`. For production set `SURREAL_USER` and `SURREAL_PASSWORD` in your env file.

## UI: Surrealist

**Surrealist** is the official SurrealDB GUI. Use it to:

- Run SurrealQL queries (with syntax highlighting, Ctrl/Cmd+Enter to run)
- Browse and edit records
- Manage schema, users, and (in 2.0) live queries and API docs
- Reset data (e.g. delete tables, re-import)

**Ways to use Surrealist:**

1. **Web**: https://app.surrealdb.com — connect to your local or remote SurrealDB (e.g. `http://localhost:50001` for deps).
2. **Desktop**: Download from [SurrealDB](https://surrealdb.com/docs/surrealist) for a standalone app.

**Connect from Surrealist:**

- **Endpoint**: `http://localhost:50001` (or your host/port if remote).
- **Namespace**: e.g. `stg_rd` or `smacktalk` (must match what you use in the app and for `surreal import`).
- **Database**: same as namespace if you use one DB, or your DB name.
- **User / Password**: `root` / `root` for deps; in production use the same credentials as in your env (`SURREAL_USER` / `SURREAL_PASSWORD`).

After connecting, you can run SurrealQL (e.g. `SELECT * FROM player;`), clear tables (`REMOVE TABLE player;`), or re-import the converted `.surql` file via the SurrealDB CLI (see `tools/arango-to-surreal/README.md`).

## Importing converted data

With SurrealDB and Surrealist running:

```bash
# Generate .surql from Arango backup
cargo run -p arango-to-surreal -- ~/work/_backups/smacktalk.zip -o _build/smacktalk.surql

# Import (deps: localhost:50001, user root, pass root)
surreal import --conn http://localhost:50001 --user root --pass root --ns stg_rd --db stg_rd _build/smacktalk.surql
```

Use the same `--ns` and `--db` in Surrealist when connecting so the UI shows the imported data.
