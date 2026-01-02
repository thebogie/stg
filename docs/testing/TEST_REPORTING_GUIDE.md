# Test Reporting Guide

This document explains how test results are reported and formatted in the STG RD project.

## Quick Summary

**Test Run Results**: 519 tests run, 517 passed, 2 failed, 0 skipped

## Reporting Mechanisms

### 1. Console Output (Real-time)

**Command**: `cargo nextest run --workspace --lib --tests`

**Output Format**:
```
Summary [  15.817s] 519 tests run: 517 passed, 2 failed, 0 skipped
```

**Features**:
- ✅ Real-time test execution status
- ✅ Color-coded output (green for pass, red for fail)
- ✅ Test timing information
- ✅ Failure details with stack traces
- ✅ Parallel execution status

**Example Output**:
```
    PASS [   0.123s] backend::player::controller_tests test_login_handler_success
    PASS [   0.145s] backend::game::controller_tests test_get_game_handler
    FAIL [   0.234s] backend::player::controller_tests test_player_logout
```

---

### 2. JUnit XML (CI/CD Integration)

**Command**: `cargo nextest run --workspace --lib --tests --junit-xml test-results.xml`

**Output File**: `test-results.xml`

**Format**: Standard JUnit XML format

**Features**:
- ✅ Machine-readable format
- ✅ Compatible with CI/CD systems (GitHub Actions, GitLab CI, Jenkins)
- ✅ Test case details (name, duration, status)
- ✅ Failure messages and stack traces
- ✅ Test suite organization

**Configuration**: `.nextest.toml`
```toml
[profile.default]
junit = { path = "test-results.xml" }
```

**Usage in CI/CD**:
```yaml
# GitHub Actions example
- name: Run tests
  run: cargo nextest run --workspace --lib --tests --junit-xml test-results.xml

- name: Upload test results
  uses: actions/upload-artifact@v3
  with:
    name: test-results
    path: test-results.xml
```

**JUnit XML Structure**:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<testsuites>
  <testsuite name="backend" tests="465" failures="2" time="15.817">
    <testcase name="test_login_handler_success" classname="backend::player::controller_tests" time="0.123"/>
    <testcase name="test_player_logout" classname="backend::player::controller_tests" time="0.234">
      <failure message="Invalid or expired session">...</failure>
    </testcase>
  </testsuite>
</testsuites>
```

---

### 3. Code Coverage Reports

#### HTML Coverage Report

**Command**: `cargo llvm-cov nextest --workspace --html --output-dir coverage/html`

**Output**: `coverage/html/index.html`

**Features**:
- ✅ Interactive HTML report
- ✅ Line-by-line coverage visualization
- ✅ Coverage percentages by file/module
- ✅ Click-through navigation
- ✅ Color-coded coverage (green = covered, red = not covered)

**Usage**:
```bash
just coverage
# Opens: coverage/html/index.html
```

#### LCOV Coverage Report (CI/CD)

**Command**: `cargo llvm-cov nextest --workspace --lcov --output-path lcov.info`

**Output**: `lcov.info`

**Features**:
- ✅ Standard LCOV format
- ✅ Compatible with codecov, coveralls, etc.
- ✅ Machine-readable
- ✅ Line-by-line coverage data

**Usage in CI/CD**:
```yaml
- name: Generate coverage
  run: cargo llvm-cov nextest --workspace --lcov --output-path lcov.info

- name: Upload to codecov
  uses: codecov/codecov-action@v3
  with:
    file: ./lcov.info
```

---

### 4. Combined Reports (Justfile)

**Command**: `just test-full`

**What it does**:
1. Runs all tests with JUnit XML output
2. Generates HTML coverage report
3. Runs E2E tests
4. Displays summary

**Output**:
```
✅ Full test suite completed!
📊 Reports:
  - JUnit XML: test-results.xml
  - Coverage: coverage/html/index.html
```

---

## Current Test Results Summary

### Unit Tests (Backend)

**Command**: `cargo nextest run --workspace --lib --tests`

**Results**:
- **Total Tests**: 519
- **Passed**: 517 ✅
- **Failed**: 2 ❌
- **Skipped**: 0
- **Duration**: ~15.8 seconds

### Failed Tests

1. **`test_player_logout`** (testing::api_tests)
   - **Error**: "Invalid or expired session"
   - **Location**: `testing/tests/api_tests.rs`
   - **Type**: Integration test (API endpoint)

2. **`test_get_current_player_unauthorized`** (testing::api_tests)
   - **Error**: "Authentication required"
   - **Location**: `testing/tests/api_tests.rs`
   - **Type**: Integration test (API endpoint)

**Note**: These are integration tests, not pure unit tests. They test API endpoints with real database connections.

---

## Reporting Workflow

### Development Workflow

```bash
# Quick feedback during development
cargo nextest run --lib

