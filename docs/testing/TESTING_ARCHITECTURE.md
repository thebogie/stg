# Rust 2026 Testing Architecture

This document describes the modern testing architecture implemented for STG RD, following industry best practices for late 2025/2026.

## Overview

The testing stack has been modernized to use:
- **cargo-nextest**: Fast, parallel test runner with better isolation
- **testcontainers-rs**: Ephemeral Docker containers for integration tests
- **cargo-llvm-cov**: LLVM source-based coverage with HTML reports
- **Playwright**: E2E testing for Yew frontend with visual regression
- **rstest**: Table-driven testing for efficient test writing

## Quick Start

### Setup

```bash
# Install all required tools
just setup

# Or manually:
cargo install cargo-nextest cargo-llvm-cov
npm install
npx playwright install --with-deps
```

### Running Tests

```bash
# Run all tests
just test-all

# Run backend tests only
just test-backend

# Run integration tests (with testcontainers)
just test-integration

# Run frontend unit tests
just test-frontend-unit

# Run frontend E2E tests
just test-frontend-e2e

# Generate coverage report
just coverage

# Full test suite with reports
just test-full
```

## Architecture Components

### 1. Test Runner: cargo-nextest

**Why**: Significantly faster than built-in test runner, isolates tests per process (reducing flaky state), produces structured data for reports.

**Configuration**: `.nextest.toml`

**Usage**:
```bash
cargo nextest run --workspace
cargo nextest run --package backend
```

### 2. Integration Orchestration: testcontainers-rs

**Why**: Instead of manually running `docker compose up`, your Rust code spins up ArangoDB and Redis containers during tests. This ensures a clean state for every test run.

**Location**: `testing/` crate

**Usage**:
```rust
use testing::TestEnvironment;

#[tokio::test]
async fn test_with_db() {
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;
    
    // Use env.arangodb_url() and env.redis_url()
    // Containers are automatically cleaned up after test
}
```

### 3. Coverage & Visuals: cargo-llvm-cov

**Why**: Uses LLVM source-based coverage (standard in modern Rust) to generate interactive HTML reports showing exactly which lines of code were executed.

**Usage**:
```bash
# Generate HTML report
just coverage

# Generate LCOV for CI
just coverage-lcov
```

Reports are generated in `coverage/html/index.html`.

### 4. Frontend Testing: Playwright

**Why**: For Yew, unit tests use wasm-bindgen-test, but for "real" testing, the industry standard is Playwright. It renders the actual WASM in a headless browser and screenshots it.

**Configuration**: `playwright.config.ts`

**Usage**:
```bash
# Run E2E tests
just test-frontend-e2e

# Run with UI
npm run test:e2e:ui

# Run in headed mode (see browser)
npm run test:e2e:headed
```

**Location**: `e2e/` directory

### 5. Testing Logic: rstest

**Why**: Allows you to write "table-driven" tests (running the same logic against 50 different data inputs) with clean syntax.

**Example**:
```rust
use rstest::rstest;

#[rstest]
#[case("test_db_1")]
#[case("test_db_2")]
#[case("test_db_3")]
async fn test_multiple_databases(#[case] db_name: &str) {
    // Test runs 3 times with different inputs
}
```

## Real Prod Data Strategy

### Safe Approach: Sanitized Snapshot Seeding

**DO NOT** connect tests directly to production. That is a catastrophic risk (accidental writes/deletes).

### Implementation

1. **Export Production Data**:
   ```bash
   just export-prod-data
   # Or manually:
   mkdir -p _build/dumps
   cargo run --package scripts --bin export_prod_data \
     --arango-url http://prod-server:8529 \
     --database stg_rd \
     --username root \
     --password <password> \
     --output _build/dumps/dump.json
   ```

