# Testing Directory

This directory contains all testing infrastructure for the project, organized by testing type:

## Structure

```
testing/
├── Cargo.toml          # Rust testing crate configuration
├── src/                 # Rust integration test utilities (testcontainers, etc.)
├── tests/               # Rust integration tests
└── e2e/                 # Playwright E2E tests (TypeScript/JavaScript)
```

## Rust Integration Tests (`src/` and `tests/`)

The Rust testing crate provides:
- `TestEnvironment` - Ephemeral Docker containers for ArangoDB and Redis
- Integration test utilities using `testcontainers-rs`
- Example tests demonstrating the testing infrastructure

**Run with:**
```bash
cargo nextest run --package testing
# or
just test-integration
```

## E2E Tests (`e2e/`)

Playwright tests for frontend E2E and visual regression testing:
- Tests the actual Yew WASM frontend in a headless browser
- Visual regression testing via screenshots
- Cross-browser testing (Chrome, Firefox, Safari)

**Run with:**
```bash
npx playwright test
# or
just test-frontend-e2e
```

## Why Both?

- **Rust tests** (`src/`, `tests/`) - Test backend integration, database operations, API endpoints
- **E2E tests** (`e2e/`) - Test the complete frontend application in a real browser

Both are essential for comprehensive testing coverage.



