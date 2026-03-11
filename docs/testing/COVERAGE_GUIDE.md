# Test Coverage Guide

This guide explains how to measure and improve test coverage in the STG RD project.

## Quick Start

### Generate Coverage Report

```bash
# Generate HTML coverage report
just coverage

# Or use the script directly
./scripts/coverage.sh
```

The HTML report will be available at `_build/coverage/html/index.html`.

### View Coverage Summary

```bash
# Quick summary (requires lcov)
just coverage-summary
```

## Understanding Coverage Metrics

### Coverage Types

1. **Line Coverage**: Percentage of executable lines covered by tests
2. **Function Coverage**: Percentage of functions called by tests
3. **Branch Coverage**: Percentage of conditional branches executed

### Target Coverage Goals

- **Unit Tests**: Aim for 80%+ line coverage
- **Integration Tests**: Aim for 60%+ line coverage of critical paths
- **E2E Tests**: Focus on user workflows, not coverage percentage

## Measuring Coverage

### Using cargo-llvm-cov

The project uses `cargo-llvm-cov` for source-based coverage:

```bash
# Install if not already installed
cargo install cargo-llvm-cov

# Generate LCOV report (for CI)
cargo llvm-cov nextest --workspace --lcov --output-path _build/lcov.info

# Generate HTML report (for viewing)
cargo llvm-cov nextest --workspace --html --output-dir _build/coverage/html
```

### Coverage Reports

1. **HTML Report**: Interactive browser-based report
   - Location: `_build/coverage/html/index.html`
   - Shows line-by-line coverage
   - Highlights uncovered code

2. **LCOV Report**: Machine-readable format
   - Location: `_build/coverage/lcov.info`
   - Used by CI/CD systems
   - Can be parsed by tools like `lcov`, `codecov`, etc.

### Using lcov for Summary

Install `lcov` for detailed coverage summaries:

```bash
# Ubuntu/Debian
sudo apt-get install lcov

# macOS
brew install lcov

# Then generate summary
lcov --summary _build/coverage/lcov.info
```

## Coverage by Test Type

### Unit Tests Coverage

Unit tests should cover:
- ✅ Business logic functions
- ✅ Data validation
- ✅ Error handling
- ✅ Edge cases
- ⚠️  Mock dependencies (not counted in coverage)

### Integration Tests Coverage

Integration tests cover:
- ✅ API endpoints
- ✅ Database operations
- ✅ External service interactions
- ✅ Authentication flows

### E2E Tests Coverage

E2E tests focus on:
- ✅ User workflows
- ✅ Frontend-backend integration
- ✅ Critical user paths

## Improving Coverage

### 1. Identify Uncovered Code

Open the HTML coverage report and look for:
- Red lines (uncovered)
- Yellow lines (partially covered)
- Files with low coverage percentage

### 2. Add Missing Tests

For each uncovered area:
- Determine if it needs testing
- Add unit tests for business logic
- Add integration tests for API/database code
- Add E2E tests for user workflows

### 3. Exclude Non-Testable Code

Some code shouldn't be counted:
- Main functions
- Test utilities
- Generated code
- Platform-specific code

Use `#[cfg(not(test))]` or coverage tool exclusions.

## CI/CD Integration

### GitHub Actions Example

```yaml
- name: Generate Coverage
  run: |
    cargo llvm-cov nextest --workspace --lcov --output-path coverage.lcov

- name: Upload Coverage
  uses: codecov/codecov-action@v3
  with:
    file: ./coverage.lcov
```

### Coverage Thresholds

Set minimum coverage thresholds in CI:

```bash
# Fail if coverage drops below threshold
lcov --summary coverage.lcov | grep -E "lines.*[0-9]+\.[0-9]+%" | \
  awk '{if ($2+0 < 80.0) exit 1}'
```

## Best Practices

1. **Don't Chase 100% Coverage**
   - Focus on critical paths
   - Test edge cases
   - Don't test trivial code

2. **Review Coverage Regularly**
   - Check coverage after major changes
   - Identify coverage gaps
   - Prioritize high-risk areas

3. **Use Coverage to Find Bugs**
   - Uncovered code may indicate:
     - Dead code (can be removed)
     - Missing error handling
     - Untested edge cases

4. **Combine with Other Metrics**
   - Coverage + mutation testing
   - Coverage + code review
   - Coverage + static analysis

## Troubleshooting

### Coverage Report Not Generating

```bash
# Ensure cargo-llvm-cov is installed
cargo install cargo-llvm-cov

# Check Rust toolchain
rustup component add llvm-tools-preview
```

### Low Coverage in Specific Areas

1. Check if code is testable
2. Consider refactoring for testability
3. Add integration tests if unit tests aren't sufficient

### Coverage Tool Errors

```bash
# Clean and rebuild
cargo clean
cargo llvm-cov nextest --workspace --html --output-dir _build/coverage/html
```

## Resources

- [cargo-llvm-cov Documentation](https://github.com/taiki-e/cargo-llvm-cov)
- [LLVM Source-Based Coverage](https://clang.llvm.org/docs/SourceBasedCodeCoverage.html)
- [lcov Documentation](https://github.com/linux-test-project/lcov)

