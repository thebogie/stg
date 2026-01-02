# Advanced Testing Guide

This document describes the comprehensive testing infrastructure we've built for the STG application, including test data factories, advanced scenarios, and performance testing.

## 🏗️ **Test Infrastructure Overview**

### **Test Data Factories** (`backend/tests/test_utils.rs`)

Our test data factories generate realistic, consistent test data for all application entities:

#### **TestDataFactory**
- **`create_game()`** - Generates realistic board games with names, descriptions, years, and BGG IDs
- **`create_venue()`** - Creates gaming venues with real city coordinates and addresses
- **`create_player()`** - Generates players with realistic names, emails, and handles
- **`create_contest()`** - Creates tournament contests with proper date/time handling
- **Bulk creation methods** - `create_games()`, `create_venues()`, `create_players()`, `create_contests()`

#### **TestUtils**
- **DTO Conversions** - Convert between models and DTOs
- **Search Queries** - Generate random search terms for testing
- **Data Validation** - Helper methods for data verification

#### **TestDbUtils**
- **Cleanup** - Remove test data by ID patterns
- **Counting** - Count test records in database
- **Database Operations** - Helper methods for database interactions

## 🧪 **Test Scenarios**

### **1. Basic Integration Tests** (`factory_integration_tests.rs`)

**Purpose**: Test the complete workflow from data creation to search functionality.

**What it tests**:
- ✅ Data creation using factories
- ✅ Database insertion operations
- ✅ Search functionality with real data
- ✅ Data consistency verification
- ✅ Proper cleanup procedures

**Usage**:
```bash
# Run basic integration tests
./scripts/run-integration-suite.sh

# Or using make
make test-integration
```

### **2. Performance Tests** (`advanced_test_scenarios.rs`)

**Purpose**: Test application performance with large datasets and complex operations.

**What it tests**:
- 🚀 **Bulk Insert Performance** - Insert 100 games, 50 venues, 200 players
- 🔍 **Search Performance** - Test search queries with timing measurements
- 🧮 **Complex Query Performance** - Multi-criteria queries with sorting and filtering
- 📊 **Performance Metrics** - Document insertion rates and query response times

**Key Metrics**:
- Insert rate (documents/second)
- Search response times
- Complex query performance
- Memory usage patterns

**Usage**:
```bash
# Run performance tests only
./scripts/run-advanced-tests.sh performance

# Or using make
make test-advanced
```

### **3. Edge Cases & Error Handling** (`advanced_test_scenarios.rs`)

**Purpose**: Test how the application handles unusual or invalid data.

**What it tests**:
- 🔄 **Duplicate ID Handling** - Test behavior with duplicate document IDs
- 📝 **Empty/Null Fields** - Test with missing or null data
- 📏 **Very Long Fields** - Test with extremely long text values
- 🔤 **Special Characters** - Test with emojis, HTML, and special chars
- 🔢 **Invalid Data Types** - Test with wrong data types

**Error Scenarios**:
- Database constraint violations
- Data validation failures
- Type conversion errors
- Memory overflow conditions

**Usage**:
```bash
# Run edge case tests only
./scripts/run-advanced-tests.sh edge-cases
```

### **4. Concurrent Operations** (`advanced_test_scenarios.rs`)

**Purpose**: Test application behavior under concurrent load.

**What it tests**:
- 📥 **Concurrent Inserts** - Multiple simultaneous document insertions
- 📖 **Concurrent Reads** - Multiple simultaneous search operations
- 🔄 **Mixed Read/Write** - Combined read and write operations
- ⚡ **Concurrency Metrics** - Success/failure rates under load

**Concurrency Patterns**:
- 10 concurrent write operations
- 20 concurrent read operations
- Mixed read/write scenarios
- Thread safety verification

**Usage**:
```bash
# Run concurrent tests only
./scripts/run-advanced-tests.sh concurrent
```

### **5. Business Logic Scenarios** (`advanced_test_scenarios.rs`)

**Purpose**: Test real-world business scenarios and data relationships.

**What it tests**:
- 🎯 **Game Categorization** - Strategy vs Party games filtering
- 📍 **Venue Proximity Search** - Geographic search with distance calculations
- 👥 **Player Activity Patterns** - Active vs inactive player analysis
- 📊 **Game Popularity Analysis** - Year-based game statistics
- 🏆 **Contest Creation** - End-to-end contest workflow

**Business Scenarios**:
- Game categorization and filtering
- Geographic venue search
- Player activity analysis
- Popularity metrics
- Contest management

**Usage**:
```bash
# Run business logic tests only
./scripts/run-advanced-tests.sh business-logic
```

### **6. Data Consistency & Integrity** (`advanced_test_scenarios.rs`)

**Purpose**: Ensure data integrity and consistency across operations.

**What it tests**:
- ✅ **Data Validation** - Verify inserted data matches original
- 🔐 **Unique Constraints** - Test duplicate handling
- 🔗 **Referential Integrity** - Test foreign key relationships
- 🔢 **Data Type Consistency** - Verify type preservation
- 💾 **Transaction Consistency** - Test multi-document operations

**Integrity Checks**:
- Data round-trip verification
- Constraint enforcement
- Type safety validation
- Transaction atomicity

**Usage**:
```bash
# Run consistency tests only
./scripts/run-advanced-tests.sh consistency
```

