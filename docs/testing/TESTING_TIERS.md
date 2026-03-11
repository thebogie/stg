# Testing Tiers & Coverage Overview

This document outlines the current testing tiers and coverage levels in the STG RD project.

## Testing Pyramid Structure

```
        /\
       /  \     Tier 3: E2E Tests (Few)
      /____\    - Playwright browser tests
     /      \   - Full user workflows
    /________\  Tier 2: Integration Tests (Some)
   /          \ - API + Database tests
  /____________\ Tier 1: Unit Tests (Many)
                 - Component tests with mocks
```

## Current Testing Tiers

### Tier 1: Unit Tests ✅ **STRONG** (465+ tests)

**Location**: `backend/src/**/*_tests.rs`, `frontend/src/**/*_test.rs`

**Coverage**:
- ✅ **465+ unit tests** across backend
- ✅ Component-level testing with mocks
- ✅ Business logic validation
- ✅ Algorithm tests (Glicko2 rating system)
- ✅ Model validation tests
- ✅ Controller tests with mock repositories
- ✅ Use case tests with mock dependencies
- ✅ Frontend component tests (WASM)

**Characteristics**:
- **Fast**: Run in milliseconds
- **Isolated**: Use mocks (`MockPlayerRepository`, `MockSessionStore`, etc.)
- **Comprehensive**: Good coverage of individual components
- **No dependencies**: Don't require databases or external services

**Example**:
```rust
// backend/src/player/controller_tests.rs
#[tokio::test]
async fn test_login_handler_success() {
    let repo = MockPlayerRepository::new();
    let session_store = MockSessionStore::new();
    // Test with mocks - fast and isolated
}
```

**Status**: ✅ **Production-ready coverage**

---

### Tier 2: Integration Tests ⚠️ **PARTIAL** (~21 tests)

**Location**: 
- `testing/tests/integration_test.rs` (4 tests)
- `testing/tests/api_tests.rs` (17 tests)
- `backend/tests/*_integration_test.rs` (5 tests)

#### 2A: Infrastructure Integration Tests ✅

**Location**: `testing/tests/integration_test.rs`

**Coverage**:
- ✅ TestEnvironment creation (testcontainers)
- ✅ Redis connectivity tests
- ✅ ArangoDB container tests
- ✅ Data dump loading tests

**Characteristics**:
- Use **testcontainers** (ephemeral Docker containers)
- Test infrastructure setup
- Isolated per test (each gets fresh containers)

**Status**: ✅ **Good foundation**

#### 2B: API Integration Tests ⚠️

**Location**: `testing/tests/api_tests.rs`

**Coverage**:
- ✅ Player registration API
- ✅ Player login API
- ✅ Some endpoint testing
- ⚠️ **Incomplete**: Not all endpoints covered
- ⚠️ **Limited**: Only happy paths mostly

**Characteristics**:
- Use **testcontainers** for databases
- Test full HTTP request/response cycle
- Use real repositories (not mocks)
- Test against real ArangoDB and Redis

**Status**: ⚠️ **Needs expansion**

#### 2C: Backend Integration Tests ⚠️

**Location**: `backend/tests/*_integration_test.rs`

**Coverage**:
- ✅ Contest search integration
- ✅ Database search integration
- ✅ Venue update integration
- ✅ Ratings integration
- ✅ Database operations

**Characteristics**:
- ⚠️ **Require manual setup**: Need `BACKEND_URL` env var
- ⚠️ **Not using testcontainers**: Require running backend
- ⚠️ **Skip if backend not running**: Not fully automated
- ⚠️ **Limited coverage**: Only a few endpoints

**Status**: ⚠️ **Needs modernization**

**Gaps**:
- ❌ No tests for all CRUD operations
- ❌ No authentication flow tests
- ❌ No error case tests (400, 401, 403, 404, 500)
- ❌ No admin endpoint tests
- ❌ No production data testing (empty databases)

---

### Tier 3: End-to-End Tests ⚠️ **MINIMAL** (1 test)

**Location**: `testing/e2e/example.spec.ts`

**Coverage**:
- ⚠️ **1 example test** (Playwright setup)
- ❌ No actual user flow tests
- ❌ No frontend-backend integration tests
- ❌ No critical workflow tests

**Characteristics**:
- Use **Playwright** for browser automation
- Test full frontend + backend stack
- Visual regression testing capability
- Cross-browser testing support

**Missing**:
- ❌ User registration → login flow
- ❌ Contest creation workflow
- ❌ Venue search → view details flow
- ❌ Admin operations flow
- ❌ Game search and filtering

**Status**: ⚠️ **Infrastructure ready, tests missing**

---

## Test Execution Tiers

### Fast Feedback Tier (Unit Tests)
```bash
just test-backend        # ~30 seconds
cargo nextest run --lib  # Unit tests only
```
**Purpose**: Quick feedback during development

### Integration Tier (API + Database)
```bash
just test-integration     # ~2-5 minutes
cargo nextest run --package testing
```
**Purpose**: Validate API contracts and database operations

### Full Stack Tier (E2E)
```bash
just test-frontend-e2e    # ~10-30 minutes
npx playwright test
```
**Purpose**: Validate complete user workflows

### Complete Suite
```bash
just test-full        # ~15-60 minutes
just test-all          # All tiers
```
**Purpose**: Pre-deployment validation

---

## Coverage by Component

### Backend Components

