# Test Coverage Recommendations

This document provides recommendations for additional tests to improve coverage across unit, integration, and E2E test suites.

## Current Coverage Summary

- **Unit Tests**: ✅ 465+ tests (Strong coverage)
- **Integration Tests**: ⚠️ ~21 tests (Partial coverage)
- **E2E Tests**: ⚠️ 6 spec files (Basic coverage)

## Priority Recommendations

### 🔴 Critical Priority - Integration Tests

#### 1. Complete CRUD Operations Testing

**Missing Integration Tests:**

##### Venues API (`/api/venues`)
- ✅ GET `/api/venues` - List all venues
- ❌ **POST `/api/venues`** - Create venue (with validation)
- ❌ **PUT `/api/venues/{id}`** - Update venue
- ❌ **DELETE `/api/venues/{id}`** - Delete venue
- ❌ **GET `/api/venues/{id}`** - Get single venue
- ❌ **GET `/api/venues/search`** - Search venues with filters
- ❌ **GET `/api/venues/search/db`** - Database search
- ❌ **POST `/api/venues/search/create`** - Search and create

**Test Scenarios Needed:**
```rust
// testing/tests/venue_api_tests.rs
- test_create_venue_success()
- test_create_venue_validation_errors()
- test_create_venue_duplicate_name()
- test_update_venue_success()
- test_update_venue_not_found()
- test_delete_venue_success()
- test_delete_venue_not_found()
- test_get_venue_not_found()
- test_search_venues_with_filters()
- test_search_venues_empty_results()
- test_venue_unauthorized_access()
```

##### Games API (`/api/games`)
- ✅ GET `/api/games` - List all games
- ❌ **POST `/api/games`** - Create game
- ❌ **PUT `/api/games/{id}`** - Update game
- ❌ **DELETE `/api/games/{id}`** - Delete game
- ❌ **GET `/api/games/{id}`** - Get single game
- ❌ **GET `/api/games/search`** - Search games
- ❌ **GET `/api/games/search/db`** - Database search

**Test Scenarios Needed:**
```rust
// testing/tests/game_api_tests.rs
- test_create_game_success()
- test_create_game_with_bgg_data()
- test_update_game_success()
- test_delete_game_success()
- test_get_game_not_found()
- test_search_games_by_name()
- test_search_games_by_category()
```

##### Contests API (`/api/contests`)
- ✅ POST `/api/contests` - Create contest
- ❌ **GET `/api/contests/{id}`** - Get contest details
- ❌ **GET `/api/contests/search`** - Search contests
- ❌ **GET `/api/contests/player/{player_id}/game/{game_id}`** - Get player-game contests

**Test Scenarios Needed:**
```rust
// testing/tests/contest_api_tests.rs
- test_get_contest_success()
- test_get_contest_not_found()
- test_search_contests_by_date_range()
- test_search_contests_by_player()
- test_search_contests_by_game()
- test_get_player_game_contests()
```

#### 2. Authentication & Authorization Tests

**Missing Tests:**
```rust
// testing/tests/auth_integration_tests.rs
- test_session_expiration()
- test_invalid_session_token()
- test_malformed_authorization_header()
- test_missing_authorization_header()
- test_admin_endpoints_require_admin_role()
- test_non_admin_cannot_access_admin_endpoints()
- test_update_email_requires_authentication()
- test_update_handle_requires_authentication()
- test_update_password_requires_authentication()
- test_concurrent_sessions()
- test_logout_invalidates_session()
- test_session_persistence_across_requests()
```

#### 3. Error Handling Integration Tests

**Missing Tests:**
```rust
// testing/tests/error_handling_tests.rs
- test_400_bad_request_validation_errors()
- test_401_unauthorized_missing_auth()
- test_401_unauthorized_invalid_session()
- test_403_forbidden_non_admin()
- test_404_not_found_resources()
- test_500_internal_server_error_database_failure()
- test_500_internal_server_error_redis_failure()
- test_malformed_json_request()
- test_oversized_request_body()
- test_invalid_content_type()
- test_missing_required_fields()
- test_invalid_field_types()
- test_sql_injection_attempts() // AQL injection
- test_xss_attempts()
```

