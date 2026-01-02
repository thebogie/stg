# Testing Guide for STG RD

This document describes the testing setup and how to run tests for the STG RD project.

## 🧪 Test Overview

The project includes comprehensive testing for both frontend and backend components:

### Backend Tests
- **Glicko2 Algorithm Tests**: Unit tests for the rating calculation system
- **Integration Tests**: API endpoint and database operation tests
- **Unit Tests**: Individual component and function tests

### Frontend Tests
- **Component Tests**: Yew component rendering and interaction tests
- **Admin Page Tests**: Admin functionality and access control tests
- **Integration Tests**: Component integration and state management tests

## 🚀 Running Tests

### Quick Start
Use the provided script to run all tests:

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

Navigate to the frontend directory and run tests:

```bash
cd frontend

# Run all tests
wasm-pack test --headless --firefox

# Run tests in browser
wasm-pack test --firefox

# Run tests with output
wasm-pack test --headless --firefox -- --nocapture
```

## 📋 Test Categories

### 1. Glicko2 Algorithm Tests (`backend/src/ratings_tests.rs`)

Tests the core rating calculation algorithm:

- **Integration Workflow**: Complete rating update workflow
- **Rating Progression**: Multi-period rating changes
- **Edge Cases**: Boundary conditions and extreme values
- **Parameter Sensitivity**: How different parameters affect results
- **Weighted Games**: Game weight impact on ratings
- **Consistency**: Deterministic and reproducible results

### 2. Admin Page Tests (`frontend/src/pages/admin_test.rs`)

Tests the admin page functionality:

- **Access Control**: Admin-only access verification
- **Tab Navigation**: Admin dashboard tab switching
- **Component Rendering**: Admin page component creation
- **State Management**: Admin state and props handling

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

### Frontend Dependencies
```toml
[dev-dependencies]
wasm-bindgen-test = "0.3"
gloo-utils = "0.3"
web-sys = "0.3"
```

## 🔍 Test Coverage

### Backend Coverage
- **Glicko2 Algorithm**: 100% core algorithm coverage
- **Rating Repository**: Database operation coverage
- **Rating Usecase**: Business logic coverage
- **Rating Controller**: API endpoint coverage
- **Rating Scheduler**: Background task coverage

### Frontend Coverage
- **Admin Page**: Component rendering and state management
- **Navigation**: Admin link visibility and routing
- **Access Control**: Admin-only feature protection
- **Component Integration**: Cross-component communication

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

#### Frontend Tests
```
test result: ok. X passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## 🚨 Troubleshooting

### Common Issues

#### Backend Tests
1. **Database Connection**: Ensure ArangoDB is running
2. **Redis Connection**: Ensure Redis is running
3. **Dependencies**: Run `cargo clean && cargo build` if tests fail to compile

#### Frontend Tests
1. **WASM Build**: Run `wasm-pack build` before testing
2. **Browser**: Ensure Firefox is installed for headless testing
3. **Dependencies**: Run `cargo clean && cargo build` if tests fail to compile

### Debug Mode
Run tests with debug output:

```bash
# Backend
RUST_LOG=debug cargo test --verbose

# Frontend
RUST_LOG=debug wasm-pack test --headless --firefox -- --nocapture
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

### Frontend Test Structure
```rust
#[wasm_bindgen_test]
async fn test_component_name() {
    // Arrange
    let props = ComponentProps {};
    
    // Act
    let component = Component::new(props);
    
    // Assert
    assert!(component.props == ComponentProps {});
}
```

## 🔄 Continuous Integration

Tests are designed to run in CI/CD pipelines:

- **Backend**: Uses `cargo test` for Rust testing
- **Frontend**: Uses `wasm-pack test` for WASM testing
- **Integration**: End-to-end system testing
- **Coverage**: Test coverage reporting

## 📚 Additional Resources

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Yew Testing Documentation](https://yew.rs/docs/concepts/testing)
- [Actix Web Testing](https://actix.rs/docs/testing/)
- [WASM Testing Guide](https://rustwasm.github.io/docs/wasm-pack/tutorials/npm-browser-packages/template-deep-dive/testing.html)
