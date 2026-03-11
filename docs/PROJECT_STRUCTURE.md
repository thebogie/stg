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

## Shared and tooling

- **shared** – Types and code shared by front/web, back/api, and tests.
- **testing** – Integration test crate (SurrealDB + Redis; same stack as production).
- **scripts/** – Dev and CI scripts (start-back, start-front, start-deps, ci, load-env, etc.). See `scripts/README.md`.
- **tools/arango-to-surreal** – One-off Arango → Surreal conversion (optional; for migration only).

## Build and run (quick reference)

- **CI (all tests):** `./ci-local.sh all` or `./ci-local.sh [build|unit|integration|e2e]`. Uses `config/.env.prod` and `deploy/docker-compose.yml`.
- **Backend (terminal 1):** `./scripts/start-back.sh` (builds and starts SurrealDB + Redis + backend). Stop: `./scripts/stop-back.sh`.
- **Frontend (terminal 2):** `./scripts/start-front.sh` (Yew/Trunk) or `./scripts/start-tauri.sh` (Tauri). Uses `config/.env.dev`.
- **Hybrid dev (backend on host):** `./scripts/start-deps.sh` then `just backend-watch` (terminal 2) and `./scripts/start-front.sh` or `./scripts/start-tauri.sh` (terminal 3). See `docs/QUICK_ITERATION.md`.
- **Production:** Build and push backend image via GitHub Actions; production pulls from GHCR. See `docs/GHCR_SETUP.md`.
