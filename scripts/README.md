# Scripts

Only four scripts; three scenarios.

## 1. CI (all tests locally)

```bash
./ci-local.sh all
# or: ./scripts/ci.sh [build|unit|integration|e2e|all]
```

Uses **config/.env.prod** and **deploy/docker-compose.yml**. Builds, runs unit tests, starts the stack, runs integration tests, runs e2e smoke, then brings the stack down (for the e2e step).

## 2. Backend (terminal 1)

```bash
./scripts/start-back.sh              # start and build if needed
./scripts/start-back.sh --no-build   # start existing images only (no rebuild)
./scripts/stop-back.sh               # stop the stack
```

Uses **config/.env.dev** (or .env.prod). With `--no-build`, compose only runs `up -d` (no `--build`).

**Arango → Surreal import to localhost:50001** (SurrealDB must already be running, e.g. from `start-back.sh` or Surrealist):

```bash
./scripts/arango-to-surreal-import.sh [path/to/smacktalk.zip] [--fresh]
```

- Zip path defaults to `ARANGO_BACKUP_ZIP` or `~/work/_backups/smacktalk.zip`.
- `--fresh`: wipes the target namespace/database before import so you get a clean load.
- Output .surql: `_build/smacktalk.surql` (or `SURREAL_IMPORT_SURQL`). Uses Docker for `surreal import`; no local SurrealDB CLI required.

**Verify SurrealDB data (run checks against localhost:50001):**

```bash
./scripts/verify-surreal-local.sh
```

Runs the same checks as `docs/verify-surreal-contest-list.surql` via **HTTP (curl)** by default so it works from WSL when SurrealDB runs in Docker Desktop (Windows). Prints PASS/FAIL per check (contest id form, edge out/in, contest list for player, counts). Requires `jq` and SurrealDB on port 50001.

- **WSL + Docker Desktop:** If `127.0.0.1:50001` doesn’t reach SurrealDB, run:  
  `SURREAL_VERIFY_URL=http://host.docker.internal:50001 ./scripts/verify-surreal-local.sh`
- **Use Docker CLI instead of curl:**  
  `SURREAL_VERIFY_USE_DOCKER=1 ./scripts/verify-surreal-local.sh`
- Override player key: `SURREAL_VERIFY_PLAYER_KEY=...`

**Run any .surql script against localhost:50001 (from WSL CLI):**

```bash
./scripts/run-surreal-script.sh docs/verify-surreal-contest-list.surql
./scripts/run-surreal-script.sh docs/verify-surreal-import.surql
```

Uses HTTP (curl) so no Surreal CLI needed. Same env as verify: `SURREAL_VERIFY_URL`, `SURREAL_NS`, `SURREAL_DB`, etc. Requires `jq` for pretty output.

## 3. Frontend (terminal 2)

```bash
./scripts/start-front.sh
```

Starts standalone frontend (Yew/Trunk; Tauri when added). Uses **config/.env.dev**. Start the backend first.

## Env

- **load-env.sh** — sourced by the above; loads **config/.env.dev** or **config/.env.prod**.
- Create env files: `./config/setup-env.sh dev` and `./config/setup-env.sh prod`.
