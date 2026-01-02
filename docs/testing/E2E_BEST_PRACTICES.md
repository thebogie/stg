# E2E Testing Best Practices

## Industry Standard Approach

**Key Principle**: Build images separately, don't build during test runs.

This approach is used by:
- GitHub Actions
- GitLab CI
- CircleCI
- Most production CI/CD pipelines

## Why This Approach?

1. **Faster Test Runs**: No build time during tests (tests run in seconds, not minutes)
2. **More Reliable**: Build issues don't affect test execution
3. **Better Debugging**: Build failures are separate from test failures
4. **CI/CD Friendly**: Matches how production pipelines work
5. **Reusable Images**: Build once, test many times

## Workflow

### Step 1: Build Images (One Time or When Code Changes)

```bash
# Build all E2E images
just test-e2e-build-images

# This builds:
# - Frontend image
# - Backend image
# - Uses production config (docker-compose.prod.yml)
# - Isolated network (e2e_testing)
```

**Time**: 5-10 minutes (first time), 2-5 minutes (incremental)

### Step 2: Run Tests (Fast!)

```bash
# Run E2E tests (no build, just start containers)
just test-frontend-e2e

# This:
# - Starts pre-built containers
# - Runs Playwright tests
# - Stops containers when done
```

**Time**: 30 seconds - 2 minutes (depending on test count)

### Alternative: Build and Test in One Command

```bash
# If you want to build and test together (slower)
just test-frontend-e2e-full
# Or:
BUILD_IMAGES=1 just test-frontend-e2e
```

## Architecture

### Isolated Environment

- **Network**: `e2e_testing` (separate from `hybrid_dev_env`)
- **Project**: `e2e_testing` (isolated containers)
- **Config**: Production-like (`docker-compose.prod.yml`)
- **Data**: Separate volumes (`arango_data_e2e`, etc.)

### Configuration Stack

1. `docker-compose.yaml` - Base services
2. `docker-compose.prod.yml` - Production settings
3. `docker-compose.e2e.yml` - E2E overrides (ports only)

## Comparison with Other Approaches

### ❌ Building During Tests (Old Approach)
- Slow: 5-10 minutes per test run
- Unreliable: Build failures break tests
- Hard to debug: Build + test errors mixed

### ✅ Pre-Building Images (Current Approach)
- Fast: 30 seconds - 2 minutes per test run
- Reliable: Build issues separate from test issues
- Easy to debug: Clear separation of concerns

## CI/CD Integration

In CI/CD, this becomes:

```yaml
# Build stage
- name: Build E2E images
  run: just test-e2e-build-images

# Test stage
- name: Run E2E tests
  run: just test-frontend-e2e
```

This allows:
- Parallel execution
- Caching of build stage
- Faster feedback on test failures

## Troubleshooting

### Images Not Found

```bash
# Rebuild images
just test-e2e-build-images
```

### Containers Won't Start

```bash
# Check if network exists
docker network ls | grep e2e_testing

# Clean up and restart
just test-frontend-e2e-stop
just test-frontend-e2e
```

### Build Failures

Build failures are now separate from test failures:
1. Fix build issues in `just test-e2e-build-images`
2. Once images build, tests run independently

## Best Practices

1. **Build images when code changes** (not on every test run)
2. **Reuse images across test runs** (faster iteration)
3. **Build in CI/CD separately** (better caching)
4. **Use production config** (accurate test coverage)
5. **Isolate test environment** (separate network/volumes)

## References

- [Docker Compose Best Practices](https://docs.docker.com/compose/production/)
- [CI/CD Testing Patterns](https://www.atlassian.com/continuous-delivery/software-testing/types-of-software-testing)
- [E2E Testing with Docker](https://www.testcontainers.org/)

