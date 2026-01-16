# Run All Tests - Single Command

## Quick Start

Run **ALL** tests with one command:

```bash
./scripts/run-all-tests.sh
```

Or using Just:

```bash
just test-everything
```

## What It Does

This script runs **EVERY** test in the project:

1. ✅ **Backend Unit Tests** - All library unit tests
2. ✅ **Backend Integration Tests** - Tests in `backend/tests/` directory (including cache tests)
3. ✅ **Testing Package Integration Tests** - Full 3-tier integration test suite
4. ✅ **Cache Integration Tests** - Explicit cache testing (if Redis available)
5. ✅ **Frontend E2E Tests** - Playwright tests (optional)

## Automatic Service Management

The script **automatically**:
- ✅ Detects if Redis/ArangoDB are running
- ✅ Starts services via `setup-hybrid-dev.sh` if needed
- ✅ Handles service cleanup

## Options

```bash
# Skip E2E tests (faster)
./scripts/run-all-tests.sh --skip-e2e

# Don't start services (assume they're already running)
./scripts/run-all-tests.sh --skip-services

# Combine options
./scripts/run-all-tests.sh --skip-e2e --skip-services
```

## Test Coverage

### What Gets Tested

- ✅ All backend unit tests (`--lib`)
- ✅ All backend integration tests (`backend/tests/`)
- ✅ Cache integration tests (`cache_integration_test.rs`)
- ✅ Repository cache tests (`repository_cache_test.rs`) - if Redis + ArangoDB available
- ✅ Testing package integration tests (3-tier strategy)
- ✅ Frontend E2E tests (Playwright)

### Cache Tests

Cache tests require Redis. The script:
- Checks if Redis is running
- Runs cache tests if available
- Warns if Redis is not available (doesn't fail)

Repository cache tests also require ArangoDB and are marked `#[ignore]` by default.

## Example Output

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
▶ 🧪 Running ALL Tests
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
▶ Setting Up Test Services
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ Redis is already running on port 6379
✅ ArangoDB is already running on port 8529

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
▶ STEP 1: Backend Unit Tests (Library)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ Backend unit tests passed

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
▶ STEP 2: Backend Integration Tests
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ Backend integration tests completed

... (continues through all test suites)
```

## Comparison with Other Scripts

| Script | What It Does |
|--------|-------------|
| `./scripts/run-all-tests.sh` | **Runs EVERYTHING** - all tests, auto-starts services |
| `./scripts/test-and-push-prod.sh` | Runs tests + builds + pushes Docker images |
| `just test-all` | Runs unit + integration + E2E (but doesn't auto-start services) |
| `./scripts/run_tests.sh` | Old script - basic unit tests only |

## Tips

1. **First Time**: Let the script start services automatically
2. **Already Running Services**: Use `--skip-services` for faster startup
3. **Quick Iteration**: Use `--skip-e2e` to skip slow E2E tests
4. **CI/CD**: Script fails fast if any test suite fails

## Troubleshooting

### Services Won't Start

If services fail to start:
```bash
# Start manually first
./scripts/setup-hybrid-dev.sh

# Then run tests
./scripts/run-all-tests.sh --skip-services
```

### Cache Tests Skipped

If you see "Redis not available":
- Start Redis: `docker run -d -p 6379:6379 redis:7-alpine`
- Or use: `./scripts/setup-hybrid-dev.sh`

### Some Tests Ignored

Repository cache tests are marked `#[ignore]` by default. They run if Redis + ArangoDB are available and tests are run with `--ignored` flag (which the script does automatically).
