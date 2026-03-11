# E2E Testing Guide with Playwright

**E2E tests are the PRIMARY method for frontend testing in this project.**

## Quick Start

```bash
# Run E2E tests (recommended)
just test-frontend-e2e

# Or use the alias
just test-frontend

# Or directly
npx playwright test
```

## What Gets Tested

E2E tests verify the complete frontend application in real browsers:
- ✅ Full user workflows
- ✅ Frontend-backend integration
- ✅ Browser compatibility (Chrome, Firefox, Safari)
- ✅ Visual regression (screenshots)
- ✅ Mobile responsiveness
- ✅ WASM execution and rendering

## Test Structure

Tests are located in `testing/e2e/`:

```
testing/e2e/
├── example.spec.ts          # Basic E2E tests
└── [your-new-tests].spec.ts # Add more test files here
```

## Running Tests

### Basic Commands

```bash
# Run all E2E tests
just test-frontend-e2e

# Run specific test file
npx playwright test example.spec.ts

# Run in headed mode (see browser)
npx playwright test --headed

# Run in debug mode (step through)
npx playwright test --debug

# Run specific browser
npx playwright test --project=chromium
```

### Test Reports

After running tests, view the HTML report:

```bash
# Reports are automatically generated at:
_build/playwright-report/index.html

# Open in browser
npx playwright show-report
```

## Writing E2E Tests

### Basic Test Structure

```typescript
import { test, expect } from '@playwright/test';

test.describe('Feature Name', () => {
  test('should do something', async ({ page }) => {
    // Navigate to page
    await page.goto('/');
    
    // Wait for WASM to load
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000); // Give WASM time to initialize
    
    // Interact with page
    await page.click('button#submit');
    
    // Assert results
    await expect(page.locator('.result')).toBeVisible();
  });
});
```

### Common Patterns

#### Waiting for WASM

```typescript
// Always wait for WASM to load
await page.goto('/');
await page.waitForLoadState('networkidle');
await page.waitForTimeout(1000); // WASM initialization
```

#### Testing User Flows

```typescript
test('user registration flow', async ({ page }) => {
  await page.goto('/register');
  await page.waitForLoadState('networkidle');
  await page.waitForTimeout(1000);
  
  // Fill form
  await page.fill('input[name="username"]', 'testuser');
  await page.fill('input[name="email"]', 'test@example.com');
  await page.fill('input[name="password"]', 'password123');
  
  // Submit
  await page.click('button[type="submit"]');
  
  // Verify success
  await expect(page.locator('.success-message')).toBeVisible();
});
```

#### Visual Regression

```typescript
test('homepage visual snapshot', async ({ page }) => {
  await page.goto('/');
  await page.waitForLoadState('networkidle');
  await page.waitForTimeout(1000);
  
  // Take screenshot
  await expect(page).toHaveScreenshot('homepage.png', {
    fullPage: true,
  });
});
```

## Configuration

Playwright is configured in `playwright.config.ts`:

- **Test Directory**: `./testing/e2e`
- **Base URL**: `http://localhost:8080` (or `FRONTEND_URL` env var)
- **Browsers**: Chromium, Firefox, WebKit (Safari), Mobile Chrome, Mobile Safari
- **Auto-start Frontend**: Yes (via `cargo run --package frontend`)

### Environment Variables

```bash
# Override frontend URL
FRONTEND_URL=http://localhost:3000 npx playwright test

# CI mode (more retries, single worker)
CI=true npx playwright test
```

## Prerequisites

### Install Playwright Browsers

```bash
# One-time setup (installs browsers only)
npx playwright install

# With system dependencies (may require sudo)
npx playwright install --with-deps

# Or use the setup command (may require sudo for system deps)
just setup
```

**Note**: In WSL2 or headless environments, you may only need browsers without system dependencies:
```bash
npx playwright install chromium  # Just Chromium
```

### Backend Requirements

For full E2E tests, you may need the backend running:

```bash
# Option 1: Start backend manually
cargo run --package backend

# Option 2: Use hybrid dev environment
./scripts/setup-hybrid-dev.sh
```