#### 4. Analytics API Tests

**Missing Tests:**
```rust
// testing/tests/analytics_api_tests.rs
- test_get_analytics_health()
- test_get_platform_analytics()
- test_get_insights()
- test_get_leaderboard()
- test_get_player_stats()
- test_get_player_achievements()
- test_get_player_rankings()
- test_get_contest_stats()
- test_get_contest_difficulty()
- test_get_contest_excitement()
- test_get_contest_trends()
- test_get_recent_contests()
- test_cache_invalidation()
- test_chart_endpoints()
```

#### 5. Ratings API Tests

**Missing Tests:**
```rust
// testing/tests/ratings_api_tests.rs
- test_get_player_rating()
- test_get_rating_history()
- test_get_leaderboard()
- test_scheduler_status()
- test_manual_scheduler_trigger()
- test_admin_only_scheduler_endpoints()
```

#### 6. Client Analytics API Tests

**Missing Tests:**
```rust
// testing/tests/client_analytics_api_tests.rs
- test_sync_client_data()
- test_query_client_analytics()
- test_validate_client_data()
- test_get_sync_status()
- test_clear_client_data()
```

#### 7. Timezone API Tests

**Missing Tests:**
```rust
// testing/tests/timezone_api_tests.rs
- test_get_timezone_by_coordinates()
- test_timezone_conversion()
- test_invalid_coordinates()
```

#### 8. Search & Filtering Tests

**Missing Tests:**
```rust
// testing/tests/search_integration_tests.rs
- test_search_pagination()
- test_search_sorting()
- test_search_filtering_multiple_criteria()
- test_search_empty_query()
- test_search_special_characters()
- test_search_case_insensitive()
- test_search_partial_matches()
- test_search_performance_large_datasets()
```

#### 9. Concurrent Operations Tests

**Missing Tests:**
```rust
// testing/tests/concurrency_tests.rs
- test_concurrent_venue_creation()
- test_concurrent_game_updates()
- test_concurrent_contest_creation()
- test_race_condition_prevention()
- test_database_transaction_isolation()
- test_redis_session_concurrency()
```

#### 10. Edge Cases & Boundary Tests

**Missing Tests:**
```rust
// testing/tests/edge_cases_tests.rs
- test_very_long_strings()
- test_unicode_characters()
- test_empty_strings()
- test_null_values()
- test_extremely_large_numbers()
- test_negative_numbers_where_invalid()
- test_date_boundaries()
- test_timezone_edge_cases()
- test_maximum_field_lengths()
- test_minimum_field_lengths()
```

---

### 🔴 Critical Priority - E2E Tests

#### 1. Complete User Workflows

**Missing E2E Tests:**

```typescript
// testing/e2e/user_workflows.spec.ts
- test('complete user registration and profile setup')
- test('complete contest creation workflow')
- test('complete venue search and selection workflow')
- test('complete game search and selection workflow')
- test('complete rating viewing workflow')
- test('complete profile update workflow')
```

#### 2. CRUD Operations E2E

**Missing Tests:**
```typescript
// testing/e2e/crud_operations.spec.ts
- test('create venue from UI')
- test('update venue from UI')
- test('delete venue from UI')
- test('create game from UI')
- test('update game from UI')
- test('delete game from UI')
- test('create contest from UI')
- test('view contest details')
```

#### 3. Search & Filtering E2E

**Missing Tests:**
```typescript
// testing/e2e/search.spec.ts
- test('search venues with filters')
- test('search games with filters')
- test('search contests with date range')
- test('pagination in search results')
- test('sorting in search results')
```

#### 4. Analytics Dashboard E2E

