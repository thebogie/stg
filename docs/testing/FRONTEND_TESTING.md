# Frontend Testing Strategy

**E2E tests with Playwright are the PRIMARY method for frontend testing.**

## Quick Reference

```bash
# Run E2E tests (recommended)
just test-frontend-e2e
# or
just test-frontend
```

## Testing Strategy

### ✅ Primary: E2E Tests (Playwright)

**Why E2E tests are preferred:**
- ✅ Test complete user workflows
- ✅ Work reliably in all environments (including WSL2)
- ✅ Test in real browsers (Chrome, Firefox, Safari)
- ✅ Visual regression testing
- ✅ Mobile responsiveness testing
- ✅ Full frontend-backend integration

**Location**: `testing/e2e/*.spec.ts`

**Run**: `just test-frontend-e2e` or `npx playwright test`

**See**: [E2E Testing Guide](./E2E_TESTING_GUIDE.md) for details

**Note**: WASM unit tests are not supported in WSL2/headless environments. E2E tests provide comprehensive coverage and work reliably in all environments.

## Test Commands

### E2E Tests (Recommended)

```bash
# Run all E2E tests
just test-frontend-e2e

# Run with UI mode (interactive)
npx playwright test --ui

# Run in debug mode
npx playwright test --debug

# Run specific test file
npx playwright test example.spec.ts

# Run in headed mode (see browser)
npx playwright test --headed
```

## Test Coverage

### Current E2E Tests

- ✅ Homepage loading
- ✅ Navigation display
- ✅ Basic page interactions
- ✅ Visual regression (screenshots)

### Recommended E2E Tests to Add

- 🔄 User registration flow
- 🔄 User login flow
- 🔄 Contest creation workflow
- 🔄 Venue/game search flows
- 🔄 Admin operations
- 🔄 Profile management

## Setup

### One-Time Setup

```bash
# Install Playwright and browsers
npm install
npx playwright install chromium  # Or --with-deps for all browsers

# Or use the setup command
just setup
```

### Running Tests

```bash
# E2E tests (frontend starts automatically)
just test-frontend-e2e

# Note: Backend may need to be running if tests make API calls
```

## Test Reports

After running E2E tests:

```bash
# View HTML report
npx playwright show-report

# Reports are at:
_build/playwright-report/index.html
```

## Best Practices

1. **Use E2E tests for user workflows** - They're more reliable and comprehensive
2. **Test what users do, not implementation** - Focus on user interactions
3. **Wait for WASM to load** - Always include `await page.waitForTimeout(1000)` after navigation
4. **Use specific selectors** - Prefer `data-testid` or specific selectors
5. **Add visual regression tests** - Screenshots catch visual bugs

## Troubleshooting

### E2E Tests

- **Frontend not starting**: Check that `cargo run --package frontend` works
- **Tests timeout**: Increase timeout or check backend is running
- **Browser not found**: Run `npx playwright install`

## Integration with CI/CD

E2E tests generate reports:

- **E2E**: JUnit XML at `_build/test-results/e2e-results.xml`
- **E2E**: HTML report at `_build/playwright-report/index.html`

## Summary

| Test Type | Status | When to Use |
|-----------|--------|-------------|
| **E2E (Playwright)** | ✅ **PRIMARY** | Always - comprehensive frontend testing |

**Recommendation**: E2E tests are the primary and recommended frontend testing method. They're reliable, comprehensive, and work in all environments including WSL2/headless setups.

