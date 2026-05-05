# Deploy

This directory is **self-contained** for production: copy the whole `deploy/` folder to your server (e.g. `scp -r deploy/ user@host:/opt/stg/deploy`) and run from there. No repo clone required on the server.

---

## Production (server)

### One-time setup

1. Copy `deploy/` to the server (e.g. `/opt/stg/deploy`). Do **not** copy local **`data/`** from a dev machine; production data lives at **`VOLUME_PATH`** on the server (absolute path).
2. Create env: `cp config/env.prod.template config/.env.prod` and edit `config/.env.prod`: set **`VOLUME_PATH`** to an absolute path **outside** the deploy folder (e.g. `/opt/stg/data`), plus passwords, etc. Compose will create `surrealdb_data`, `redis_data`, `backend_data` under that path.
3. (Optional) Install hourly SurrealDB backup cron: from `deploy/` run `sudo ./setup_cron_backup_for_surreal.sh`.

### Deploy / update

From the **deploy/** directory on the server:

```bash
./deploy_stg.sh <tag>
```

- `<tag>` = image tag to pull and run (e.g. `latest`, or short SHA from CI like `0013844`).
- Script: stops service → pulls backend + frontend images from GHCR → compose down → starts `surrealdb`+`redis` → runs `run_surreal_migrations.sh` (all `deploy/migrations/*.surql`) → starts full stack.
- Deploy aborts if migration fails.

See **WEB_AND_TAURI.md** for CI/CD overview and Tauri setup; **env.tauri.prod.template** for production API URL for the desktop app.

### Contents (production)

| File / dir | Purpose |
|------------|--------|
| **deploy_stg.sh** | Deploy/update: pull images, restart full stack. Run from `deploy/`. |
| **run_surreal_migrations.sh** | Runs SurrealDB migrations in lexical order from `deploy/migrations/*.surql`. |
| **migrations/** | Versioned SurrealDB migration files (`*.surql`) applied during deploy before backend/frontend start. |
| **setup_cron_backup_for_surreal.sh** | One-time: install cron for hourly SurrealDB backups. |
| **docker-compose.full.yml** | Full stack: SurrealDB, Redis, backend, frontend. Uses `BACKEND_IMAGE` and `FRONTEND_IMAGE` (set by deploy_stg.sh). |
| **Caddyfile.frontend** | Caddy config: static SPA + proxy `/api`, `/health`, `/version` to backend. |
| **config/env.prod.template** | Template for `config/.env.prod` (required). |
| **config/.env.prod** | Production env (you create from template; not in git). Must set `VOLUME_PATH` to an absolute path outside `deploy/` (e.g. `/opt/stg/data`). |
| **WEB_AND_TAURI.md** | CI/CD, web + Tauri, deploy flow. |
| **env.tauri.prod.template** | Example `STG_API_URL` for Tauri production. |

---

## Local / CI (from repo root)

- **docker-compose.yml** — SurrealDB, Redis, backend. Used by `scripts/ci.sh` and `scripts/start-back.sh`.
- **docker-compose.full.yml** — Same plus web frontend. For full-stack runs (e.g. `scripts/full-prod-test.sh`) set `BACKEND_IMAGE` and `FRONTEND_IMAGE` to your built images.
- Frontend runs standalone via `scripts/start-front.sh` if you prefer.

Run full stack locally (builds images, then compose up): `./scripts/test-prod-gate.sh` (same as `full-prod-test.sh`). Local bind mounts live under repo-root **`data/`** via `VOLUME_PATH` (defaults from `scripts/load-env.sh` / `config/env.*.template`; CI uses `data/ci-<env>`). Project name: **stg**.

On the server, prefer **`./scripts/install-from-ci.sh <tag>`** from a full repo clone, or run **`./deploy_stg.sh <tag>`** from this directory—both pull the same GHCR images CI built.

---

## DB Migration Workflow (N -> N+1)

1. Add one or more versioned files in `deploy/migrations/` (example: `20260319T190000_n_plus_1.surql`).
2. Make each migration file idempotent (safe to re-run).
3. Deploy with `./deploy_stg.sh <tag>`.
4. The deploy script runs DB migration before starting backend/frontend; if migration fails, app update is blocked.