**Missing Tests:**
```typescript
// testing/e2e/analytics.spec.ts
- test('view player statistics')
- test('view leaderboard')
- test('view contest analytics')
- test('view achievement progress')
- test('interact with charts')
```

#### 5. Admin Features E2E

**Missing Tests:**
```typescript
// testing/e2e/admin.spec.ts
- test('admin login and access')
- test('admin scheduler controls')
- test('admin data management')
- test('non-admin cannot access admin features')
```

#### 6. Error Handling E2E

**Missing Tests:**
```typescript
// testing/e2e/error_handling.spec.ts
- test('display 404 page for invalid routes')
- test('display error messages for failed API calls')
- test('handle network timeouts gracefully')
- test('handle validation errors in forms')
- test('handle session expiration')
```

#### 7. Performance & Load E2E

**Missing Tests:**
```typescript
// testing/e2e/performance.spec.ts
- test('page load times are acceptable')
- test('search results load quickly')
- test('large datasets render correctly')
- test('infinite scroll performance')
```

---

### 🟡 High Priority - Unit Tests

#### 1. Middleware Unit Tests

**Missing Tests:**
```rust
// backend/src/middleware_tests.rs
- test_cors_middleware_origin_validation()
- test_cors_middleware_methods()
- test_cors_middleware_headers()
- test_logger_middleware_request_logging()
- test_logger_middleware_error_logging()
- test_admin_ip_allowlist_middleware()
- test_admin_audit_middleware()
```

#### 2. Repository Error Handling

**Missing Tests:**
```rust
// backend/src/*/repository_tests.rs
- test_repository_database_connection_failure()
- test_repository_query_timeout()
- test_repository_transaction_rollback()
- test_repository_duplicate_key_errors()
- test_repository_not_found_handling()
```

#### 3. Use Case Validation

**Missing Tests:**
```rust
// backend/src/*/usecase_tests.rs
- test_usecase_input_validation()
- test_usecase_business_rule_enforcement()
- test_usecase_error_propagation()
- test_usecase_transaction_handling()
```

#### 4. Third-Party API Error Handling

**Missing Tests:**
```rust
// backend/src/third_party/*_tests.rs
- test_bgg_api_timeout()
- test_bgg_api_rate_limiting()
- test_bgg_api_invalid_response()
- test_google_places_api_failure()
- test_google_timezone_api_failure()
- test_third_party_api_retry_logic()
```

#### 5. Input Validation Edge Cases

**Missing Tests:**
```rust
// backend/src/*/controller_tests.rs
- test_controller_oversized_payload()
- test_controller_malformed_json()
- test_controller_missing_required_fields()
- test_controller_invalid_field_types()
- test_controller_sql_injection_attempts()
- test_controller_xss_attempts()
```

---

### 🟢 Medium Priority - Additional Tests

#### 1. Performance Tests

**Missing Tests:**
```rust
// testing/tests/performance_tests.rs
- test_bulk_insert_performance()
- test_complex_query_performance()
- test_search_performance_large_datasets()
- test_concurrent_request_handling()
- test_memory_usage_under_load()
- test_database_connection_pooling()
```

#### 2. Migration Tests

**Missing Tests:**
```rust
// testing/tests/migration_tests.rs
- test_forward_migration()
- test_rollback_migration()
- test_migration_with_existing_data()
- test_migration_idempotency()
- test_migration_data_integrity()
```

#### 3. Security Tests

**Missing Tests:**
```rust
// testing/tests/security_tests.rs
- test_password_hashing_verification()
- test_session_token_generation()
- test_csrf_protection()
- test_rate_limiting()
- test_input_sanitization()
- test_authorization_bypass_attempts()
- test_privilege_escalation_attempts()
```

#### 4. Data Integrity Tests

**Missing Tests:**
```rust
// testing/tests/data_integrity_tests.rs
- test_foreign_key_constraints()
- test_unique_constraints()
- test_data_consistency()
- test_cascade_deletes()
- test_orphaned_record_prevention()
```

