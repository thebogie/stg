# Project structure

This document is the single source of truth for repo layout. Cargo workspace root is the repo root; the backend package name is `backend` (source under `back/api`).

## Front (clients)

| Path | Description |
|------|-------------|
| **front/web** | Yew (WASM) web app. Build with Trunk; run standalone with `./scripts/start-front.sh` or serve in Docker. |
| **front/tauri** | Tauri desktop/mobile shell; embeds the same Yew app. Run with `./scripts/start-tauri.sh`. |

Both talk to the same **back** API (see config `BACKEND_URL` / `BACKEND_PORT`).

## Back (server + data)

| Path | Description |
|------|-------------|
| **back/api** | Rust API (Actix-web). Package name: `backend`. Uses SurrealDB and Redis. |
| **deploy/** | Single Docker Compose: SurrealDB, Redis, backend. Used by `./scripts/start-back.sh` and `./ci-local.sh`. Frontend runs standalone (not in this compose). |

SurrealDB and Redis run as containers only (see `deploy/docker-compose.yml`). No ArangoDB in the current stack.

## Config and env

| Path | Description |
|------|-------------|
| **config/** | Env templates and generated env files. |
| | Create: `./config/setup-env.sh dev` → `config/.env.dev`; `./config/setup-env.sh prod` → `config/.env.prod`. |
| | Scripts use `config/.env.dev` or `config/.env.prod` (see `scripts/load-env.sh`). |

## Local Docker data (`data/`)

| Path | Description |
|------|-------------|
| **data/** | Gitignored bind-mount roots for SurrealDB, Redis, and backend file cache. Defaults: `data/dev`, `data/prod`; CI uses `data/ci-<env>`. Set **`VOLUME_PATH`** in `.env.*` (templates use `data/dev` or `data/prod`; production servers should use an **absolute** path outside the repo). |

Do not store secrets under `data/`. Do not put compose files or `.env` files here—use **`deploy/`** and **`config/`**.

## Shared and tooling

- **shared** – Types and code shared by front/web, back/api, and tests.
- **testing** – Integration test crate (SurrealDB + Redis; same stack as production).
- **scripts/** – How to run local stacks and tests. Canonical overview: **`scripts/README.md`** (four primary workflows).
- **tools/arango-to-surreal** – One-off Arango → Surreal conversion (optional; for migration only).

## Build and run (quick reference)

- **Full prod-image gate (unit + full integration + Playwright):** `./scripts/test-prod-gate.sh` (wraps `full-prod-test.sh`). Playwright E2E defaults to **Docker** (`./scripts/run-playwright-e2e-docker.sh`); host run needs `FULL_PROD_TEST_PLAYWRIGHT_HOST=1`.
- **Quick prod-like smoke:** `./scripts/test-prod-like-smoke.sh` or `./ci-local.sh smoke prod`.
- **CI driver (stages):** `./ci-local.sh [build|unit|smoke|integration|e2e|all] [dev|prod]`. Uses `deploy/docker-compose.yml` and `config/.env.*`.
- **Daily dev (incl. prod snapshot):** `docs/DAILY_WORKFLOW.md`.
- **Dev + breakpoints:** `./scripts/dev-debug.sh` then `just backend-watch`.
- **Backend in Docker:** `./scripts/start-back.sh` · stop: `./scripts/stop-back.sh`.
- **Frontend:** `./scripts/start-front.sh` or `./scripts/start-tauri.sh` (uses `config/.env.dev`).
- **Production install from GHCR:** `./scripts/install-from-ci.sh <tag>` on the server (wraps `deploy/deploy_stg.sh`). See `docs/GHCR_SETUP.md` and `deploy/README.md`.