**Note**: The frontend server starts automatically via Playwright's `webServer` config, but the backend needs to be running separately if your tests make API calls.

## Best Practices

### 1. Wait for WASM

Always wait for WASM to initialize:

```typescript
await page.waitForLoadState('networkidle');
await page.waitForTimeout(1000);
```

### 2. Use Specific Selectors

Prefer data-testid or specific selectors:

```typescript
// Good
await page.click('[data-testid="submit-button"]');

// Avoid
await page.click('button'); // Too generic
```

### 3. Test User Flows, Not Implementation

Focus on what users do, not how it's implemented:

```typescript
// Good: Test user workflow
test('user can register and login', async ({ page }) => {
  // Register
  await page.goto('/register');
  // ... registration steps
  
  // Login
  await page.goto('/login');
  // ... login steps
});

// Avoid: Testing internal state
// Don't test WASM internals directly
```

### 4. Use Page Object Pattern (for complex tests)

For complex features, create page objects:

```typescript
// testing/e2e/pages/LoginPage.ts
export class LoginPage {
  constructor(private page: Page) {}
  
  async goto() {
    await this.page.goto('/login');
    await this.page.waitForLoadState('networkidle');
    await this.page.waitForTimeout(1000);
  }
  
  async login(username: string, password: string) {
    await this.page.fill('input[name="username"]', username);
    await this.page.fill('input[name="password"]', password);
    await this.page.click('button[type="submit"]');
  }
}

// In test
const loginPage = new LoginPage(page);
await loginPage.goto();
await loginPage.login('user', 'pass');
```

## Debugging

### Debug Mode

```bash
# Step through tests
npx playwright test --debug

# Debug specific test
npx playwright test example.spec.ts --debug
```

### Screenshots and Videos

Screenshots are automatically taken on failure. View them at:
- `_build/test-results/` (screenshots)
- `_build/playwright-report/` (HTML report with screenshots)

### Trace Viewer

Traces are collected on retry. View them:

```bash
npx playwright show-trace trace.zip
```

## CI/CD Integration

Playwright generates JUnit XML for CI:

```bash
# JUnit XML is automatically generated at:
_build/test-results/e2e-results.xml
```

### GitHub Actions Example

```yaml
- name: Run E2E tests
  run: |
    npx playwright install --with-deps
    npx playwright test
    
- name: Upload test results
  uses: actions/upload-artifact@v3
  if: always()
  with:
    name: playwright-report
    path: _build/playwright-report/
```

## Comparison: E2E vs WASM Unit Tests

| Aspect | E2E Tests (Playwright) | WASM Unit Tests |
|--------|------------------------|-----------------|
| **Coverage** | Full user workflows | Individual functions |
| **Browser** | Real browsers | Geckodriver (unreliable) |
| **Speed** | Slower (10-30 min) | Faster (if working) |
| **Reliability** | ✅ Very reliable | ⚠️ Fails in WSL2/headless |
| **Visual Testing** | ✅ Screenshots | ❌ No |
| **Cross-browser** | ✅ Yes | ❌ Firefox only |
| **Recommended** | ✅ **YES** | ⚠️ Optional |

## Troubleshooting

### Tests Timeout

```typescript
// Increase timeout for specific test
test('slow test', async ({ page }) => {
  test.setTimeout(60000); // 60 seconds
  // ...
});
```

### Frontend Not Starting

Check that:
1. Frontend builds successfully: `cd frontend && cargo build`
2. Port 8080 is available
3. Backend is running (if needed)

### Browser Not Found

```bash
# Reinstall browsers
npx playwright install --with-deps
```

### WASM Not Loading

Ensure:
1. Frontend is built: `cd frontend && wasm-pack build`
2. Wait time is sufficient: `await page.waitForTimeout(1000)`
3. Check browser console for errors

## Next Steps

1. ✅ Add more E2E tests for critical user flows
2. ✅ Implement page object pattern for complex features
3. ✅ Add visual regression tests for key pages
4. ✅ Set up CI/CD integration

## Resources

- [Playwright Documentation](https://playwright.dev)
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
- [Test Examples](https://playwright.dev/docs/test-examples)

