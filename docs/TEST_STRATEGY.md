# Test Strategy

## Test Categories

### 1. Unit Tests (`--lib`)
- **What**: Tests that don't require external dependencies
- **Where**: `backend/src/**/*.rs` (test modules with `#[cfg(test)]`)
- **Containers**: None
- **Command**: `cargo nextest run --workspace --lib`

### 2. Integration Tests (testcontainers)
- **What**: Tests that use `TestEnvironment` from `testing` package
- **Where**: `testing/tests/**/*.rs`, some tests in `backend/tests/` that use `TestEnvironment`
- **Containers**: Ephemeral testcontainers (ArangoDB + Redis) - created per test, destroyed after
- **How**: Tests create `TestEnvironment::new().await?` which spins up containers
- **Command**: `cargo nextest run --package testing --test '*'`
- **Note**: Should NOT use production containers - they manage their own containers

### 3. E2E-style Integration Tests (Production Containers)
- **What**: Tests that use `BACKEND_URL` to connect to running backend (HTTP-based)
- **Where**: 
  - `backend/tests/db_search_integration_test.rs`
  - `backend/tests/venue_update_integration_test.rs`
  - `backend/tests/contest_search_integration_test.rs`
- **Containers**: Production containers (started by test script)
- **Command**: `cargo nextest run --package backend --test 'db_search_integration_test' --test 'venue_update_integration_test' --test 'contest_search_integration_test'`

### 4. E2E Tests (Playwright)
- **What**: Frontend end-to-end tests
- **Where**: `testing/**/*.spec.ts`
- **Containers**: Production containers (full stack)
- **Command**: `npx playwright test`

### 5. Hybrid Integration Tests (Env-based)
- **What**: Tests that use `REDIS_URL` or `ARANGO_URL` env vars (can use testcontainers or production)
- **Where**: 
  - `backend/tests/cache_integration_test.rs` (uses `REDIS_URL`)
  - `backend/tests/repository_cache_test.rs` (uses `REDIS_URL` + `ARANGO_URL`)
- **Containers**: 
  - **During development**: Should use testcontainers (default `REDIS_URL`/`ARANGO_URL` point to testcontainers)
  - **For production validation**: Can use production containers if env vars are set
- **Strategy**: 
  - Development: Use testcontainers (unset `REDIS_URL`/`ARANGO_URL`)
  - Production validation: Use production containers (set env vars to production container URLs)

## Recommended Test Execution

### Development (Fast Feedback)
**When**: While coding, before committing, quick sanity checks
```bash
# Unit tests only (fast, no Docker needed)
./scripts/test-dev.sh

# Or manually:
cargo nextest run --workspace --lib
```

### Integration Testing (Isolated, testcontainers)
**When**: 
- Testing integration logic (database operations, API endpoints)
- Before committing code changes
- In CI/CD pipelines
- When you want isolated, fast integration tests

**Script**: `./scripts/test-integration.sh`
```bash
# Integration tests with testcontainers (ephemeral containers)
./scripts/test-integration.sh

# This runs:
# 1. testing/tests/*.rs (all integration tests using TestEnvironment)
# 2. backend/tests/database_integration_test.rs (database tests)

# Or manually:
cargo nextest run --package testing --test '*'
cargo nextest run --package backend --test 'database_integration_test'
```

### Production Validation (Full Stack, Production Containers)
**When**: 
- Before deploying to production
- Validating production Docker images
- End-to-end testing with real production setup

**Script**: `./scripts/run-tests-setup-prod.sh`
```bash
# Builds production images, starts production containers, runs all tests
./scripts/run-tests-setup-prod.sh

# This runs:
# 1. Unit tests (no containers)
# 2. E2E-style integration tests (against production backend)
# 3. E2E tests (Playwright against production frontend)
```

## Current Test Files

### Backend Tests (`backend/tests/`)
- `cache_integration_test.rs` - Uses `REDIS_URL` env var
- `contest_search_integration_test.rs` - Uses `BACKEND_URL` (E2E-style) - marked `#[ignore]`
- `database_integration_test.rs` - Uses `TestEnvironment` (disabled/outdated)
- `db_search_integration_test.rs` - Uses `BACKEND_URL` (E2E-style)
- `ratings_integration_test.rs` - Unit/integration tests (no containers needed)
- `repository_cache_test.rs` - Uses `REDIS_URL` + `ARANGO_URL` - marked `#[ignore]`
- `venue_update_integration_test.rs` - Uses `BACKEND_URL` (E2E-style)

### Testing Package (`testing/tests/`)
- All use `TestEnvironment` (testcontainers)
- Create their own ephemeral containers

## Recommendations

### For `run-tests-setup-prod.sh`:
1. **Unit tests** (`--lib`) ✓ - Already correct, no containers
2. **Integration tests using `BACKEND_URL`** ✓ - Already correct, uses production backend
3. **Integration tests using env vars** - Should be run SEPARATELY with testcontainers, or explicitly run against production containers if needed
4. **E2E tests** (Playwright) ✓ - Already correct, uses production containers

### For `test-dev.sh`:
- Currently runs unit tests only ✓
- Fast feedback during development

### For `test-integration.sh`:
- Runs integration tests with testcontainers ✓
- Isolated, ephemeral containers
- Fast, no need to start production containers
