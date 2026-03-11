# Directory audit (current state)

**Stack:** SurrealDB + Redis, Yew web (`front/web`) + Tauri (`front/tauri`), backend (`back/api`). No migrations crate, no dataload crate.

---

## Keep (required)

| Directory   | Why |
|------------|-----|
| **back/api**   | Rust API (Actix). Package name `backend`. SurrealDB + Redis. |
| **front/web**  | Yew (WASM) web app. Trunk. |
| **front/tauri**| Tauri desktop/mobile; embeds same Yew app. |
| **shared**     | Shared types/DTOs between API and frontends. |
| **testing**    | Integration tests (SurrealDB + Redis). Playwright E2E at repo root. |
| **config**     | Env: `setup-env.sh dev|prod` → `.env.dev`, `.env.prod`. |
| **deploy**     | Single `docker-compose.yml` (SurrealDB, Redis, backend). Used by `scripts/start-back.sh` and `ci-local.sh`. |
| **scripts**    | Shell only: `start-back.sh`, `start-front.sh`, `start-deps.sh`, `stop-back.sh`, `ci.sh`, `load-env.sh`, `backend-watch.sh`, `arango-to-surreal-import.sh`, `verify-surreal-local.sh`, `run-surreal-script.sh`. |
| **.cargo**     | Cargo config. Needed for builds. |

---

## Optional

| Path                    | Notes |
|-------------------------|--------|
| **tools/arango-to-surreal** | One-off Arango → Surreal conversion. Keep if you still migrate from Arango; else archive or remove. |

---

## Generated / local (do not commit)

| Path           | Notes |
|----------------|--------|
| **_build**     | Build outputs, logs. Gitignore. |
| **target**     | Cargo build. Gitignore. |
| **node_modules** | npm. Gitignore. |
| **test-results** | Playwright. Gitignore. |
| **docker-data** | Local Docker volumes. Gitignore. |

---

## Reference

- **Single source of truth for layout:** [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md)
- **Workflow:** [WORKFLOW.txt](WORKFLOW.txt)
- **Deployment:** [GHCR_SETUP.md](GHCR_SETUP.md)
