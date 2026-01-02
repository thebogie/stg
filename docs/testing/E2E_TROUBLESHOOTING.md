# E2E Test Troubleshooting

## Common Issues

### 1. WebServer Timeout on First Run

**Problem**: E2E tests timeout waiting for the frontend server to start.

**Symptoms**:
```
Error: Timed out waiting 300000ms from config.webServer.
```

**Cause**: First-time build of the frontend WASM takes 3-5 minutes.

**Solution**:
- **Wait**: The first run takes longer as Trunk builds the frontend. Subsequent runs are faster.
- **Pre-build**: Build the frontend first to speed up:
  ```bash
  cd frontend
  cargo build --target wasm32-unknown-unknown --no-default-features --features frontend
  ```
- **Increase timeout**: Already set to 5 minutes (300s) in `playwright.config.ts`

### 2. Frontend Server Not Starting

**Problem**: Trunk fails to start.

**Symptoms**:
```
Error: Process from config.webServer was not able to start.
```

**Solutions**:
- **Check Trunk is installed**: `which trunk` or `cargo install trunk`
- **Check port 8080 is available**: `lsof -i :8080`
- **Check frontend builds**: `cd frontend && cargo build --target wasm32-unknown-unknown`
- **Check CSS exists**: `ls frontend/public/styles.css`

### 3. Tests Can't Connect to Frontend

**Problem**: Tests run but can't reach the frontend.

**Symptoms**:
```
page.goto: net::ERR_CONNECTION_REFUSED
```

**Solutions**:
- **Check server started**: Look for `[WebServer]` logs showing "Serving on http://..."
- **Check port**: Ensure `FRONTEND_URL` matches the port in `playwright.config.ts` (default: 8080)
- **Check firewall**: Ensure port 8080 is accessible

### 4. WASM Not Loading

**Problem**: Page loads but WASM doesn't execute.

**Symptoms**: Tests pass navigation but fail on interactions.

**Solutions**:
- **Wait longer**: Add `await page.waitForTimeout(2000)` after navigation
- **Check browser console**: Look for WASM errors in Playwright's browser console
- **Verify build**: Ensure frontend built successfully (check for `target/wasm32-unknown-unknown/debug/frontend.wasm`)

### 5. Slow Test Execution

**Problem**: Tests take a long time to run.

**Solutions**:
- **First run is slow**: Initial build takes 3-5 minutes, subsequent runs are faster
- **Use single browser**: Run `npx playwright test --project=chromium` instead of all browsers
- **Skip visual tests**: Comment out visual regression tests if not needed
- **Run in parallel**: Tests run in parallel by default (disable with `--workers=1` if needed)

## Quick Fixes

### Pre-build Frontend (Recommended for CI)

```bash
# Build frontend before running tests
cd frontend
cargo build --target wasm32-unknown-unknown --no-default-features --features frontend

# Then run tests (will be faster)
cd ..
just test-frontend-e2e
```

### Run Tests with Existing Server

If you already have the frontend running:

```bash
# Start frontend manually
cd frontend
trunk serve --port 8080

# In another terminal, run tests (will reuse existing server)
just test-frontend-e2e
```

### Debug Server Startup

```bash
# Run trunk manually to see errors
cd frontend
trunk serve --address 0.0.0.0 --port 8080 --no-default-features --features frontend
```

## Environment Variables

Set these if needed:

```bash
# Override frontend URL
export FRONTEND_URL=http://localhost:3000

# Disable color output (may cause issues with trunk)
unset NO_COLOR

# Run tests
just test-frontend-e2e
```

## Still Having Issues?

1. **Check Playwright logs**: Look at `_build/playwright-report/` for detailed error messages
2. **Check WebServer logs**: Look for `[WebServer]` prefix in test output
3. **Verify dependencies**: `npm install` and `npx playwright install chromium`
4. **Clean build**: `cd frontend && cargo clean && cargo build --target wasm32-unknown-unknown`

