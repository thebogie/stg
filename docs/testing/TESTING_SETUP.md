# Testing Setup Guide

This guide will help you get unit and integration tests running.

## Prerequisites Check

### 1. Install Required Tools

```bash
# Install cargo-nextest and cargo-llvm-cov
cargo install cargo-nextest cargo-llvm-cov

# Or use the setup command
just setup
```

### 2. Verify Docker is Running

Integration tests use testcontainers-rs, which requires Docker:

```bash
docker ps
# Should show running containers or at least not error
```

If Docker is not running, start it:
- Linux: `sudo systemctl start docker`
- macOS: Start Docker Desktop
- Windows: Start Docker Desktop

## Step 1: Run Unit Tests (No Docker Required)

Unit tests don't require Docker and should work immediately:

```bash
# Run all unit tests
cargo test --lib

# Or use nextest (faster, better output)
cargo nextest run --lib

# Or use the Justfile
just test-backend
```

**Expected output:** Tests should compile and run. You may see some tests pass/fail depending on your setup.

## Step 2: Set Up Integration Tests

Integration tests need Docker containers. Currently, the `TestEnvironment` uses environment variables as a fallback.

### Option A: Use Existing Docker Containers (Quick Start)

If you have ArangoDB and Redis running via docker-compose:

```bash
# Start your services
docker-compose up -d

# Set environment variables
export ARANGO_URL="http://localhost:8529"
export REDIS_URL="redis://localhost:6379/"

# Run integration tests
cargo nextest run --package testing
```

### Option B: Use Testcontainers (Recommended)

The testcontainers implementation needs to be completed. Currently it falls back to environment variables.

**To enable true testcontainers:**

1. Check your testcontainers-rs version:
   ```bash
   cargo tree | grep testcontainers
   ```

2. Update `testing/src/lib.rs` with the correct API for your version. The current implementation is a placeholder.

3. Once testcontainers is working, tests will automatically spin up containers.

## Step 3: Run All Tests

```bash
# Run all tests with nextest
cargo nextest run --workspace

# Or use the Justfile
just test-all
```

## Step 4: Run Specific Test Suites

```bash
# Backend unit tests only
just test-backend

# Integration tests only
just test-integration

# Frontend unit tests (WASM)
just test-frontend-unit

# Frontend E2E tests (Playwright)
just test-frontend-e2e
```

## Troubleshooting

### Tests Fail to Connect to Database

**Problem:** Integration tests can't connect to ArangoDB/Redis

**Solution:**
1. Ensure services are running: `docker-compose ps`
2. Check environment variables: `echo $ARANGO_URL`
3. Verify ports are correct (default: ArangoDB 8529, Redis 6379)

### Testcontainers Not Working

**Problem:** `testcontainers-rs` errors or containers don't start

**Solution:**
1. Verify Docker is running: `docker ps`
2. Check Docker permissions: `docker run hello-world`
3. The current implementation uses env vars as fallback - this is fine for now
4. Update `testing/src/lib.rs` when ready to use real testcontainers

### Nextest Not Found

**Problem:** `cargo nextest: command not found`

**Solution:**
```bash
cargo install cargo-nextest
```

### Playwright Tests Fail

**Problem:** E2E tests can't find browser or frontend

**Solution:**
```bash
# Install Playwright browsers
npx playwright install --with-deps

# Ensure frontend builds
cd frontend && wasm-pack build
```

## Next Steps

1. ✅ Run unit tests - should work immediately
2. ✅ Run integration tests with docker-compose - set env vars and run
3. 🔄 Complete testcontainers implementation - update `testing/src/lib.rs`
4. 🔄 Add more integration tests - expand `testing/tests/`
5. 🔄 Set up CI/CD - add GitHub Actions workflow

## Quick Test Commands Reference

```bash
# Unit tests (fast, no Docker)
cargo test --lib
cargo nextest run --lib

# Integration tests (needs Docker/env vars)
cargo nextest run --package testing

# All tests
cargo nextest run --workspace
just test-all

# With coverage
just coverage

# Watch mode (auto-rerun on changes)
just test-watch
```