| Component | Unit Tests | Integration Tests | E2E Tests | Status |
|-----------|-----------|------------------|-----------|--------|
| **Player** | ✅ 50+ | ⚠️ Partial | ❌ None | ⚠️ |
| **Venue** | ✅ 30+ | ⚠️ Partial | ❌ None | ⚠️ |
| **Game** | ✅ 40+ | ❌ None | ❌ None | ⚠️ |
| **Contest** | ✅ 60+ | ⚠️ Partial | ❌ None | ⚠️ |
| **Ratings** | ✅ 100+ | ✅ Some | ❌ None | ✅ |
| **Analytics** | ✅ 50+ | ❌ None | ❌ None | ⚠️ |
| **Auth** | ✅ 20+ | ⚠️ Partial | ❌ None | ⚠️ |

### Frontend Components

| Component | Unit Tests | Integration Tests | E2E Tests | Status |
|-----------|-----------|------------------|-----------|--------|
| **Pages** | ✅ Some | ❌ None | ⚠️ 1 example | ⚠️ |
| **Components** | ✅ Some | ❌ None | ❌ None | ⚠️ |
| **API Client** | ✅ Some | ❌ None | ❌ None | ⚠️ |
| **Auth** | ✅ Some | ❌ None | ❌ None | ⚠️ |

---

## Testing Infrastructure

### ✅ Available Tools

1. **cargo-nextest**: Fast, parallel test runner
2. **testcontainers-rs**: Ephemeral Docker containers
3. **cargo-llvm-cov**: Code coverage reporting
4. **Playwright**: E2E browser testing
5. **Mock implementations**: `MockPlayerRepository`, `MockSessionStore`, etc.

### ✅ Test Patterns

1. **Dependency Injection**: Trait-based DI for testability
2. **Mock Objects**: Manual mocks for unit tests
3. **Testcontainers**: Real databases for integration tests
4. **Builder Pattern**: `TestEnvironmentBuilder` for flexible setup

### ✅ Production Data Support

- Data dump loading infrastructure
- `TestEnvironmentBuilder.with_data_dump()`
- Automatic data discovery
- Sanitized production data support

---

## Coverage Gaps by Tier

### Tier 1 (Unit Tests): ✅ **GOOD**
- ✅ Comprehensive component coverage
- ✅ Good mock usage
- ⚠️ Could add more edge case tests

### Tier 2 (Integration Tests): ⚠️ **NEEDS WORK**

**Missing**:
- ❌ Complete API endpoint coverage
- ❌ Authentication/authorization tests
- ❌ Error handling tests (400, 401, 403, 404, 500)
- ❌ Production data testing
- ❌ Database migration tests
- ❌ Rate limiting tests
- ❌ Concurrent request tests

**Needs**:
- 🔄 Modernize `backend/tests/` to use testcontainers
- 🔄 Expand `testing/tests/api_tests.rs` to cover all endpoints
- 🔄 Add error case tests
- 🔄 Add production data tests

### Tier 3 (E2E Tests): ❌ **CRITICAL GAP**

**Missing**:
- ❌ User registration → login flow
- ❌ Contest creation workflow
- ❌ Venue search → view details
- ❌ Game search and filtering
- ❌ Admin operations
- ❌ Profile management
- ❌ Analytics dashboard

**Needs**:
- 🔄 Implement 5-10 critical user flows
- 🔄 Visual regression tests
- 🔄 Cross-browser testing

---

## Recommended Test Distribution

### Current Distribution
```
Tier 1 (Unit):     465 tests  (95%)
Tier 2 (Integration): ~21 tests  (4%)
Tier 3 (E2E):       1 test    (1%)
```

### Target Distribution (Industry Standard)
```
Tier 1 (Unit):     465 tests  (60-70%)
Tier 2 (Integration): 50-100 tests  (20-30%)
Tier 3 (E2E):       10-20 tests  (5-10%)
```

---

## Priority Actions

### 🔴 Critical (Do First)
1. **Expand API Integration Tests** (Tier 2)
   - Add tests for all CRUD operations
   - Add authentication flow tests
   - Add error case tests
   - Target: 50+ integration tests

2. **Implement E2E User Flows** (Tier 3)
   - User registration → login
   - Contest creation workflow
   - Venue/game search flows
   - Target: 10+ E2E tests

### 🟡 High Priority
3. **Modernize Backend Integration Tests**
   - Migrate to testcontainers
   - Remove manual setup requirements
   - Add production data tests

4. **Add Security Tests**
   - Authentication/authorization
   - Rate limiting
   - Input validation
   - CSRF protection

### 🟢 Medium Priority
5. **Add Performance Tests**
   - Load testing
   - Query performance
   - Memory leak detection

6. **Add Migration Tests**
   - Forward migrations
   - Rollback scenarios
   - Migration on existing data

---

## Running Tests by Tier

```bash
# Tier 1: Unit Tests (Fast)
just test-backend
cargo nextest run --lib

# Tier 2: Integration Tests (Medium)
just test-integration
cargo nextest run --package testing

# Tier 3: E2E Tests (Slow)
just test-frontend-e2e
npx playwright test

# All Tiers
just test-all
just test-full
```

---

## Summary

| Tier | Status | Count | Coverage | Priority |
|------|--------|-------|----------|----------|
| **Tier 1: Unit Tests** | ✅ Strong | 465+ | ~95% | ✅ Good |
| **Tier 2: Integration** | ⚠️ Partial | ~21 | ~4% | 🔴 Critical |
| **Tier 3: E2E** | ⚠️ Minimal | 1 | ~1% | 🔴 Critical |

**Overall Assessment**: Strong unit test coverage, but **critical gaps** in integration and E2E testing prevent production confidence.