2. **Sanitize PII**:
   ```bash
   just sanitize-data _build/dumps/dump.json _build/dumps/dump.sanitized.json.gz
   # Or manually:
   cargo run --package scripts --bin sanitize_data \
     --input _build/dumps/dump.json \
     --output _build/dumps/dump.sanitized.json.gz
   ```

3. **Use in Tests**:
   ```rust
   let env = TestEnvironmentBuilder::new()
       .with_data_dump("_build/dumps/dump.sanitized.json.gz")
       .build()
       .await?;
   ```

### What Gets Sanitized

- **Email addresses**: Replaced with `test_user_{idx}@example.com`
- **First names**: Replaced with `TestUser{idx}`
- **Handles/Usernames**: Replaced with `test_user_{idx}`
- **Password hashes**: Replaced with test hash
- **Addresses**: Replaced with generic test address
- **Other PII**: Removed (phone, SSN, credit cards, IPs, etc.)

## Visual Reporting Pipeline

The `Justfile` automates the complete testing and reporting pipeline:

```bash
just test-full
```

This runs:
1. `cargo nextest run` → Generates JUnit XML (`test-results.xml`)
2. `cargo llvm-cov` → Generates HTML coverage report (`coverage/html/`)
3. `playwright test` → Generates screenshots and HTML report (`playwright-report/`)

## File Structure

```
.
├── Justfile                    # Modern Makefile replacement
├── .nextest.toml               # Nextest configuration
├── playwright.config.ts        # Playwright configuration
├── package.json                # Node deps for Playwright
├── testing/                      # All testing infrastructure
│   ├── Cargo.toml             # Rust testing crate
│   ├── src/lib.rs             # TestEnvironment with testcontainers
│   ├── tests/                 # Rust integration tests
│   │   └── integration_test.rs
│   └── e2e/                   # Playwright E2E tests
│       └── example.spec.ts
├── scripts/                    # Data export/sanitization scripts
│   ├── export_prod_data.rs
│   └── sanitize_data.rs
└── coverage/                   # Generated coverage reports
    └── html/
```

## CI/CD Integration

### GitHub Actions Example

```yaml
- name: Run tests
  run: |
    cargo nextest run --workspace --junit-xml test-results.xml
    cargo llvm-cov nextest --workspace --lcov --output-path lcov.info
    npx playwright test

- name: Upload coverage
  uses: codecov/codecov-action@v3
  with:
    file: ./lcov.info
```

## Best Practices

1. **Always use testcontainers for integration tests** - Never assume a database is running
2. **Sanitize all production data** - Never use real PII in tests
3. **Use rstest for parameterized tests** - Reduces code duplication
4. **Generate coverage reports regularly** - Track coverage trends
5. **Use Playwright for E2E** - Unit tests aren't enough for frontend
6. **Run tests in parallel** - Nextest does this by default
7. **Use visual regression testing** - Catch UI bugs early

## Troubleshooting

### Testcontainers not starting

- Ensure Docker is running: `docker ps`
- Check Docker permissions
- Increase timeout in test code if needed

### Coverage not generating

- Ensure `cargo-llvm-cov` is installed: `cargo install cargo-llvm-cov`
- Check that tests are actually running
- Verify LLVM tools are available

### Playwright tests failing

- Ensure browsers are installed: `npx playwright install`
- Check that frontend server is running
- Verify base URL in `playwright.config.ts`

## Migration from Old Testing

The old testing setup used:
- `cargo test` (replaced by `cargo nextest`)
- Manual Docker Compose (replaced by `testcontainers-rs`)
- `tarpaulin` (replaced by `cargo-llvm-cov`)
- `wasm-pack test` only (still used for unit tests, but Playwright for E2E)

See `TESTING.md` for the old documentation (kept for reference).

## References

- [cargo-nextest](https://nexte.st/)
- [testcontainers-rs](https://github.com/testcontainers/testcontainers-rs)
- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)
- [Playwright](https://playwright.dev/)
- [rstest](https://github.com/la10736/rstest)

