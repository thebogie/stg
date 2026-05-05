# Integration Testing: Official Stack Only

Integration tests run against the **same SurrealDB + Redis stack** used in dev and production. Prefer **`./ci-local.sh integration prod`** or **`./scripts/run-integration-tests.sh`** after the stack is up. No testcontainers.

## Verify the app works first (smoke test)

Before debugging failing integration tests, confirm the **running backend** (real SurrealDB + Redis) can register and log in:

1. Start stack: `./scripts/start-back.sh prod` (or bring up deps + backend via `./ci-local.sh smoke prod` for a shorter automated path)
2. Apply minimal schema: `source scripts/load-env.sh prod && ./scripts/apply-surreal-schema-minimal.sh`
3. Start the backend (e.g. from `back/api`: `source ../../scripts/load-env.sh prod && cargo run`, or use the backend container if the full stack is up)
4. Run: `./scripts/smoke-test-player-auth.sh [BASE_URL]`  
   Default BASE_URL is `http://127.0.0.1:50002`. Exit 0 means register → login → GET /api/players/me all succeeded.

If the smoke test passes, the source code works with the real stack; test failures are then likely test-environment or test-app setup. If the smoke test fails (e.g. login 404), fix the backend/repository first.

## Quick start

**Recommended (host-accessible URLs):** Use the script so tests always use `127.0.0.1` (avoids "name resolution" errors if your env has Docker-internal hostnames like `surrealdb`):

```bash
# From repo root (scripts set --project-directory so volume paths resolve correctly):
./ci-local.sh integration prod
# Or if the stack is already up, only the tests:
./scripts/run-integration-tests.sh -- --include-ignored --test-threads=1
```

To run only auth integration tests:

```bash
./scripts/run-integration-tests.sh -- auth_integration --include-ignored --test-threads=1
```

Or run tests manually (ensure `SURREAL_URL` and `REDIS_URL` are host-accessible, e.g. `127.0.0.1`):

```bash
source scripts/load-env.sh prod
export SURREAL_URL="http://127.0.0.1:${SURREALDB_PORT}" REDIS_URL="redis://127.0.0.1:${REDIS_PORT}/"
cargo test -p testing -- --include-ignored --test-threads=1
```

Or in one go (from repo root):

```bash
./ci-local.sh integration prod
```

## In test code

```rust
use testing::{TestEnvironment, app_setup};

#[tokio::test]
async fn test_something() -> Result<()> {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    let app_data = app_setup::setup_test_app_data(&env).await?;
    // use app_data.* repos, session_store, etc.
    Ok(())
}
```

`TestEnvironment` reads from env: `SURREAL_URL`, `REDIS_URL`, `SURREAL_NS`, `SURREAL_DB`, `SURREAL_USER`, `SURREAL_PASSWORD`. The same vars are set by **`./ci-local.sh integration`** / **`./scripts/run-integration-tests.sh`** and by your stack.

## Environment variables

| Variable          | Default                    | Purpose              |
|-------------------|----------------------------|----------------------|
| `SURREAL_URL`     | `http://127.0.0.1:50001`   | SurrealDB HTTP URL   |
| `REDIS_URL`       | `redis://127.0.0.1:6379/`  | Redis URL            |
| `SURREAL_NS`      | `stg_rd`                   | SurrealDB namespace  |
| `SURREAL_DB`      | `stg_rd`                   | SurrealDB database   |
| `SURREAL_USER`    | `root`                     | SurrealDB user       |
| `SURREAL_PASSWORD`| `root`                     | SurrealDB password   |

Ports match **`deploy/docker-compose.yml`** and `config/.env.*` (e.g. SurrealDB 50001, Redis 6379).

## Why no testcontainers

We test the **official containers** that go to production. The same `surrealdb/surrealdb` and `redis` images and compose used in CI and prod are what run in dev; integration tests hit that stack via env vars. One stack, one set of images, everywhere.
