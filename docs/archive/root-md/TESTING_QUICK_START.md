# Testing Quick Start Guide

**Run all tests after your bug fix!**

## 🚀 Quick Commands

### Run Everything (Recommended)
```bash
# Run all tests: unit + integration + E2E
just test-all
```

### Individual Test Suites

#### 1. Unit Tests (Backend)
```bash
# Fast unit tests - no Docker needed
just test-backend

# Or directly:
cargo nextest run --workspace --lib --tests
```

#### 2. Integration Tests
```bash
# Integration tests (needs Docker services running)
just test-integration

# Or directly:
cargo nextest run --package testing
```

**Note:** Integration tests need Redis/ArangoDB. If you have hybrid dev running, they'll connect automatically.

#### 3. E2E Tests (Frontend) - **PRIMARY FRONTEND TESTING METHOD**
```bash
# Playwright E2E tests (recommended for frontend testing)
just test-frontend-e2e

# Or use the alias:
just test-frontend

# Or directly:
npx playwright test
```

**Note:** E2E tests will start the frontend automatically. You may need the backend running if tests make API calls.

**Note:** WASM unit tests are not supported in WSL2/headless environments. E2E tests provide comprehensive frontend coverage and work reliably in all environments.

## 📊 With Coverage Reports

### Full Test Suite with Reports
```bash
# Run all tests + generate coverage + JUnit XML
just test-full
```

This will:
- ✅ Run all unit tests
- ✅ Generate coverage report at `_build/coverage/html/index.html`
- ✅ Generate JUnit XML at `_build/test-results.xml`
- ✅ Run E2E tests

### Just Coverage
```bash
# Generate HTML coverage report
just coverage

# Or LCOV format (for CI)
just coverage-lcov
```

## 🔧 Prerequisites

### Install Testing Tools (One Time)
```bash
# Install cargo-nextest and coverage tools
cargo install cargo-nextest cargo-llvm-cov

# Install Playwright browsers
npx playwright install --with-deps
```

Or use the setup command:
```bash
just setup
```

### Environment Setup

**For Integration Tests:**
- Start hybrid dev environment (ArangoDB + Redis):
  ```bash
  ./scripts/setup-hybrid-dev.sh
  ```
- Or ensure Docker services are running on expected ports

**For E2E Tests:**
- Frontend will start automatically
- Backend should be running (or tests will fail)

## 📋 Test Results

After running tests, check:

- **JUnit XML**: `_build/test-results.xml`
- **Coverage HTML**: `_build/coverage/html/index.html`
- **Playwright Report**: `_build/playwright-report/index.html`
- **E2E Results XML**: `_build/test-results/e2e-results.xml`

## 🎯 Recommended Workflow

After making a bug fix:

```bash
# 1. Run unit tests first (fastest)
just test-backend

# 2. If unit tests pass, run integration tests
just test-integration

# 3. If integration tests pass, run E2E tests
just test-frontend-e2e

# 4. Or just run everything at once
just test-all
```

## 🔍 Running Specific Tests

### Run Tests Matching a Pattern
```bash
# Run tests matching "bgg" pattern
just test-pattern bgg

# Or directly:
cargo nextest run --workspace --lib --tests -- bgg
```

### Run Tests in Watch Mode
```bash
# Auto-rerun tests on file changes
just test-watch
```

## 📝 Test Structure

- **Unit Tests**: `backend/src/**/*_tests.rs` and `backend/src/**/tests/`
- **Integration Tests**: `testing/tests/`
- **E2E Tests**: `testing/e2e/`

## ⚠️ Troubleshooting

**Tests fail to connect to services:**
- Make sure hybrid dev is running: `./scripts/setup-hybrid-dev.sh`
- Check ports match your `.env.development` file

**E2E tests fail:**
- Ensure backend is running on the expected port
- Check `FRONTEND_URL` in `playwright.config.ts` matches your setup

**Coverage not generating:**
- Make sure `cargo-llvm-cov` is installed: `cargo install cargo-llvm-cov`

## 📚 More Information

- **Full Testing Guide**: `docs/testing/TESTING_SETUP.md`
- **Testing Architecture**: `docs/testing/TESTING_ARCHITECTURE.md`
- **Test Status**: `docs/testing/TESTING_STATUS.md`

