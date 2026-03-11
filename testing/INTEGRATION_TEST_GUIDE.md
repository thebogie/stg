# Integration Testing: Official Stack Only

Integration tests run against the **same SurrealDB + Redis stack** used in dev and production. Start the stack with `./deploy/stack.sh start`, then run tests. No testcontainers.

## Quick start

```bash
./deploy/stack.sh start
cargo test -p testing
```

Or in one go:

```bash
./deploy/ci-local.sh integration   # starts stack if needed, then runs tests
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

`TestEnvironment` reads from env: `SURREAL_URL`, `REDIS_URL`, `SURREAL_NS`, `SURREAL_DB`, `SURREAL_USER`, `SURREAL_PASSWORD`. The same vars are set by `./deploy/ci-local.sh` and by your stack.

## Environment variables

| Variable          | Default                    | Purpose              |
|-------------------|----------------------------|----------------------|
| `SURREAL_URL`     | `http://127.0.0.1:50001`   | SurrealDB HTTP URL   |
| `REDIS_URL`       | `redis://127.0.0.1:6379/`  | Redis URL            |
| `SURREAL_NS`      | `stg_rd`                   | SurrealDB namespace  |
| `SURREAL_DB`      | `stg_rd`                   | SurrealDB database   |
| `SURREAL_USER`    | `root`                     | SurrealDB user       |
| `SURREAL_PASSWORD`| `root`                     | SurrealDB password   |

Ports match `deploy/docker-compose.ci.yml` + `docker-compose.local.yml` (e.g. SurrealDB 50001, Redis 6379).

## Why no testcontainers

We test the **official containers** that go to production. The same `surrealdb/surrealdb` and `redis` images and compose used in CI and prod are what run in dev; integration tests hit that stack via env vars. One stack, one set of images, everywhere.
