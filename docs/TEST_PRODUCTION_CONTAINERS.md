# Testing Production Containers

## Quick Answer

**To test the EXACT containers you'll deploy to production:**

```bash
./scripts/run-all-tests.sh --prod-containers
```

Or using Just:

```bash
just test-everything-prod
```

## What This Does

1. ✅ **Builds production Docker images** (release mode, optimized)
2. ✅ **Starts production containers** locally
3. ✅ **Runs ALL tests** against those production containers:
   - Backend unit tests (code logic)
   - Backend integration tests (API endpoints)
   - Cache integration tests (Redis from containers)
   - Testing package integration tests (full suite)
   - Frontend E2E tests (against production frontend)
4. ✅ **Uses production environment** configuration
5. ✅ **Cleans up** containers when done

## Why This Matters

### Without `--prod-containers`:
- Tests run against code on your host machine
- Different build settings, optimizations
- May miss container-specific issues
- **Risk**: Code works in tests but fails in production containers

### With `--prod-containers`:
- Tests run against **EXACT** Docker containers
- Same build settings as production
- Same optimizations as production
- **Confidence**: If tests pass, production will work

## Comparison

| Command | What It Tests | Production Parity |
|---------|---------------|-------------------|
| `./scripts/run-all-tests.sh` | Host code directly | ⚠️ Close but not identical |
| `./scripts/run-all-tests.sh --prod-containers` | Production Docker containers | ✅ **Exact match** |
| `./scripts/test-and-push-prod.sh` | Tests → Builds → Pushes | ✅ Tests host code, then builds |

## Workflow Recommendation

### For Development (Fast Iteration)
```bash
# Quick tests while developing
./scripts/run-all-tests.sh --skip-e2e
```

### For Pre-Deployment (Production Confidence)
```bash
# Test exact production containers
./scripts/run-all-tests.sh --prod-containers
```

### For Complete Deployment Pipeline
```bash
# Tests → Build → Push (like test-and-push-prod.sh)
./scripts/test-and-push-prod.sh
```

## What Gets Tested with `--prod-containers`

### ✅ Backend Unit Tests
- Run on host (test code logic)
- Same code that's in containers

### ✅ Backend Integration Tests
- Connect to production backend container
- Test actual API endpoints
- Use Redis/ArangoDB from production containers

### ✅ Cache Integration Tests
- Use Redis from production containers
- Test caching behavior in production setup

### ✅ Frontend E2E Tests
- Connect to production frontend container
- Test user workflows end-to-end
- Verify production frontend works correctly

## Environment

When using `--prod-containers`:
- Uses `config/.env.production` configuration
- Starts containers with production settings
- Tests connect via production ports
- Uses production Docker network

## Example Output

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
▶ PRE-PHASE: Building Production Docker Containers
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

⚠️  This ensures you're testing the EXACT containers that will be deployed to production

Building production Docker images...
✅ Production images built: vabc123-20250120-143022
  Git Commit: abc123def
  Build Date: 2025-01-20 14:30:22

Starting Production Containers for Testing
✅ Production containers are running and ready!
  Backend: http://localhost:50012
  Frontend: http://localhost:50013
  ArangoDB: http://localhost:8529
  Redis: redis://127.0.0.1:6379/

⚠️  Tests will run against PRODUCTION containers
⚠️  Using production environment configuration

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
▶ 🧪 Running ALL Tests
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

... (all tests run against production containers) ...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
▶ ✅ All Tests Completed!
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ Test Summary:
  🐳 Production Docker containers: YES (testing exact deployment artifacts)
  📦 Version Tag: vabc123-20250120-143022
  ✅ Backend unit tests (library)
  ✅ Backend integration tests
  ✅ Testing package integration tests
  ✅ Frontend E2E tests
  ✅ Cache integration tests

🎉 All tests completed against PRODUCTION containers!
These are the EXACT containers that will be deployed to production.
If all tests passed, you can confidently deploy version: vabc123-20250120-143022
```

## Next Steps After Tests Pass

If all tests pass with `--prod-containers`:

1. **Deploy with confidence**: These exact containers are tested
2. **Version tracking**: Note the version tag (e.g., `vabc123-20250120-143022`)
3. **Deploy**: Use `test-and-push-prod.sh` or deploy the tested images

## Tips

- **First run**: May take longer (builds production images)
- **Subsequent runs**: Faster (images may be cached)
- **Data safety**: Uses production environment but tests don't modify production data
- **Cleanup**: Containers are automatically stopped after tests

## Troubleshooting

### Production containers won't start
- Check `config/.env.production` exists
- Verify Docker is running
- Check ports aren't already in use

### Tests fail against production containers
- Check container logs: `docker compose logs`
- Verify services are healthy: `docker compose ps`
- Ensure production data is loaded (if needed)

### Want to keep containers running
- Remove the cleanup trap or comment it out
- Containers will stay running after tests complete
