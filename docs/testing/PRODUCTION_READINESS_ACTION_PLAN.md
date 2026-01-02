# Production Readiness Action Plan

## Current Confidence: **~40%** ⚠️

You have a good foundation, but **cannot confidently say "tests passed = production ready"** yet.

## What You Have ✅

1. **465 unit tests** - Good component coverage
2. **Testcontainers infrastructure** - Ephemeral containers working
3. **Some API tests** - But incomplete and require manual setup
4. **Modern tooling** - Nextest, testcontainers, Playwright

## Critical Gaps to Fix

### Priority 1: Full-Stack API Tests (CRITICAL - Do First)

**Current State**: Some tests exist but:
- Require `BACKEND_URL` env var (manual setup)
- Don't use testcontainers
- Only test a few endpoints
- Skip if backend not running

**What to Build**:

```rust
// Example: testing/tests/api_tests.rs
use testing::TestEnvironment;
use actix_web::test;

#[tokio::test]
async fn test_player_registration() {
    let env = TestEnvironment::new().await?;
    // Start backend server with testcontainers DB
    // Test POST /api/players/register
    // Verify player created in database
}

#[tokio::test]
async fn test_player_login() {
    // Test POST /api/players/login
    // Verify session created in Redis
    // Verify JWT token returned
}
```

**Required Tests** (Minimum 20-30):
- ✅ Player registration
- ✅ Player login
- ✅ Player logout
- ✅ Get current player
- ✅ Update player email/handle/password
- ✅ Venue CRUD operations
- ✅ Game search
- ✅ Contest creation
- ✅ Authentication required endpoints
- ✅ Admin-only endpoints
- ✅ Error cases (400, 401, 403, 404, 500)

### Priority 2: End-to-End User Flows (HIGH)

**What to Build**:

```rust
// Example: testing/tests/e2e_flows.rs
#[tokio::test]
async fn test_user_registration_flow() {
    // 1. Register new user
    // 2. Login with credentials
    // 3. Get user profile
    // 4. Update profile
    // 5. Logout
}
```

**Required Flows** (Minimum 5):
- User registration → login → profile update
- Venue search → view details
- Contest creation → join contest
- Admin operations
- Error recovery flows

### Priority 3: Production Data Testing (MEDIUM)

**Current State**: Scripts exist but not used

**What to Build**:

```rust
// Update TestEnvironment to load data dumps
let env = TestEnvironmentBuilder::new()
    .with_data_dump("_build/dumps/dump.sanitized.json.gz")
    .build()
    .await?;
```

**Required**:
- Implement data dump loading in `TestEnvironment`
- Create tests that use sanitized production data
- Test with realistic data volumes

### Priority 4: Security & Error Handling (MEDIUM)

**Required Tests**:
- Authentication required (401 for unauthenticated)
- Authorization (403 for non-admin on admin endpoints)
- CSRF protection
- Input validation (400 for invalid input)
- Database connection failures
- Redis connection failures

## Implementation Roadmap

### Week 1: Critical API Tests
**Goal**: Can test all API endpoints automatically

1. Create `testing/tests/api_tests.rs`
2. Build helper to start backend with testcontainers
3. Test all player endpoints (register, login, logout, get, update)
4. Test all venue endpoints
5. Test all game endpoints
6. Test all contest endpoints

**Success Criteria**: 20+ API tests passing automatically

### Week 2: E2E Flows
**Goal**: Can test complete user journeys

1. Create `testing/tests/e2e_flows.rs`
2. Test user registration → login flow
3. Test contest creation flow
4. Test venue search flow
5. Test admin operations flow

**Success Criteria**: 5+ E2E flows passing

### Week 3: Production Data
**Goal**: Can test with real production data

1. Implement data dump loading
2. Create tests with sanitized data
3. Test with realistic volumes

**Success Criteria**: Tests run with production data

### Week 4: Security & Errors
**Goal**: Security and error handling verified

1. Authentication/authorization tests
2. Error handling tests
3. Input validation tests

**Success Criteria**: All security paths tested

## Quick Start: First API Test

Here's a template to get you started:

```rust
// testing/tests/api_tests.rs
use testing::TestEnvironment;
use actix_web::{test, web, App};
use backend::*; // Your backend modules

#[tokio::test]
async fn test_player_registration() -> Result<()> {
    // Setup test environment with containers
    let env = TestEnvironment::new().await?;
    env.wait_for_ready().await?;

    // Setup backend with testcontainers DB
    let db = setup_database(env.arangodb_url()).await?;
    let redis = setup_redis(env.redis_url()).await?;
    
    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .app_data(web::Data::new(redis))
            .configure(backend::player::controller::configure_routes)
    ).await;

    // Test registration
    let req = test::TestRequest::post()
        .uri("/api/players/register")
        .set_json(&json!({
            "username": "testuser",
            "email": "test@example.com",
            "password": "password123"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Verify player in database
    // ... assertions

    Ok(())
}
```

## Success Metrics

**Current**: ~40% confidence
- Unit tests: ✅
- Basic integration: ⚠️ (incomplete)
- API tests: ❌ (manual, incomplete)
- E2E: ❌
- Production data: ❌

**After Week 1**: ~60% confidence
- Unit tests: ✅
- API tests: ✅ (automated, comprehensive)

**After Week 2**: ~75% confidence
- Unit tests: ✅
- API tests: ✅
- E2E flows: ✅

**After Week 3-4**: ~90% confidence
- All above: ✅
- Production data: ✅
- Security: ✅

**Production Ready**: ~90%+ confidence
- All tests automated
- All critical paths covered
- Production data tested
- Security verified

## Recommendation

**Do NOT deploy to production** until you complete at least:
1. ✅ Full-Stack API Tests (Week 1)
2. ✅ E2E Flows (Week 2)

After that, you can say: **"Tests passed = production ready"** with ~75% confidence.

For 90%+ confidence, complete all 4 weeks.

