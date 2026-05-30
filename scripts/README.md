# Scripts (`scripts/`)

Repo layout for ops (Option A):

| Path | Role |
|------|------|
| **`config/`** | Env **templates** (`env.*.template`) and generated **`config/.env.dev` / `config/.env.prod`** (not committed). No runtime data. |
| **`deploy/`** | **Compose files**, Caddyfile, server deploy (`deploy_stg.sh`), Surreal migrations. No runtime data. |
| **`data/`** | **Local Docker bind mounts** only (`data/dev`, `data/prod`, `data/ci-*`). Gitignored. Never put secrets here. |
| **`scripts/`** | **How to run** things locally and thin wrappers around `deploy/`. |

---

## Four commands you care about

### 1) Full test gate on **production-built** images (same as “push-ready”)

Builds backend + frontend Docker images, brings up **full** stack (`deploy/docker-compose.full.yml`), runs unit + full integration + Playwright.

**E2E:** Playwright runs **inside Docker** (`--network host`) against the prod stack on `127.0.0.1:${FRONTEND_PORT}`. The gate provisions per-run users (`E2E_USER_EMAIL`, etc.) and authenticates analytics/crud via API `global-setup` (no UI login in `beforeEach`). Build the Playwright image once when `package-lock.json` changes or on a new machine:

```bash
./scripts/build-playwright-e2e-image.sh   # needs network once (npm + Playwright CDN)
```

Then `./scripts/run-playwright-e2e-docker.sh` / the full gate do **not** download browsers at test time. Override with `FULL_PROD_TEST_PLAYWRIGHT_HOST=1` to run on the host instead (needs Node).

```bash
./scripts/test-prod-gate.sh
# same as: ./scripts/full-prod-test.sh
```

### 2) **Dev** stack for VS Code / breakpoints (DB in Docker, backend on host)

Surreal + Redis in Docker; you run the Rust API locally so lldb / CodeLLDB breakpoints work.

```bash
./scripts/dev-debug.sh
# then: just backend-watch   OR   source scripts/load-env.sh dev && cargo run -p backend
# frontend: ./scripts/start-front.sh  or  ./scripts/start-tauri.sh
```

---

## AI (local, self-hosted)

The backend exposes:

- `POST /api/ai/ask`
- `POST /api/ai/smacktalk`

For local dev stacks started via `./scripts/start-back.sh` or `./scripts/full-prod-test.sh`, the Compose stack now includes an `ollama` service and the backend defaults to `LLM_BASE_URL=http://ollama:11434`.

To override:

```bash
# dev (host backend) example
export LLM_BASE_URL="http://127.0.0.1:11434"
export LLM_MODEL="llama3.2:3b"
```

### 3) **Quick** prod-like test (shorter than the full gate)

Build + unit + same Docker stack prep as integration + **`testing` `api_tests`** + one ignored backend smoke test, then **compose down**.

```bash
./scripts/test-prod-like-smoke.sh          # uses config/.env.prod
./scripts/test-prod-like-smoke.sh dev      # optional: config/.env.dev
# same as: ./ci-local.sh smoke prod
```

### 4) **Production server**: install images built in GitHub Actions

On the host, from a tree that includes **`deploy/`** (and `config/.env.prod` next to it as laid out in `deploy/README.md`):

```bash
./scripts/install-from-ci.sh <tag>   # e.g. latest or the CI short sha
# runs deploy/deploy_stg.sh — set DEPLOY_ROOT if your deploy folder is not repo-root/deploy
```

---

## Other scripts (still supported)

| Script | Purpose |
|--------|---------|
| `./ci-local.sh …` | Wrapper for `./scripts/ci.sh` (`build`, `unit`, `smoke`, `integration`, `e2e`, `all`). |
| `./scripts/start-back.sh` | Full backend stack in Docker (dev default). |
| `./scripts/start-deps.sh` | Surreal + Redis only (for hybrid dev). |
| `./scripts/stop-back.sh` | Stop compose stack. |
| `./scripts/load-env.sh` | Sourced by other scripts; loads `config/.env.*`, normalizes `VOLUME_PATH`. |
| `./scripts/apply-surreal-schema-minimal.sh` | Minimal Surreal schema (tests). |
| `./scripts/apply-surreal-functions.sh` | Apply `tools/arango-to-surreal/surreal-functions.surql`. |
| `./scripts/run-surreal-script.sh` | Run a `.surql` file against local Surreal (HTTP). |
| `./scripts/run-integration-tests.sh` | Re-run `cargo test -p testing` with host URLs when stack is already up. |
| `./scripts/build-playwright-e2e-image.sh` | Build `stg-playwright-e2e` image (npm ci + Chromium baked in; run when lockfile changes). |
| `./scripts/run-playwright-e2e-docker.sh` | Run Playwright E2E in the pre-baked container (see script header for `PLAYWRIGHT_*` env vars). |
| `./scripts/verify-surreal-local.sh` | Data checks against localhost Surreal. |
| `./scripts/test-surrealdb-auth.sh` | Debug which root password a running Surreal accepts. |
| `./scripts/smoke-test-player-auth.sh` | Quick HTTP smoke vs `BASE_URL` (see `testing/INTEGRATION_TEST_GUIDE.md`). |
| `./scripts/backend-watch.sh` | `cargo watch` for backend (also: `just backend-watch`). |
| `./scripts/start-front.sh` / `start-tauri.sh` | Frontend dev servers. |
| `./scripts/import-bgg-catalog.sh`, `arango-to-surreal-import.sh` | Data tooling. |
| `./scripts/backfill-contest-names.sh` | Dry-run or `--apply` retro rename of contest titles (`{Game} — {Weekday Mon D}`). |

Create env files once: `./config/setup-env.sh dev` and `./config/setup-env.sh prod`.
