# Test Scripts Quick Reference

## Three Test Scripts

### 1. `./scripts/test-dev.sh` - Unit Tests (Fast)
**When to run**: While coding, before committing, quick sanity checks
- **Speed**: ⚡ Very fast (seconds)
- **Docker**: ❌ Not needed
- **What it runs**: Unit tests only (`--lib`)
- **Use case**: Quick feedback during development

```bash
./scripts/test-dev.sh
```

### 2. `./scripts/test-integration.sh` - Integration Tests (Testcontainers)
**When to run**: 
- Testing integration logic (database, API endpoints)
- Before committing code changes
- In CI/CD pipelines
- When you want isolated, fast integration tests

- **Speed**: 🚀 Fast (minutes)
- **Docker**: ✅ Required (creates ephemeral containers)
- **What it runs**: 
  - `testing/tests/*.rs` (all integration tests)
  - `backend/tests/database_integration_test.rs`
- **Use case**: Test integration logic with isolated containers

```bash
./scripts/test-integration.sh
```

### 3. `./scripts/run-tests-setup-prod.sh` - Production Validation (Full Stack)
**When to run**: 
- Before deploying to production
- Validating production Docker images
- End-to-end testing with real production setup

- **Speed**: 🐢 Slow (10-15 minutes)
- **Docker**: ✅ Required (builds and runs production containers)
- **What it runs**:
  - Unit tests
  - E2E-style integration tests (HTTP-based, use `BACKEND_URL`)
  - E2E tests (Playwright)
- **Use case**: Validate production deployment

```bash
./scripts/run-tests-setup-prod.sh
```

## Test Categories

| Test Type | Script | Containers | Speed |
|-----------|--------|-------------|-------|
| Unit tests | `test-dev.sh` | None | ⚡ Very fast |
| Integration (testcontainers) | `test-integration.sh` | Ephemeral | 🚀 Fast |
| E2E-style (HTTP) | `run-tests-setup-prod.sh` | Production | 🐢 Slow |
| E2E (Playwright) | `run-tests-setup-prod.sh` | Production | 🐢 Slow |

## Typical Workflow

### During Development
```bash
# Quick check while coding
./scripts/test-dev.sh

# Test integration logic
./scripts/test-integration.sh
```

### Before Committing
```bash
# Run both fast test suites
./scripts/test-dev.sh
./scripts/test-integration.sh
```

### Before Deploying
```bash
# Full production validation
./scripts/run-tests-setup-prod.sh
```

## What Each Script Does NOT Run

- **`test-dev.sh`**: Does NOT run integration tests or E2E tests
- **`test-integration.sh`**: Does NOT run E2E-style HTTP tests or Playwright tests
- **`run-tests-setup-prod.sh`**: Does NOT run testcontainers-based integration tests (those are in `test-integration.sh`)

## Quick Test Commands

```bash
# Unit tests only
cargo nextest run --workspace --lib

# Integration tests (testcontainers)
cargo nextest run --package testing --test '*'
cargo nextest run --package backend --test 'database_integration_test'

# E2E-style tests (requires BACKEND_URL)
export BACKEND_URL=http://localhost:50002
cargo nextest run --package backend --test 'db_search_integration_test'
```
