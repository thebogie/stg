# Real APIs vs Mocks: Testing Strategy Guide

This document defines when to use **real external APIs** vs **mocks/stubs** across different test types.

## Quick Decision Tree

```
┌─────────────────────────────────────┐
│ Need to test?                       │
└──────────────┬──────────────────────┘
               │
       ┌───────┴────────┐
       │                │
   Unit Test      Integration/E2E
       │                │
       ▼                ▼
    USE MOCKS      Real API Decision:
                   ┌─────┴─────┐
                   │           │
            Public/Stable?  Rate Limited?
                   │           │
              YES / NO      YES / NO
                   │           │
                   ▼           ▼
               Real API    Mock/Stub
```

## Test Tier Breakdown

### Tier 1: Unit Tests → **ALWAYS USE MOCKS** ✅

**Location**: `backend/src/**/*_tests.rs`

**Strategy**: Mock all external dependencies

**Why**:
- ✅ **Fast**: Run in milliseconds (no network calls)
- ✅ **Isolated**: Test only the component's logic
- ✅ **Reliable**: No flakiness from network issues
- ✅ **Deterministic**: Same results every time
- ✅ **CI-Friendly**: No API keys or network setup needed

**What to Mock**:
- External APIs (BGG, Google Places, etc.)
- Databases (use `MockPlayerRepository`, `MockGameRepository`)
- Session stores (`MockSessionStore`)
- HTTP clients
- File system operations
- Time/date functions

**Example**:
```rust
// backend/src/player/controller_tests.rs
#[tokio::test]
async fn test_login_handler_success() {
    let repo = MockPlayerRepository::new();  // ✅ Mock
    let session_store = MockSessionStore::new();  // ✅ Mock
    // Fast, isolated test with no external dependencies
}
```

---

### Tier 2: Integration Tests → **USE REAL APIs WHEN SAFE** ⚠️

**Location**: `testing/tests/*.rs`, `backend/tests/*_integration_test.rs`

**Strategy**: Use real infrastructure (databases) but mock external APIs based on risk

#### 2A: Internal Infrastructure → **ALWAYS REAL** ✅

**Use Real Services**:
- ✅ **ArangoDB** (via testcontainers - isolated Docker containers)
- ✅ **Redis** (via testcontainers - isolated Docker containers)
- ✅ **Internal repositories** (real database operations)

**Why**: These are your own services, completely under your control.

**Example**:
```rust
// testing/tests/api_tests.rs
#[tokio::test]
async fn test_player_registration() -> Result<()> {
    let env = TestEnvironment::new().await?;  // ✅ Real ArangoDB + Redis
    let app_data = app_setup::setup_test_app_data(&env).await?;
    // Uses real databases, but isolated per test
}
```

#### 2B: External APIs → **MOCK BY DEFAULT, REAL FOR SMALL PERFORMANCE TESTS** ⚠️

**Default Approach**: Mock external APIs in integration tests

**Mock These** (Default):
- ❌ **BGG API** (BoardGameGeek) - Not included in test setup
- ❌ **Google Places API** - Not configured in test setup
- ❌ **Payment APIs** (if any)
- ❌ **Email services**
- ❌ **Third-party webhooks**

**Why Mock External APIs** (Default):
1. **Rate Limiting**: Real APIs have rate limits that tests can hit
2. **Cost**: Some APIs charge per request
3. **Reliability**: External APIs can be down or slow
4. **Test Isolation**: Don't want test results affected by external services
5. **Speed**: Network calls add latency
6. **Determinism**: External API responses may change

**Exception: Performance Tests with Small Scenarios** ✅

For **performance tests that don't do actual load testing** (just a few requests), it's acceptable to use real APIs:

- ✅ Small scenarios (1-10 requests) won't hit rate limits
- ✅ Provides real-world performance data
- ✅ Validates actual API integration timing
- ✅ Better than mocks for performance validation

