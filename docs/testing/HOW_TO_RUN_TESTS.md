# How to run tests

## Recommended: full CI locally

```bash
./ci-local.sh all
```

Runs, in order: build → unit tests → start stack (SurrealDB, Redis, backend) → integration tests → E2E smoke → tear down. Uses `config/.env.prod` and `deploy/docker-compose.yml`. This is the same flow used for pre-push and can be mirrored in CI.

Individual phases:

```bash
./ci-local.sh build         # cargo build -p backend
./ci-local.sh unit          # cargo test -p backend (no Docker)
./ci-local.sh integration   # cargo test -p testing (stack must be up)
./ci-local.sh e2e           # stack up, health check, then down
```

## Backend unit tests (no Docker)

```bash
cargo nextest run -p backend --no-fail-fast
# or
just test-backend
```

Fast; no services required.

## Integration tests

Require the backend stack to be running (SurrealDB, Redis, backend). Start it with:

```bash
./scripts/start-back.sh
```

Then either run integration as part of full CI (`./ci-local.sh integration`) or run the testing crate:

```bash
cargo test -p testing
```

(If your Justfile has `just test-integration` calling a script that exists, you can use that; otherwise use `./ci-local.sh integration` after starting the stack.)

## Frontend E2E (Playwright)

Playwright E2E tests are run by `./ci-local.sh e2e` as a smoke step. To run the full Playwright suite manually:

```bash
npx playwright test
```

You may need to start the stack first (`./scripts/start-back.sh`, `./scripts/start-front.sh`) or use Playwright config that starts services. Check `playwright.config.ts` and the Justfile for any `test-frontend-e2e` / image-build steps; some of those scripts may not exist in this repo—use `./ci-local.sh all` for a single command that runs everything that is wired up.

## Prerequisites

- **Docker** – Required for `ci-local.sh` (integration and e2e) and for running the stack.
- **Env** – `config/.env.prod` (create via `./config/setup-env.sh prod`). Used by `ci-local.sh`.
- **Rust** – `cargo nextest run` (install with `cargo install nextest-cli` if needed).

## Summary

| Goal              | Command |
|-------------------|--------|
| Run everything    | `./ci-local.sh all` |
| Unit only         | `./ci-local.sh unit` or `just test-backend` |
| Integration       | Start stack, then `./ci-local.sh integration` or `cargo test -p testing` |
| E2E smoke         | `./ci-local.sh e2e` |
| Frontend E2E (Playwright) | `npx playwright test` (stack or config as required) |
