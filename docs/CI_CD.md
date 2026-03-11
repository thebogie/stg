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
| Integration   | `./ci-local.sh integration`| Stack must be up; `cargo test -p testing` |
| E2E (smoke)   | `./ci-local.sh e2e`        | Full stack up, health check, then down |
| **All**       | `./ci-local.sh all`        | build → unit → start stack → integration → e2e |

Uses `config/.env.prod` and `deploy/docker-compose.yml`. From repo root:

```bash
./ci-local.sh all
```

(`ci-local.sh` is a wrapper for `scripts/ci.sh`.)

## Production: build and deploy

- **Build and push:** On push to `main`, GitHub Actions builds the backend image from `back/api/Dockerfile.backend` and pushes to GHCR (e.g. `ghcr.io/OWNER/REPO/backend:latest`).
- **Deploy:** On the production server, pull the image and run the stack. See **[GHCR_SETUP.md](GHCR_SETUP.md)** for one-time setup and commands.

No `build-prod-images.sh`, `deploy-production.sh`, or `test.sh` in the repo; the current path is GHCR for backend and the single compose file.

## File reference

| What            | Where |
|-----------------|--------|
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
