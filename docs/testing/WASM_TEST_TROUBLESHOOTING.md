# WASM Unit Test Troubleshooting

## Geckodriver Issues

If you see errors like:
```
driver status: signal: 9 (SIGKILL)
Error: http status: 500
```

This is a known issue with `wasm-bindgen-test` in certain environments, particularly:
- WSL2 (Windows Subsystem for Linux)
- Headless environments without proper display setup
- CI environments without Firefox properly installed

## Solutions

### Option 1: Skip WASM Unit Tests (Recommended)

The WASM unit tests are less critical since we have comprehensive E2E tests with Playwright. The `test-frontend-unit` command is designed to gracefully handle geckodriver failures:

```bash
just test-frontend-unit
# Will show a warning but won't fail the build
```

### Option 2: Use E2E Tests Instead

E2E tests provide better coverage and work reliably:

```bash
just test-frontend-e2e
# Uses Playwright, which handles browsers better
```

### Option 3: Install Firefox Properly (If Needed)

If you really need WASM unit tests to work:

```bash
# On Ubuntu/Debian
sudo apt-get update
sudo apt-get install firefox

# Verify geckodriver
which geckodriver
```

### Option 4: Run Tests in Non-Headless Mode

If you have a display available:

```bash
cd frontend
wasm-pack test --firefox  # Remove --headless flag
```

## Why This Happens

`wasm-bindgen-test` uses geckodriver (Firefox's WebDriver) to run WASM tests in a browser. In headless environments or WSL2, geckodriver may be killed by the system due to:
- Memory constraints
- Missing display server
- Incompatible Firefox installation
- System security policies

## Recommendation

**For most development workflows, skip WASM unit tests and rely on:**
1. ✅ **E2E tests** (`just test-frontend-e2e`) - Comprehensive browser testing
2. ✅ **Backend unit tests** (`just test-backend`) - Fast, reliable
3. ✅ **Integration tests** (`just test-integration`) - API and database testing

The WASM unit tests are nice-to-have but not critical since E2E tests cover the same functionality more thoroughly.

