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

1. **Start the official stack** (deps only):
   ```bash
   ./deploy/stack.sh start
   ```
2. **Run tests** (they use `SURREAL_URL`, `REDIS_URL` from env, same as stack):
   ```bash
   cargo test -p testing
   ```
   Or use the combined CI script (starts stack if needed, then runs build/unit/integration/e2e):
   ```bash
   ./deploy/ci-local.sh integration
   # or
   ./ci-local.sh all
   ```

## What gets tested

- **Unit** (`cargo test -p backend`): No stack required.
- **Integration** (`cargo test -p testing`): Uses the stack from `./deploy/stack.sh start` (SurrealDB + Redis).
- **E2E** (`./deploy/ci-local.sh e2e`): Full stack (backend + frontend) in Docker, then smoke checks.

Same compose and images for local dev, CI, and production.