**Current Implementation**:
```rust
// testing/src/app_setup.rs
// Default: Game repository WITHOUT BGG service for tests
let game_repo = web::Data::new(
    backend::game::repository::GameRepositoryImpl::new(db.clone())
    // ✅ No BGG service - tests database search only
);

// For performance tests, can enable real APIs:
let game_repo = if use_real_apis {
    let bgg_service = BGGService::new_with_config(&BGGConfig {
        api_url: env::var("BGG_API_URL")?,
        api_token: env::var("BGG_API_TOKEN").ok(),
    });
    web::Data::new(
        backend::game::repository::GameRepositoryImpl::new_with_bgg(db.clone(), bgg_service)
    )
} else {
    web::Data::new(backend::game::repository::GameRepositoryImpl::new(db.clone()))
};
```

**Guidelines for Real API Usage in Performance Tests**:
- ✅ **Limit requests**: Keep to < 10 requests per test
- ✅ **Use test credentials**: Never production API keys
- ✅ **Mark as optional**: Use `#[ignore]` or environment variable flag
- ✅ **Don't block CI**: Make real API tests optional/non-fatal
- ❌ **No load testing**: Don't do actual load (hundreds/thousands of requests)

#### 2C: When to Use Real External APIs (Rare)

**Consider Real API When**:
1. ✅ **Public/Free API** with generous rate limits
2. ✅ **Stable API** with versioning guarantees
3. ✅ **Critical Integration** that must be validated
4. ✅ **Test Fixtures Available** (recorded responses)

**Example - Using Real API (Hypothetical)**:
```rust
// Only if BGG API is stable and we have test fixtures
#[tokio::test]
#[ignore]  // Mark as optional - don't block CI
async fn test_bgg_integration_real() -> Result<()> {
    let bgg_service = BGGService::new_with_config(&BGGConfig {
        api_url: env::var("BGG_TEST_URL")?,
        api_token: env::var("BGG_TEST_TOKEN").ok(),
    });
    // Test real BGG integration
}
```

---

### Tier 3: E2E Tests → **USE REAL APIs WHEN APPROPRIATE** 🎯

**Location**: `testing/e2e/*.spec.ts`

**Strategy**: Use real APIs when testing complete user workflows

#### 3A: Staging/Test Environments → **USE REAL** ✅

**Use Real Services**:
- ✅ **Test/staging API keys** (not production)
- ✅ **Test database** (separate from production)
- ✅ **Real external APIs** (with test credentials)

**Why**: E2E tests validate the complete system, including external integrations.

**Example**:
```typescript
// testing/e2e/game-search.spec.ts
test('user can search games from BGG', async ({ page }) => {
  // Uses real BGG API (but with test credentials)
  await page.goto('/games');
  await page.fill('input[name="search"]', 'Catan');
  await page.click('button[type="submit"]');
  // Validates real BGG integration works
});
```

#### 3B: Production → **NEVER IN TESTS** ❌

**Never**:
- ❌ Use production API keys in tests
- ❌ Hit production databases
- ❌ Send real emails/notifications
- ❌ Charge real credit cards

---

## Decision Matrix

| Test Type | Internal Services | External APIs | External APIs (E2E) |
|-----------|------------------|---------------|---------------------|
| **Unit** | Mock ✅ | Mock ✅ | N/A |
| **Integration** | Real ✅ (testcontainers) | Mock ⚠️ (default) | N/A |
| **Performance** | Real ✅ (testcontainers) | Real ✅ (small scenarios only) | N/A |
| **E2E** | Real ✅ (staging) | Real ✅ (test keys) | Real ✅ (test env) |

---

## Current Implementation

### What We're Doing Right ✅

1. **Unit Tests**: All mocks (`MockPlayerRepository`, `MockSessionStore`)
2. **Integration Tests**: Real databases (testcontainers), no BGG API (mocked)
3. **Performance Tests**: Real databases, can optionally use real APIs for small scenarios
4. **E2E Tests**: Use staging/test environment APIs

### Example: Game Search Testing

