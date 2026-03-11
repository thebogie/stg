# Cargo Nextest Quick Reference

## Installation
```bash
cargo install cargo-nextest
# or
make install-nextest
```

## Basic Usage

### Unit Tests (Fast - Daily Development)
```bash
# Quick unit tests
make test

# Parallel execution for speed
make test-parallel

# Watch mode for development
make test-watch

# Specific test categories
make test-games
make test-venues
make test-players
make test-security
make test-performance
```

### Integration Tests (Slower - Before Releases)
```bash
# Integration tests only
make test-integration

# All tests (unit + integration)
make test-all
```

### Development Workflow
```bash
# During development (fast feedback)
make dev-test

# Watch mode during development
make dev-watch

# Debug specific test
make test-filter filter=test_create_game
```

## Nextest Commands

### Basic Commands
```bash
# Run all tests
cargo nextest run

# Run specific test file
cargo nextest run --test game_tests

# Run tests matching pattern
cargo nextest run test_create

# Exclude integration tests
cargo nextest run --exclude integration_tests
```

### Profiles
```bash
# Use specific profile
cargo nextest run --profile integration
cargo nextest run --profile ci
cargo nextest run --profile debug

# Available profiles:
# - default: Fast unit tests
# - integration: Slower, more resources
# - ci: CI/CD optimized
# - debug: Single-threaded debugging
```

### Parallelization
```bash
# Auto-detect threads
cargo nextest run --test-threads=auto

# Specific number of threads
cargo nextest run --test-threads=8

# Single thread (for debugging)
cargo nextest run --test-threads=1
```

### Watch Mode
```bash
# Watch for changes and re-run
cargo nextest run --watch

# Watch specific tests
cargo nextest run --watch --test game_tests
```

## Performance Comparison

| Command | Execution Time | Use Case |
|---------|----------------|----------|
| `cargo test` | ~60 seconds | Standard testing |
| `cargo nextest run` | ~20 seconds | Fast feedback |
| `cargo nextest run --test-threads=auto` | ~15 seconds | Maximum speed |
| `make test-parallel` | ~15 seconds | Recommended daily |

## Test Organization

### Unit Tests (Fast)
- **Location**: `backend/tests/*.rs` (except `integration_tests.rs`)
- **Execution**: `make test` or `cargo nextest run --exclude integration_tests`
- **Profile**: `default`
- **Frequency**: Every commit, during development

### Integration Tests (Slower)
- **Location**: `backend/tests/integration_tests.rs`
- **Execution**: `make test-integration`
- **Profile**: `integration`
- **Frequency**: Before releases, CI/CD

## Environment Setup

### For Unit Tests
```bash
# No setup needed - uses mocks
make test
```

### For Integration Tests
```bash
# Set required environment variables
export ARANGO_URL="http://localhost:8529"
export ARANGO_USERNAME="test"
export ARANGO_PASSWORD="test"
export REDIS_URL="redis://localhost:6379"

# Run integration tests
make test-integration
```

## Common Patterns

### Development Cycle
```bash
# 1. Write code
# 2. Run unit tests (fast feedback)
make test

# 3. Run specific tests if needed
make test-games

# 4. Watch mode during development
make dev-watch

# 5. Before commit - run all unit tests
make test-parallel
```

### Release Process
```bash
# 1. Unit tests (fast validation)
make test-parallel

# 2. Integration tests (service validation)
make test-integration

# 3. All tests (complete validation)
make test-all
```

### Debugging
```bash
# Debug mode (single thread, verbose)
make test-debug

# Debug specific test
make test-filter filter=test_create_game

# Debug with watch mode
cargo nextest run --profile debug --watch
```

## CI/CD Integration

### GitHub Actions Example
```yaml
- name: Unit Tests
  run: make test-ci

- name: Integration Tests
  run: make test-integration
  env:
    ARANGO_URL: ${{ secrets.ARANGO_URL }}
    ARANGO_USERNAME: ${{ secrets.ARANGO_USERNAME }}
    ARANGO_PASSWORD: ${{ secrets.ARANGO_PASSWORD }}
    REDIS_URL: ${{ secrets.REDIS_URL }}
```

## Troubleshooting

### Common Issues

#### Tests Running Slowly
```bash
# Use parallel execution
make test-parallel

# Check thread count
cargo nextest run --test-threads=auto --verbose
```

#### Integration Tests Failing
```bash
# Check environment variables
echo $ARANGO_URL
echo $REDIS_URL

# Run with debug profile
cargo nextest run --profile debug --test integration_tests -- --ignored
```

#### Watch Mode Not Working
```bash
# Ensure you're in the right directory
cd backend

# Use explicit watch command
cargo nextest run --watch --exclude integration_tests
```

### Performance Tips

1. **Use parallel execution** for unit tests
2. **Exclude integration tests** during development
3. **Use watch mode** for continuous feedback
4. **Profile tests** to identify slow ones
5. **Use specific test filters** when debugging

## Advanced Features

### Test Filtering
```bash
# Run tests matching pattern
cargo nextest run test_create

# Run tests in specific module
cargo nextest run --test game_tests

# Exclude specific tests
cargo nextest run --exclude integration_tests
```

### Output Control
```bash
# Verbose output
cargo nextest run --verbose

# Quiet output
cargo nextest run --quiet

# JSON output for CI
cargo nextest run --profile ci
```

### Test Statistics
```bash
# List all tests without running
cargo nextest run --no-run --list

# Show test statistics
make test-stats
``` 