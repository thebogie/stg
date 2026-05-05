# Development setup

How to set up your environment and run the app locally. For repo layout, see [PROJECT_STRUCTURE.md](../PROJECT_STRUCTURE.md).

## Prerequisites

- Docker and Docker Compose
- Rust toolchain (for local backend/frontend builds)
- (Optional) VSCode with Rust Analyzer, CodeLLDB, Docker extension

## 1. Environment files

Create env files from templates:

```bash
./config/setup-env.sh dev    # creates config/.env.dev
./config/setup-env.sh prod   # creates config/.env.prod
```

Edit `config/.env.dev` (and `.env.prod` if you use it). Key variables:

- `VOLUME_PATH` – Where Docker bind-mounts store data (default: `data/dev` or `data/prod` under the repo; see `scripts/load-env.sh`)
- `BACKEND_PORT` – API port (default: `50002`)
- `FRONTEND_PORT` – Frontend port (default: `50003`)
- `SURREALDB_PORT` – SurrealDB port (default: `50001`)
- `REDIS_PORT` – Redis port (default: `6379`)
- `SURREAL_USER`, `SURREAL_PASSWORD` – SurrealDB credentials

Scripts load env via `scripts/load-env.sh` (default: dev). Override with an argument or `ENV=prod`.

## 2. Run the stack

### Full stack (backend in Docker)

**Terminal 1 – backend (SurrealDB + Redis + API):**

```bash
./scripts/start-back.sh           # build and start
./scripts/start-back.sh --no-build   # start only (no rebuild)
```

**Terminal 2 – frontend:**

```bash
./scripts/start-front.sh          # Yew/Trunk in browser
# or
./scripts/start-tauri.sh          # Tauri desktop window
```

Stop backend stack:

```bash
./scripts/stop-back.sh
```

### Hybrid (backend on host, faster iteration)

**Terminal 1 – dependencies only (SurrealDB + Redis):**

```bash
./scripts/start-deps.sh
```

**Terminal 2 – backend (watches and rebuilds):**

```bash
just backend-watch
# or: ./scripts/backend-watch.sh
```

**Terminal 3 – frontend:**

```bash
./scripts/start-front.sh
# or ./scripts/start-tauri.sh
```

See [QUICK_ITERATION.md](../QUICK_ITERATION.md) for more detail.

## 3. Verify

- Backend: `curl http://localhost:50002/health` (or `$BACKEND_PORT` from env)
- Frontend: open `http://localhost:50003` (or `$FRONTEND_PORT`)
- SurrealDB: port 50001 (or from env)

## 4. Tests

```bash
./ci-local.sh all              # build, unit, integration, e2e
./ci-local.sh unit             # unit only
./ci-local.sh integration      # integration (stack must be up)
./ci-local.sh e2e              # E2E smoke
```

See [testing/HOW_TO_RUN_TESTS.md](../testing/HOW_TO_RUN_TESTS.md).

## File layout (relevant)

- **config/** – `.env.dev`, `.env.prod`, `setup-env.sh`, templates
- **deploy/** – single `docker-compose.yml` (SurrealDB, Redis, backend). No deploy.sh; use scripts above.
- **scripts/** – `start-back.sh`, `start-front.sh`, `start-deps.sh`, `stop-back.sh`, `backend-watch.sh`, `ci.sh`, `load-env.sh`

## Troubleshooting

- **Port in use:** Change ports in `config/.env.dev` or stop the process using the port.
- **Backend can’t reach SurrealDB:** Ensure the backend stack is up (`./scripts/start-back.sh`) or deps only (`./scripts/start-deps.sh`). Backend in Docker uses service hostname `surrealdb`; local backend uses `localhost` and the port from env.
- **Frontend can’t reach backend:** Ensure backend is running and `BACKEND_URL` / proxy in Trunk points to it (e.g. `http://localhost:50002`).
- **Clean slate:** `./scripts/stop-back.sh`, then remove the relevant tree under **`data/`** (e.g. `data/dev/surrealdb_data`) if you want to wipe DB/Redis data.

## Next steps

- [PROJECT_STRUCTURE.md](../PROJECT_STRUCTURE.md) – Repo layout and quick reference
- [WORKFLOW.txt](../WORKFLOW.txt) – CI, backend, frontend, production
- [testing/HOW_TO_RUN_TESTS.md](../testing/HOW_TO_RUN_TESTS.md) – Running tests
