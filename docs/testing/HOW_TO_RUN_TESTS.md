# How to Run All Tests

## Quick Answer

```bash
# Run ALL tests (recommended)
just test-all

# Or with coverage reports
just test-full
```

## What Gets Run

### `just test-all` (Recommended)

Runs all test suites in order:

1. **Backend Unit Tests** (`just test-backend`)
   - Fast unit tests (no Docker needed)
   - ~30 seconds
   - 514+ tests

2. **Frontend E2E Tests** (`just test-frontend-e2e`)
   - Full stack tests with Docker
   - ~5-10 minutes (first run), ~2-5 minutes (subsequent)
   - Tests complete user workflows

3. **Frontend WASM Unit Tests** (`just test-frontend-unit`)
   - Optional, may fail in WSL2
   - Gracefully skipped if geckodriver issues occur

### `just test-full` (With Reports)

Runs all tests AND generates reports:

1. JUnit XML (`just test-junit`)
2. Coverage reports (`just coverage`)
3. E2E tests (`just test-frontend-e2e`)

Reports are generated at:
- `_build/test-results.xml` (JUnit)
- `_build/coverage/html/index.html` (Coverage)
- `_build/playwright-report/index.html` (E2E)

## Individual Test Suites

### Backend Unit Tests
```bash
just test-backend
# Fast, no dependencies needed
```

### Integration Tests
```bash
just test-integration
# Uses testcontainers (Docker), tests API + database
```

### Frontend E2E Tests
```bash
just test-frontend-e2e
# Uses Docker, tests complete frontend application
```

### Frontend WASM Unit Tests (Optional)
```bash
just test-frontend-unit
# May fail in WSL2, gracefully skipped
```

## Test Execution Times

| Test Suite | Time | Dependencies |
|------------|------|--------------|
| Backend Unit | ~30s | None |
| Integration | ~2-5 min | Docker (testcontainers) |
| E2E | ~5-10 min (first) | Docker (full stack) |
| E2E | ~2-5 min (subsequent) | Docker (reused) |
| **test-all** | **~10-20 min** | Docker |

## Prerequisites

### For `just test-all`:

1. **Docker** (for E2E and integration tests)
   ```bash
   docker ps  # Should work
   ```

2. **Environment file** (for E2E tests)
   ```bash
   ls config/.env.development  # Should exist
   ```

3. **Testing tools** (one-time setup)
   ```bash
   just setup
   # Installs: cargo-nextest, cargo-llvm-cov, Playwright browsers
   ```

## Common Workflows

### Quick Feedback (During Development)
```bash
# Fast unit tests only
just test-backend
```

### Before Committing
```bash
# Run everything
just test-all
```

### Pre-Deployment
```bash
# Full suite with reports
just test-full
```

### Watch Mode (Auto-rerun on Changes)
```bash
# Backend tests only
just test-watch
```

## Troubleshooting

### E2E Tests Fail to Start

**Problem**: Docker containers don't start

**Solution**:
```bash
# Check Docker is running
docker ps

# Check environment file exists
ls config/.env.development

# Start containers manually to see errors
./scripts/start-e2e-docker.sh
```

### Integration Tests Fail

**Problem**: Can't connect to databases

**Solution**:
```bash
# Integration tests use testcontainers (auto-starts Docker containers)
# If they fail, check Docker is running:
docker ps

# Or start hybrid dev environment:
./scripts/setup-hybrid-dev.sh
```

### WASM Unit Tests Fail

**Problem**: Geckodriver errors

**Solution**: This is expected in WSL2. E2E tests provide better coverage anyway. The test suite gracefully skips them.

## Summary

**To run all tests:**
```bash
just test-all
```

**To run all tests with reports:**
```bash
just test-full
```

**Individual suites:**
- `just test-backend` - Backend unit tests
- `just test-integration` - API + database tests
- `just test-frontend-e2e` - Frontend E2E tests (Docker)
- `just test-frontend-unit` - WASM unit tests (optional)

