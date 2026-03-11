# Deploy

Single stack: **SurrealDB + Redis + backend** (production-like). Frontend runs standalone via `scripts/start-front.sh`.

## Compose

- **docker-compose.yml** — SurrealDB, Redis, backend. Used by `scripts/ci.sh` and `scripts/start-back.sh`.
- Requires **config/.env.prod** (create from `config/env.prod.template` via `./config/setup-env.sh prod`).

## Usage

- **CI:** `./ci-local.sh all` (build, unit, integration, e2e).
- **Terminal 1 (backend):** `./scripts/start-back.sh`.
- **Terminal 2 (frontend):** `./scripts/start-front.sh`.

Data dirs: `VOLUME_PATH` (default `./docker-data`). Scripts create `surrealdb_data`, `redis_data`, `backend_data` and set permissions as needed.

**When `start-back.sh` is running, you should see:**

| Type   | Name                | Purpose |
|--------|---------------------|--------|
| Network| `stg`               | Bridge network for all services |
| Container | `stg-wait-for-surrealdb` | One-off: waited for SurrealDB then exited |
| Container | `stg-surrealdb`  | SurrealDB (host 50001 → 8000) |
| Container | `stg-redis`      | Redis (host 6379 → 6379) |
| Container | `stg-backend`    | Backend API (host 50002 → 50002) |

Project name is **stg** (set in scripts). To inspect: `docker compose -p stg -f deploy/docker-compose.yml --env-file config/.env.dev ps`.