---

## Test Implementation Priority

### Phase 1: Critical Integration Tests (Weeks 1-2)
1. Complete CRUD operations for Venues, Games, Contests
2. Authentication & Authorization integration tests
3. Error handling integration tests
4. **Target**: 50+ new integration tests

### Phase 2: Critical E2E Tests (Weeks 3-4)
1. Complete user workflows
2. CRUD operations E2E
3. Search & filtering E2E
4. **Target**: 15+ new E2E tests

### Phase 3: High Priority Unit Tests (Week 5)
1. Middleware unit tests
2. Repository error handling
3. Use case validation
4. **Target**: 30+ new unit tests

### Phase 4: Medium Priority Tests (Week 6+)
1. Performance tests
2. Security tests
3. Migration tests
4. **Target**: 20+ new tests

---

## Expected Coverage Improvements

### Current State
- Unit Tests: 465 tests (~95% of test suite)
- Integration Tests: ~21 tests (~4% of test suite)
- E2E Tests: ~6 tests (~1% of test suite)

### Target State
- Unit Tests: 495+ tests (~60-70% of test suite)
- Integration Tests: 70+ tests (~20-30% of test suite)
- E2E Tests: 20+ tests (~5-10% of test suite)

### Coverage Metrics
- **Code Coverage**: Target 80%+ overall
- **API Endpoint Coverage**: Target 100% of endpoints
- **Critical Path Coverage**: Target 100% of user workflows

---

## Testing Best Practices

### Integration Tests
- Use `testcontainers` for all database dependencies
- Test both happy paths and error cases
- Test authentication/authorization for all protected endpoints
- Test input validation for all endpoints
- Test concurrent operations where applicable

### E2E Tests
- Test complete user workflows, not just individual pages
- Use realistic test data
- Test error scenarios (network failures, validation errors)
- Test cross-browser compatibility
- Use visual regression testing for UI consistency

### Unit Tests
- Test edge cases and boundary conditions
- Test error handling paths
- Test with invalid inputs
- Test concurrent access patterns where applicable
- Mock external dependencies

---

## Quick Reference: Missing Test Files

### Integration Tests to Create
- `testing/tests/venue_api_tests.rs`
- `testing/tests/game_api_tests.rs`
- `testing/tests/contest_api_tests.rs`
- `testing/tests/auth_integration_tests.rs`
- `testing/tests/error_handling_tests.rs`
- `testing/tests/analytics_api_tests.rs`
- `testing/tests/ratings_api_tests.rs`
- `testing/tests/client_analytics_api_tests.rs`
- `testing/tests/timezone_api_tests.rs`
- `testing/tests/search_integration_tests.rs`
- `testing/tests/concurrency_tests.rs`
- `testing/tests/edge_cases_tests.rs`

### E2E Tests to Create/Expand
- `testing/e2e/user_workflows.spec.ts`
- `testing/e2e/crud_operations.spec.ts` (expand existing)
- `testing/e2e/search.spec.ts`
- `testing/e2e/analytics.spec.ts` (expand existing)
- `testing/e2e/admin.spec.ts`
- `testing/e2e/error_handling.spec.ts` (expand existing)
- `testing/e2e/performance.spec.ts`

### Unit Tests to Expand
- `backend/src/middleware_tests.rs`
- Expand existing repository tests with error cases
- Expand existing usecase tests with validation
- Expand existing controller tests with edge cases

---

## Summary

**Total Recommended Tests**: ~115+ new tests
- Integration Tests: ~50 tests
- E2E Tests: ~20 tests
- Unit Tests: ~30 tests
- Performance/Security Tests: ~15 tests

**Priority Focus**: Integration and E2E tests (currently the weakest areas)

**Expected Impact**: 
- Improved confidence in production deployments
- Better error handling coverage
- Complete API endpoint coverage
- Full user workflow validation
- Better security posture

