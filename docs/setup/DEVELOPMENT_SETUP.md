# Development setup

One-time environment setup and troubleshooting. **Day-to-day commands** (including prod snapshot import): **[DAILY_WORKFLOW.md](../DAILY_WORKFLOW.md)**.

For repo layout, see [PROJECT_STRUCTURE.md](../PROJECT_STRUCTURE.md).

## Prerequisites

- Docker and Docker Compose
- Rust toolchain (for local backend/frontend builds)
- (Optional) VS Code with Rust Analyzer, CodeLLDB, Docker extension

## Environment files

Create env files from templates:

```bash
./config/setup-env.sh dev    # creates config/.env.dev
./config/setup-env.sh prod   # creates config/.env.prod
```

Edit `config/.env.dev`. Key variables:

| Variable | Default / notes |
|----------|-----------------|
| `VOLUME_PATH` | `data/dev` — Docker bind mounts (Surreal, Redis, backend files) |
| `BACKEND_PORT` | `50002` |
| `FRONTEND_PORT` | `50003` |
| `SURREALDB_PORT` | `50001` |
| `REDIS_PORT` | `6379` |
| `SURREAL_USER`, `SURREAL_PASSWORD` | SurrealDB credentials |
| `SURREAL_SEED_DIR` | Optional — directory with `*.surql.gz` prod exports (see [DAILY_WORKFLOW.md](../DAILY_WORKFLOW.md)) |

Scripts load env via `scripts/load-env.sh` (default: dev). Override: `./scripts/start-back.sh prod` or `ENV=prod`.

## Tests

```bash
./ci-local.sh all              # build, unit, integration, e2e
./ci-local.sh unit             # unit only
```

See [testing/HOW_TO_RUN_TESTS.md](../testing/HOW_TO_RUN_TESTS.md).

## Relevant paths

- **config/** — `.env.dev`, `.env.prod`, templates, `setup-env.sh`
- **deploy/** — `docker-compose.yml` (SurrealDB, Redis, backend)
- **scripts/** — `start-back.sh`, `start-deps.sh`, `start-front.sh`, `stop-back.sh`, `dev-debug.sh`, `load-env.sh`
- **data/dev/** — gitignored local DB/Redis mounts (default `VOLUME_PATH`)

## Troubleshooting

- **Port in use:** Change ports in `config/.env.dev` or stop the conflicting process.
- **Backend can't reach SurrealDB:** Stack up via `./scripts/start-back.sh` or `./scripts/start-deps.sh`. Docker backend uses hostname `surrealdb`; host backend uses `127.0.0.1` and `SURREALDB_PORT`.
- **Frontend can't reach backend:** Backend running; `BACKEND_URL` in `.env.dev` matches (e.g. `http://localhost:50002`).
- **Clean slate:** `./scripts/stop-back.sh`, then remove `data/dev/surrealdb_data` (and redis/backend subdirs if needed). Or prod re-seed: `SURREAL_SEED_FORCE=1 ./scripts/start-deps.sh` (see [DAILY_WORKFLOW.md](../DAILY_WORKFLOW.md)).
- **Missing Surreal functions:** `./scripts/apply-surreal-functions.sh` (hybrid `start-deps.sh` applies them automatically after seed).

## See also

- [DAILY_WORKFLOW.md](../DAILY_WORKFLOW.md) — prod snapshot, daily start commands, deploy
- [WORKFLOW.txt](../WORKFLOW.txt) — CI and script entrypoints
- [setup/SURREALDB_UI.md](SURREALDB_UI.md) — Surrealist / DB UI
