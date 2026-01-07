# Testing Guide for STG RD

This document describes the testing setup and how to run tests for the STG RD project.

## 🧪 Test Overview

The project includes comprehensive testing for both frontend and backend components:

### Backend Tests
- **Glicko2 Algorithm Tests**: Unit tests for the rating calculation system
- **Integration Tests**: API endpoint and database operation tests
- **Unit Tests**: Individual component and function tests

### Frontend Tests
- **E2E Tests**: End-to-end tests using Playwright that test the complete application in real browsers
- **Note**: WASM unit tests are not supported in WSL2/headless environments, so E2E tests are the primary frontend testing method

## 🚀 Running Tests

### Quick Start

**⚠️ Important:** The `run_tests.sh` script runs tests directly on your host machine (not in Docker containers). It tests your local code before building production images.

**To test production Docker containers:**
```bash
# 1. Build production images
./scripts/build-prod-images.sh

# 2. Test production containers
./scripts/test-prod-containers.sh --run-tests
```

**For unit tests on host machine:**
```bash
./scripts/run_tests.sh
```

### Backend Tests

Navigate to the backend directory and run tests:

```bash
cd backend

# Run all tests
cargo test

# Run specific test modules
cargo test ratings_tests::glicko_tests --lib --verbose

# Run tests with output
cargo test --verbose

# Run tests in watch mode (requires cargo-watch)
cargo watch -x test
```

### Frontend Tests

Frontend testing is done via E2E tests using Playwright:

```bash
# Run E2E tests (recommended - uses Docker containers)
just test-frontend-e2e

# Or directly with Playwright
npx playwright test
```

**Note**: WASM unit tests (`wasm-pack test`) are not supported in WSL2/headless environments. E2E tests provide comprehensive coverage of frontend functionality.

## 📋 Test Categories

### 1. Glicko2 Algorithm Tests (`backend/src/ratings_tests.rs`)

Tests the core rating calculation algorithm:

- **Integration Workflow**: Complete rating update workflow
- **Rating Progression**: Multi-period rating changes
- **Edge Cases**: Boundary conditions and extreme values
- **Parameter Sensitivity**: How different parameters affect results
- **Weighted Games**: Game weight impact on ratings
- **Consistency**: Deterministic and reproducible results

### 2. Frontend E2E Tests (`testing/e2e/*.spec.ts`)

Tests the complete frontend application:

- **User Workflows**: Complete user interactions from start to finish
- **Page Rendering**: All pages load and display correctly
- **Navigation**: Routing and navigation between pages
- **Integration**: Frontend-backend API integration
- **Visual Regression**: Screenshot comparisons for UI consistency

### 3. Integration Tests (`backend/tests/ratings_integration_test.rs`)

Tests the complete system integration:

- **API Endpoints**: Ratings API functionality
- **Database Operations**: Repository and usecase operations
- **Scheduler Operations**: Background task scheduling
- **Error Handling**: System error scenarios

## 🛠️ Test Dependencies

### Backend Dependencies
```toml
[dev-dependencies]
tokio-test = { workspace = true }
mockall = { workspace = true }
test-log = "0.2"
pretty_assertions = "1.4"
insta = { workspace = true }
fake = "2.9"
rstest = "0.18"
test-case = "3.3"
proptest = "1.4"
approx = "0.5"  # For floating-point comparisons
```

### Frontend E2E Dependencies
```json
{
  "devDependencies": {
    "@playwright/test": "^1.40.0",
    "playwright": "^1.40.0"
  }
}
```

## 🔍 Test Coverage

### Backend Coverage
- **Glicko2 Algorithm**: 100% core algorithm coverage
- **Rating Repository**: Database operation coverage
- **Rating Usecase**: Business logic coverage
- **Rating Controller**: API endpoint coverage
- **Rating Scheduler**: Background task coverage

### Frontend Coverage
- **E2E Tests**: Complete user workflows and page interactions
- **Visual Regression**: Screenshot-based UI testing
- **API Integration**: Frontend-backend communication
- **Browser Compatibility**: Tests run in Chrome, Firefox, and Safari

## 📊 Test Results

### Expected Test Output

#### Backend Tests
```
running 6 tests
test ratings_tests::glicko_tests::test_glicko2_consistency ... ok
test ratings_tests::glicko_tests::test_glicko2_edge_cases ... ok
test ratings_tests::glicko_tests::test_glicko2_integration_workflow ... ok
test ratings_tests::glicko_tests::test_glicko2_parameter_sensitivity ... ok
test ratings_tests::glicko_tests::test_glicko2_rating_progression ... ok
test ratings_tests::glicko_tests::test_glicko2_weighted_games ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

#### Frontend E2E Tests
```
Running 10 tests using 3 workers
  ✓ tests/example.spec.ts:5:3 › Homepage loads correctly (1.2s)
  ✓ tests/example.spec.ts:12:3 › Navigation displays correctly (0.8s)
  
  10 passed (15.3s)
```

## 🚨 Troubleshooting

### Common Issues

#### Backend Tests
1. **Database Connection**: Ensure ArangoDB is running
2. **Redis Connection**: Ensure Redis is running
3. **Dependencies**: Run `cargo clean && cargo build` if tests fail to compile

#### Frontend E2E Tests
1. **Docker**: Ensure Docker is running (E2E tests use Docker containers)
2. **Playwright**: Run `npx playwright install` to install browser binaries
3. **Environment**: Ensure `config/.env.development` is configured

### Debug Mode
Run tests with debug output:

```bash
# Backend
RUST_LOG=debug cargo test --verbose

# Frontend E2E
npx playwright test --debug  # Opens Playwright Inspector
npx playwright test --ui     # Opens interactive UI mode
```

## 📝 Adding New Tests

### Backend Test Structure
```rust
#[test]
fn test_function_name() {
    // Arrange
    let input = "test_data";
    
    // Act
    let result = function_to_test(input);
    
    // Assert
    assert_eq!(result, expected_output);
}
```

### Frontend E2E Test Structure
```typescript
import { test, expect } from '@playwright/test';

test('component name', async ({ page }) => {
  // Navigate to page
  await page.goto('http://localhost:8080');
  
  // Assert element exists
  await expect(page.locator('h1')).toBeVisible();
  
  // Interact with elements
  await page.click('button');
  
  // Assert state changed
  await expect(page.locator('.result')).toContainText('Expected');
});
```

## 🔄 Continuous Integration

Tests are designed to run in CI/CD pipelines:

- **Backend**: Uses `cargo test` for Rust testing
- **Frontend**: Uses Playwright for E2E testing
- **Integration**: End-to-end system testing
- **Coverage**: Test coverage reporting

## 📚 Additional Resources

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Playwright Documentation](https://playwright.dev/)
- [Actix Web Testing](https://actix.rs/docs/testing/)
- [E2E Testing Guide](./E2E_TESTING_GUIDE.md)
