# Testing

Integration tests run against the **same official stack** (SurrealDB + Redis) used in dev and production. No testcontainers—the images you ship are the ones you test.

## Structure

```
testing/
├── Cargo.toml       # Testing crate
├── src/             # TestEnvironment (env-based), app_setup
├── tests/           # Integration tests
└── e2e/             # Playwright E2E (if present)
```

## Running integration tests

From the **repo root**:

1. **Bring up the stack and run integration** (recommended):

   ```bash
   ./ci-local.sh integration prod
   ```

2. **Or** start deps + backend yourself, then run tests:

   ```bash
   ./scripts/start-back.sh prod
   ./scripts/run-integration-tests.sh -- --include-ignored --test-threads=1
   ```

3. **Quick prod-like smoke** (shorter than full integration crate):

   ```bash
   ./ci-local.sh smoke prod
   ```

See **`docs/testing/HOW_TO_RUN_TESTS.md`** and **`testing/INTEGRATION_TEST_GUIDE.md`**.

## What gets tested

- **Unit** (`cargo test -p backend`): No stack required.
- **Integration** (`cargo test -p testing`): Uses the Docker stack from `deploy/docker-compose.yml`.
- **Smoke** (`./ci-local.sh smoke prod`): Build + unit + stack + `api_tests` + one ignored backend smoke test, then stack down.
- **E2E** (`./ci-local.sh e2e`): Full stack in Docker, health check, then down (see `scripts/ci.sh`).

Same compose and images for local dev, CI, and production.
