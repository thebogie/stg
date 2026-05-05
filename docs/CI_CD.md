# CI/CD

Single pipeline, same stack everywhere (SurrealDB + Redis, backend). No testcontainers; tests run against the same Docker stack used for local dev and production.

## Pipeline overview

```
Build → Unit tests → (Start stack) → Integration tests → E2E smoke → (Production: push to main → GHCR → pull on server)
```

- **One stack:** `deploy/docker-compose.yml` (SurrealDB, Redis, backend). Frontend runs standalone for dev; production may serve it via the same host or separately.
- **Local / CI:** Same commands. CI can mirror these steps (e.g. in GitHub Actions).

## Local and CI (same flow)

| Step          | Command                    | Notes |
|---------------|----------------------------|--------|
| Build         | `./ci-local.sh build`      | `cargo build -p backend` |
| Unit tests    | `./ci-local.sh unit`       | `cargo test -p backend` (no stack) |
| Prod-like smoke | `./ci-local.sh smoke prod` | build + unit + stack + short tests, then down |
| Integration   | `./ci-local.sh integration`| Stack must be up; `cargo test -p testing` |
| E2E (smoke)   | `./ci-local.sh e2e`        | Full stack up, health check, then down |
| **All**       | `./ci-local.sh all`        | build → unit → start stack → integration → e2e |

Uses `config/.env.prod` and `deploy/docker-compose.yml`. From repo root:

```bash
./ci-local.sh all
```

(`ci-local.sh` is a wrapper for `scripts/ci.sh`.)

## Production: build, test, and deploy

### Pre-push: test exact production images

Run **`./scripts/test-prod-gate.sh`** (same as `full-prod-test.sh`) before committing. It:

1. Builds a **production** backend image with a single **build version** (e.g. `20250312-143022-abc1234` = date + short SHA).
2. Starts the **same** stack as production (SurrealDB, Redis, backend) using that image.
3. Runs **unit** and **integration** tests against that stack (production data/config).
4. Starts the **frontend** (Trunk) and runs **full Playwright E2E** against backend + frontend.
5. Writes results to **`_build/<build_version>/`** (summary.json, summary.txt, unit.log, integration.log, e2e.log, e2e/test-results, e2e/playwright-report).
6. The build version appears as the Docker image tag and in the backend `/api/version` (and Tauri/Yew footer via `IMAGE_TAG`).

If everything passes, you can commit. Pushing to `main` triggers GHCR to build the **same** production image (same Dockerfile and build-args), tagged with commit SHA and `latest`.

### Build and push (GHCR)

- On push to `main`, GitHub Actions builds the backend image from `back/api/Dockerfile.backend` and pushes to GHCR (e.g. `ghcr.io/OWNER/REPO/backend:latest` and `backend:<sha>`).
- **Deploy:** On the production server, pull the image and run the stack. See **[GHCR_SETUP.md](GHCR_SETUP.md)** for one-time setup and commands.

## Integration tests and production data

- **Auth (and other integration) tests do not require production data.** They create their own users via `/api/players/register` and only need SurrealDB + Redis and correct credentials.
- **Credentials** in `config/.env.prod` (`SURREAL_USER`, `SURREAL_PASSWORD`) must match what the SurrealDB container uses (default `root`/`root` in compose).
- **`scripts/test-prod-gate.sh`** applies a minimal schema (player table only) from `docs/surreal-schema-minimal-tests.surql` after Surreal is up, so integration tests run against a fresh DB even when no production data is loaded. To apply it manually: `source scripts/load-env.sh prod && ./scripts/apply-surreal-schema-minimal.sh`.

## File reference

| What            | Where |
|-----------------|--------|
| **Prod build + test** | `scripts/test-prod-gate.sh` / `scripts/full-prod-test.sh` (build version → _build/<version>/) |
| CI script       | `ci-local.sh` (root), `scripts/ci.sh` |
| Compose         | `deploy/docker-compose.yml` |
| Env             | `config/.env.dev`, `config/.env.prod` (from `config/setup-env.sh dev|prod`) |
| Backend source  | `back/api` (package name: `backend`) |
| Workflow summary| `docs/WORKFLOW.txt` |
| Production deploy | `docs/GHCR_SETUP.md` |

## Optional: GitHub Actions

To run the same flow in CI:

1. Checkout, set up Rust (e.g. `actions-rs/toolchain`).
2. (Optional) `cargo fmt -- --check`, `cargo clippy`.
3. `./ci-local.sh build` then `./ci-local.sh unit`.
4. Start stack (e.g. `docker compose -f deploy/docker-compose.yml --env-file config/.env.prod up -d`), then `./ci-local.sh integration` and `./ci-local.sh e2e`.
5. On main: build and push backend image to GHCR (see existing workflow or GHCR_SETUP).

Use the same env vars as local; inject secrets via the CI platform.