## 🚀 **Test Execution**

### **Quick Start**

```bash
# Run all test suites
./scripts/run-advanced-tests.sh

# Run specific test suite
./scripts/run-advanced-tests.sh performance
./scripts/run-advanced-tests.sh edge-cases
./scripts/run-advanced-tests.sh concurrent
./scripts/run-advanced-tests.sh business-logic
./scripts/run-advanced-tests.sh consistency

# Run using make
make test                    # All tests
make test-unit              # Unit tests only
make test-integration       # Integration tests only
make test-advanced          # Advanced tests only
```

### **Test Environment Setup**

The test runner automatically:
1. **Sets up test environment** - Docker containers for ArangoDB and Redis
2. **Loads test configuration** - Environment variables and database setup
3. **Runs test scenarios** - Executes specified test suites
4. **Cleans up** - Removes test data and containers

### **Environment Variables**

Required for integration and advanced tests:
```bash
ARANGO_URL=http://localhost:50011
ARANGO_USERNAME=test
ARANGO_PASSWORD=testpass
ARANGO_DB=stg_rd_test
REDIS_URL=redis://127.0.0.1:6380/
RUN_INTEGRATION_TESTS=1
```

## 📊 **Test Results & Metrics**

### **Performance Benchmarks**

Our performance tests measure:
- **Insert Rate**: Documents per second
- **Search Response Time**: Query execution time
- **Concurrent Throughput**: Operations per second under load
- **Memory Usage**: Resource consumption patterns

### **Success Criteria**

Tests are considered successful when:
- ✅ All assertions pass
- ✅ No unexpected errors occur
- ✅ Performance meets baseline expectations
- ✅ Data integrity is maintained
- ✅ Cleanup completes successfully

### **Test Reports**

The test runner provides:
- **Colored output** - Easy-to-read test results
- **Timing information** - Duration for each test suite
- **Success/failure counts** - Summary statistics
- **Detailed logs** - Verbose output for debugging

## 🔧 **Customization & Extension**

### **Adding New Test Scenarios**

1. **Create test function** in `advanced_test_scenarios.rs`:
```rust
#[actix_web::test]
#[ignore]
async fn test_your_scenario() {
    // Your test logic here
}
```

2. **Add to test runner** in `run-advanced-tests.sh`:
```bash
run_your_tests() {
    # Test setup and execution
}
```

3. **Update Makefile** with new target:
```makefile
test-your-scenario: check-nextest
    # Test execution logic
```

### **Custom Test Data**

Extend `TestDataFactory` with new methods:
```rust
impl TestDataFactory {
    pub fn create_custom_entity(id: Option<String>) -> CustomEntity {
        // Custom entity creation logic
    }
}
```

### **Performance Tuning**

Adjust test parameters in `advanced_test_scenarios.rs`:
- Dataset sizes (number of records)
- Concurrent operation counts
- Query complexity levels
- Performance thresholds

## 🐛 **Troubleshooting**

### **Common Issues**

1. **Test Environment Not Ready**
   ```bash
   # Ensure Docker is running
   docker ps
   
   # Check test environment setup
   ./scripts/setup-test-env.sh
   ```

2. **Database Connection Issues**
   ```bash
   # Verify ArangoDB is accessible
   curl http://localhost:50011/_api/version
   
   # Check environment variables
   source .env.test && env | grep ARANGO
   ```

3. **Test Timeouts**
   ```bash
   # Increase timeout for slow tests
   export RUST_TEST_THREADS=1
   cargo test --test advanced_test_scenarios -- --timeout 300
   ```

### **Debug Mode**

Run tests with verbose output:
```bash
# Verbose test output
RUST_LOG=debug cargo test --test advanced_test_scenarios -- --nocapture

# Single test debugging
cargo test --test advanced_test_scenarios test_performance_with_large_datasets -- --nocapture
```

## 📈 **Continuous Integration**

### **CI/CD Integration**

Our test suite is designed for CI/CD pipelines:

```yaml
# Example GitHub Actions workflow
- name: Run Advanced Tests
  run: |
    ./scripts/run-advanced-tests.sh
    make test-advanced
```

### **Performance Regression Testing**

Monitor performance over time:
```bash
# Run performance benchmarks
./scripts/run-advanced-tests.sh performance

# Compare with baseline
# (Implement performance comparison logic)
```

## 🎯 **Best Practices**

### **Test Development**

1. **Use Factories** - Always use `TestDataFactory` for consistent test data
2. **Clean Up** - Ensure proper cleanup in all tests
3. **Isolation** - Tests should be independent and not affect each other
4. **Realistic Data** - Use realistic test data that mirrors production
5. **Performance Awareness** - Consider test execution time and resource usage

### **Test Maintenance**

1. **Regular Updates** - Keep test data and scenarios current
2. **Performance Monitoring** - Track test execution times
3. **Coverage Analysis** - Ensure comprehensive test coverage
4. **Documentation** - Keep test documentation up to date

### **Production Readiness**

1. **Environment Parity** - Test environment should match production
2. **Data Volume** - Test with realistic data volumes
3. **Load Testing** - Include performance and load testing
4. **Error Scenarios** - Test error handling and edge cases

---

This comprehensive testing infrastructure ensures our application is robust, performant, and reliable across all scenarios. 