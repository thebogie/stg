# Archived documentation

These docs are kept for reference only. They describe **obsolete** setups, scripts, or stacks that are no longer current. The project follows **Rust, Yew, Tauri, SurrealDB, and Redis**; see `docs/README.md`, `docs/PROJECT_STRUCTURE.md`, `docs/WORKFLOW.txt`, `docs/GHCR_SETUP.md`, and `docs/CI_CD.md` for the current workflow.

## archive/outdated-stack/

Docs that **do not follow** current industry standards (SurrealDB, back/api, front/web, Tauri, Yew). Archived so we follow one canonical stack.

- **ARANGODB_TO_SURREALDB_MIGRATION.md** – Migration plan; stack is now SurrealDB.
- **ARANGO_LEFTOVERS_AUDIT.md** – Grep patterns for Arango leftovers; reference only.
- **PLAN_FIX_DATA_AND_GETTING_DATA.md** – One-time post-migration fix plan.
- **SURREALDB_MIGRATION_STATUS.md** – Migration status checklist (completed).
- **SURREALDB_CODE_MIGRATION.md** – Code migration checklist (completed).
- **VERIFY_IMPORT_MANUAL.md** – Arango→Surreal import verification (one-time).
- **version-system.md** – Referenced old path `frontend/`; current frontend is `front/web`.
- **CI_CD_WORKFLOW.md** – Older flow (ArangoDB, setup-hybrid-dev.sh, start-frontend.sh). Use `docs/CI_CD.md`.
- **CACHE_TEST_COVERAGE.md** – Referenced `backend/src`; current backend is `back/api`.
- **TEST_STRATEGY.md** – Testcontainers + ArangoDB; tests now use SurrealDB + Redis stack.

## Other archived (root of archive/)

- **TESTCONTAINERS_COMPLETE.md** – Testcontainers removed; tests use the same SurrealDB + Redis stack.
- **E2E_*.md** – Old E2E Docker setup; E2E is run via `./ci-local.sh e2e` with the same stack.
- **HYBRID_DEVELOPMENT.md** – ArangoDB hybrid dev; backend now uses SurrealDB only.
- **RUN_ALL_TESTS.md** – Referenced `scripts/run-all-tests.sh`, which does not exist. Use `./ci-local.sh all` (see `docs/testing/HOW_TO_RUN_TESTS.md`).
- **root-md/** – Old root-level docs (e.g. DEPLOY_TO_PRODUCTION, README_QUICK_START, CICD_WORKFLOW) that were moved here; deployment is now in `docs/GHCR_SETUP.md`.