**Unit Test** (Mock):
```rust
// backend/src/game/repository_tests.rs
#[tokio::test]
async fn test_search_logic() {
    let mut mock_repo = MockGameRepository::new();
    // Test search logic with mock data
}
```

**Integration Test** (Real DB, Mock BGG):
```rust
// testing/tests/search_integration_tests.rs
#[tokio::test]
async fn test_search_games() -> Result<()> {
    let app_data = app_setup::setup_test_app_data(&env).await?;
    // ✅ Real database search
    // ❌ No BGG API calls
}
```

**E2E Test** (Real Everything):
```typescript
// testing/e2e/game-search.spec.ts
test('search games', async ({ page }) => {
  // ✅ Uses real BGG API (test credentials)
  // ✅ Validates complete user workflow
});
```

---

## Best Practices

### 1. Default to Mocks in Unit & Integration Tests

**Rule**: If you're not specifically testing external API integration, mock it.

### 2. Use Test Fixtures/VCR for Real API Testing

If you need to test with real APIs, record responses:

```rust
// Example using vcr or similar
#[tokio::test]
async fn test_bgg_with_fixture() {
    let fixture = load_fixture("bgg_search_catan.json");
    // Test with recorded response
}
```

### 3. Mark Real API Tests as Optional

```rust
#[tokio::test]
#[ignore]  // Don't run in CI by default
async fn test_real_bgg_api() {
    // Only runs with: cargo test -- --ignored
}
```

### 4. Use Environment-Specific Config

```rust
// In test setup
let bgg_service = if cfg!(test) {
    None  // No BGG in unit/integration tests
} else if env::var("USE_REAL_BGG").is_ok() {
    Some(BGGService::new_with_config(...))
} else {
    None
};
```

### 5. Separate Test Suites

```bash
# Fast tests (mocks only)
cargo test --lib

# Integration tests (real DB, mocked APIs)
cargo test --test '*_integration_test'

# E2E tests (real everything, staging)
just test-frontend-e2e
```

---

## When to Add Real API Testing

Add real API tests when:

1. ✅ **New External Integration**: First integration with a new API
2. ✅ **API Contract Changes**: When external API version changes
3. ✅ **Production Issues**: When integration bugs are discovered
4. ✅ **Critical Business Logic**: When revenue/fees depend on API
5. ✅ **Performance Validation**: Small-scenario performance tests (< 10 requests)

**But**: Keep these tests separate, optional, and clearly marked.

## Enabling Real APIs in Tests

### For Performance Tests

```bash
# Enable real BGG API for performance tests
export USE_REAL_BGG_API=1
export BGG_API_URL="https://boardgamegeek.com/xmlapi2"
export BGG_API_TOKEN="your_test_token"  # Optional

# Run performance tests with real API
cargo nextest run --package testing --test performance_tests

# Run normal integration tests (still use mocks)
cargo nextest run --package testing --test '*_integration_tests'
```

### Implementation

The test setup now supports conditional real API usage:

```rust
// testing/src/app_setup.rs
let game_repo = if std::env::var("USE_REAL_BGG_API").is_ok() {
    // Use real BGG API (for performance tests)
    // Only when explicitly enabled
} else {
    // Default: database only (for integration tests)
};
```

**Best Practice**: Only enable for specific test runs, not by default.

---

## Summary

| Test Tier | Internal Services | External APIs | Speed | Isolation |
|-----------|------------------|---------------|-------|-----------|
| **Unit** | Mock | Mock | ⚡⚡⚡ | ✅✅✅ |
| **Integration** | Real (testcontainers) | Mock | ⚡⚡ | ✅✅ |
| **Performance** | Real (testcontainers) | Real* (small scenarios) | ⚡⚡ | ✅✅ |
| **E2E** | Real (staging) | Real (test keys) | ⚡ | ✅ |

*Performance tests can use real APIs for small scenarios (< 10 requests) when `USE_REAL_BGG_API=1` is set.

**Key Principle**: Test your code, not third-party APIs. Use real APIs only when testing the integration itself is the goal.

