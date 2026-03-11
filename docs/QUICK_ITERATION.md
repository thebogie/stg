# Quick iteration: backend, frontend, database

Exact steps to get backend, frontend, and database up so you can fix errors with minimal restarts.

---

## Prerequisites (one-time)

1. **Config**
   ```bash
   ./config/setup-env.sh dev
   ```
   (Creates `config/.env.dev` if missing.)

2. **Tools** (for backend watch mode)
   ```bash
   just setup
   ```
   Installs `cargo-nextest`, `cargo-llvm-cov`, `cargo-watch`, Playwright.

3. **Data** (one of):
   - Put a SurrealDB export at `_build/smacktalk.surql`, or  
   - Put an ArangoDB backup at `~/work/_backups/smacktalk.zip`  
   The start scripts will import it when they run.

---

## Option A: Full stack in Docker (simplest)

Everything in Docker; backend restarts only when you rebuild the image.

| Step | Terminal | Command |
|------|----------|--------|
| 1 | 1 | `./scripts/start-back.sh` |
| 2 | 2 | `./scripts/start-tauri.sh` |

- Backend: http://127.0.0.1:50002  
- Frontend: Tauri app (Trunk dev server inside Tauri).  
- To restart backend after code changes: `./scripts/start-back.sh --no-build` (recreates container) or rebuild: `./scripts/start-back.sh`.

---

## Option B: DB in Docker, backend local with watch (fastest iteration)

Database and Redis in Docker; backend runs locally and **auto-restarts on save**.

| Step | Terminal | Command |
|------|----------|--------|
| 1 | 1 | `./scripts/start-deps.sh` |
| 2 | 2 | `just backend-watch` |
| 3 | 3 | `./scripts/start-tauri.sh` |

- **Terminal 1:** SurrealDB + Redis start; if `_build/smacktalk.surql` or backup zip exists, DB is reset and imported.  
- **Terminal 2:** Backend runs locally; every time you save a file in `back/api` or `shared`, it rebuilds and restarts. `just backend-watch` loads `config/.env.dev` for you (no need to source anything).  
- **Terminal 3:** Tauri (frontend) runs; point it at backend on port 50002.

---

## Option C: You already ran full stack; switch to local backend

You ran `./scripts/start-back.sh` and want to switch to local backend + watch without re-importing.

| Step | Terminal | Command |
|------|----------|--------|
| 1 | 1 | `docker stop stg-backend` |
| 2 | 2 | `just backend-watch` |
| 3 | 3 | `./scripts/start-tauri.sh` (if not already running) |

DB and Redis keep running in Docker; only the backend container is stopped. Local backend talks to SurrealDB and Redis on localhost.

---

## Verify

- **Backend:** `curl -s http://127.0.0.1:50002/health`  
- **SurrealDB:** e.g. Surrealist at `http://127.0.0.1:50001` (ns/db: stg_rd/stg_rd).  
- **Frontend:** Tauri window opens; leaderboard and other pages hit the backend.

---

## Stop

- **Option A:** `./scripts/stop-back.sh` then close Tauri.  
- **Option B:** Ctrl+C in terminals 2 and 3; then `docker compose -f deploy/docker-compose.yml --env-file config/.env.dev down` (or stop only surrealdb/redis if you prefer).  
- **Option C:** Ctrl+C in terminal 2; `./scripts/stop-back.sh` to bring down the rest.

---

## Summary

| Goal | Use |
|------|-----|
| Easiest: everything in Docker | Option A |
| Fast backend iteration (restart on save) | Option B |
| Already have full stack, switch to local backend | Option C |
