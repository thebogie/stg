# Integration Tests vs E2E Tests

This document clearly defines the distinction between **Integration Tests** and **End-to-End (E2E) Tests** in the STG RD project.

## Quick Summary

| Aspect | Integration Tests | E2E Tests |
|--------|------------------|-----------|
| **What they test** | Backend API + Database | Full stack (Frontend + Backend + Browser) |
| **Language** | Rust | TypeScript/JavaScript |
| **Location** | `testing/tests/*.rs`, `backend/tests/*_integration_test.rs` | `testing/e2e/*.spec.ts` |
| **Tools** | testcontainers, actix-web test utils | Playwright |
| **Dependencies** | Real databases (ArangoDB, Redis) | Real browser, frontend server, backend server |
| **Speed** | Medium (2-5 minutes) | Slow (10-30 minutes) |
| **Scope** | API contracts, database operations | Complete user workflows |

## Integration Tests (Tier 2)

### Definition
Integration tests verify that **backend components work together correctly** with real infrastructure (databases, Redis) but **without the frontend or browser**.

### Characteristics

1. **Test Backend API Endpoints**
   - HTTP request/response cycle
   - API contracts and data formats
   - Authentication/authorization flows
   - Error handling (400, 401, 403, 404, 500)

2. **Use Real Infrastructure**
   - Real ArangoDB database (via testcontainers)
   - Real Redis (via testcontainers)
   - Real repositories (not mocks)
   - Real session management

3. **No Frontend/Browser**
   - Direct HTTP calls to backend API
   - No browser rendering
   - No WASM execution
   - No UI interaction

4. **Isolated Per Test**
   - Each test gets fresh Docker containers
   - Ephemeral databases (created/destroyed per test)
   - No shared state between tests

### Example

```rust
// testing/tests/api_tests.rs
#[tokio::test]
async fn test_player_registration() -> Result<()> {
    // 1. Start testcontainers (ArangoDB + Redis)
    let env = TestEnvironment::new().await?;
    
    // 2. Set up backend app with real databases
    let app_data = app_setup::setup_test_app_data(&env).await?;
    let app = test::init_service(/* ... */).await;
    
    // 3. Make HTTP request to backend API
    let req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({ "username": "test", ... }))
        .to_request();
    
    // 4. Verify response
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    // 5. Verify data in database
    // (containers auto-cleanup when test ends)
    Ok(())
}
```

### What Integration Tests Verify

✅ **API Endpoints Work**
- Registration, login, logout
- CRUD operations
- Search and filtering
- Authentication flows

✅ **Database Operations**
- Data persistence
- Queries return correct results
- Transactions work correctly
- Relationships are maintained

✅ **Error Handling**
- Invalid input returns 400
- Unauthorized requests return 401
- Not found returns 404
- Server errors return 500

✅ **Business Logic Integration**
- Multiple components working together
- Real data flow through the system
- State management (sessions, caching)

### What Integration Tests DON'T Test

❌ Frontend rendering
❌ Browser behavior
❌ User interactions (clicks, forms)
❌ Visual appearance
❌ WASM compilation/execution
❌ Cross-browser compatibility

---

## End-to-End (E2E) Tests (Tier 3)

### Definition
E2E tests verify that the **complete application works correctly** from the user's perspective, including the frontend, backend, and browser.

### Characteristics

1. **Test Full User Workflows**
   - Complete user journeys
   - Multi-step interactions
   - Real user scenarios

2. **Use Real Browser**
   - Playwright browser automation
   - Actual WASM frontend execution
   - Real DOM rendering
   - Browser APIs (localStorage, fetch, etc.)

3. **Test Frontend + Backend**
   - Frontend makes real API calls
   - Backend processes requests
   - Data flows through entire stack
   - Visual verification possible

4. **Slower but Comprehensive**
   - Browser startup overhead
   - Network requests
   - Rendering time
   - Full stack execution

### Example

```typescript
// testing/e2e/user-registration.spec.ts
import { test, expect } from '@playwright/test';

test('user can register and login', async ({ page }) => {
  // 1. Navigate to frontend (starts frontend server automatically)
  await page.goto('/');
  
  // 2. Interact with UI (real browser, real WASM)
  await page.click('text=Register');
  await page.fill('input[name="username"]', 'testuser');
  await page.fill('input[name="email"]', 'test@example.com');
  await page.fill('input[name="password"]', 'password123');
  await page.click('button[type="submit"]');
  
  // 3. Verify UI updates (real rendering)
  await expect(page.locator('text=Welcome, testuser')).toBeVisible();
  
  // 4. Verify backend state (via API or UI)
  await page.goto('/profile');
  await expect(page.locator('text=test@example.com')).toBeVisible();
});
```