# With detailed output
cargo nextest run --workspace --lib --tests --nocapture

# Watch mode (auto-rerun on changes)
just test-watch
```

### Pre-Commit Workflow

```bash
# Run all unit tests
just test-backend

# Check for failures
if [ $? -ne 0 ]; then
  echo "Tests failed! Fix before committing."
  exit 1
fi
```

### CI/CD Workflow

```bash
# Generate all reports
just test-full

# Or step by step:
just test-junit        # JUnit XML for CI
just coverage-lcov     # Coverage for codecov
just test-frontend-e2e # E2E tests
```

---

## Report Formats Comparison

| Format | Use Case | Output | Readable By |
|--------|----------|--------|-------------|
| **Console** | Development | Terminal | Humans |
| **JUnit XML** | CI/CD | `test-results.xml` | CI systems |
| **HTML Coverage** | Review | `coverage/html/` | Humans (browser) |
| **LCOV** | CI/CD | `lcov.info` | Coverage services |

---

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Install nextest
        run: cargo install cargo-nextest
      
      - name: Run tests
        run: cargo nextest run --workspace --lib --tests --junit-xml test-results.xml
      
      - name: Generate coverage
        run: cargo llvm-cov nextest --workspace --lcov --output-path lcov.info
      
      - name: Upload test results
        uses: actions/upload-artifact@v3
        with:
          name: test-results
          path: test-results.xml
      
      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          file: ./lcov.info
```

### GitLab CI Example

```yaml
test:
  stage: test
  script:
    - cargo nextest run --workspace --lib --tests --junit-xml test-results.xml
    - cargo llvm-cov nextest --workspace --lcov --output-path lcov.info
  artifacts:
    reports:
      junit: test-results.xml
    paths:
      - lcov.info
```

---

## Viewing Reports

### Console Output
```bash
cargo nextest run --workspace --lib --tests
# View directly in terminal
```

### JUnit XML
```bash
# View with XML tools
cat test-results.xml | xmllint --format -

# Or use CI/CD UI (GitHub Actions, GitLab, etc.)
```

### HTML Coverage
```bash
just coverage
# Then open in browser:
# file:///path/to/project/coverage/html/index.html

# Or serve locally:
python3 -m http.server 8000 -d coverage/html
# Open: http://localhost:8000
```

---

## Test Result Interpretation

### Success Criteria

✅ **All tests pass**: `519 tests run: 519 passed, 0 failed`

⚠️ **Some failures**: `519 tests run: 517 passed, 2 failed`
- Review failure details
- Fix failing tests
- Don't merge until all pass

❌ **Many failures**: `519 tests run: 400 passed, 119 failed`
- Likely breaking change
- Review recent changes
- May need rollback

### Performance Metrics

- **Unit tests**: Should complete in < 30 seconds
- **Integration tests**: Should complete in < 5 minutes
- **E2E tests**: Should complete in < 30 minutes

---

## Troubleshooting

### Tests Not Running

```bash
# Check nextest is installed
cargo nextest --version

# Install if missing
cargo install cargo-nextest
```

### JUnit XML Not Generated

```bash
# Check .nextest.toml configuration
cat .nextest.toml

# Run with explicit flag
cargo nextest run --workspace --lib --tests --junit-xml test-results.xml
```

### Coverage Not Generating

```bash
# Check llvm-cov is installed
cargo llvm-cov --version

# Install if missing
cargo install cargo-llvm-cov
```

---

## Best Practices

1. **Always check test results before committing**
2. **Use JUnit XML in CI/CD for automated reporting**
3. **Generate coverage reports regularly to track trends**
4. **Review HTML coverage reports to identify untested code**
5. **Fix failing tests immediately - don't let them accumulate**

---

## Summary

**Current Test Status**: 517/519 passing (99.6% pass rate)

**Reporting Available**:
- ✅ Console output (real-time)
- ✅ JUnit XML (CI/CD)
- ✅ HTML coverage (review)
- ✅ LCOV coverage (CI/CD)

**Next Steps**:
1. Fix 2 failing integration tests
2. Review coverage report to identify gaps
3. Set up CI/CD to automatically generate reports