### What E2E Tests Verify

✅ **Complete User Workflows**
- Registration → Login → Use app
- Search → View details → Take action
- Create contest → Manage → View results

✅ **Frontend-Backend Integration**
- API calls from frontend work
- Data displays correctly in UI
- Errors are shown to users
- Loading states work

✅ **Browser Compatibility**
- Works in Chrome, Firefox, Safari
- WASM loads and executes
- Browser APIs function correctly

✅ **Visual/UI Behavior**
- Pages render correctly
- Forms submit properly
- Navigation works
- Visual regression (screenshots)

### What E2E Tests DON'T Test

❌ Individual API endpoints in isolation
❌ Database query performance
❌ Internal business logic details
❌ Error handling edge cases (usually)
❌ Fast feedback (too slow for TDD)

---

## Key Differences

### 1. **Scope**

**Integration Tests**: Backend only
```
HTTP Request → Backend API → Database → Response
```

**E2E Tests**: Full stack
```
User Action → Browser → Frontend (WASM) → API → Database → Response → UI Update
```

### 2. **What Gets Tested**

| Integration Tests | E2E Tests |
|------------------|-----------|
| API contracts | User workflows |
| Database operations | UI interactions |
| Authentication logic | Visual appearance |
| Error responses | Browser compatibility |
| Data persistence | Frontend state management |

### 3. **Test Speed**

- **Integration**: 2-5 minutes for full suite
- **E2E**: 10-30 minutes for full suite

### 4. **Failure Debugging**

**Integration Tests**:
- Fast feedback
- Clear error messages
- Easy to debug (Rust stack traces)
- Isolated failures

**E2E Tests**:
- Slower feedback
- May need screenshots/videos
- Harder to debug (browser + WASM)
- Can fail due to timing issues

### 5. **When to Use Each**

**Use Integration Tests When**:
- Testing API endpoints
- Verifying database operations
- Testing authentication flows
- Validating business logic integration
- Need fast feedback during development

**Use E2E Tests When**:
- Testing complete user workflows
- Verifying frontend-backend integration
- Testing browser compatibility
- Validating visual appearance
- Pre-deployment smoke tests

---

## Current Test Distribution

### Integration Tests
- **Location**: `testing/tests/`, `backend/tests/`
- **Count**: ~21 tests
- **Coverage**: API endpoints, database operations
- **Status**: ⚠️ Needs expansion

### E2E Tests
- **Location**: `testing/e2e/`
- **Count**: 1 example test
- **Coverage**: Minimal (infrastructure only)
- **Status**: ⚠️ Needs implementation

---

## Running Tests

### Integration Tests
```bash
# Run all integration tests
just test-integration
cargo nextest run --package testing

# Run specific integration test
cargo nextest run --package testing --test api_tests test_player_registration
```

### E2E Tests
```bash
# Run all E2E tests
just test-frontend-e2e
npx playwright test

# Run specific E2E test
npx playwright test user-registration.spec.ts

# Run with UI (headed mode)
npx playwright test --headed
```

---

## Best Practices

### Integration Tests
1. ✅ Use testcontainers for isolation
2. ✅ Test both happy paths and error cases
3. ✅ Verify database state after operations
4. ✅ Keep tests fast (< 30 seconds each)
5. ✅ Use descriptive test names

### E2E Tests
1. ✅ Test complete user workflows
2. ✅ Use page object pattern for maintainability
3. ✅ Add visual regression tests
4. ✅ Test critical paths only (keep suite small)
5. ✅ Use screenshots for debugging failures

---

## Summary

**Integration Tests** = Backend API + Database (no browser, no frontend)
- Fast, isolated, test API contracts
- Written in Rust
- Use testcontainers

**E2E Tests** = Full Stack (browser + frontend + backend)
- Slow, comprehensive, test user workflows
- Written in TypeScript
- Use Playwright

Both are essential for a well-tested application, but they serve different purposes and complement each other.

