#!/bin/bash

# Integration test runner with smart retry logic
# Runs tests with higher parallelism first, then retries failures with lower parallelism
# Usage: ./scripts/test-integration-with-retry.sh

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

# Phase 1: Run with moderate parallelism (4 threads)
log_info "Phase 1: Running integration tests with 4 threads..."
if cargo nextest run --package testing --test '*' --test-threads 4 --no-fail-fast 2>&1 | tee /tmp/nextest-output.log; then
    log_success "All tests passed on first attempt!"
    exit 0
fi

# Extract failed tests from the output
log_warning "Some tests failed. Extracting failed test names..."
# Extract test names from lines like: "FAIL [ 465.438s] testing::error_handling_tests test_401_unauthorized_invalid_session"
# Pattern: FAIL [time] testing::test_file test_name
FAILED_TESTS=$(grep -E "^\s+FAIL.*testing::" /tmp/nextest-output.log | \
    sed -E 's/.*testing::([^ ]+) ([^ ]+).*/\1::\2/' | \
    sort -u | tr '\n' ' ' || true)

# If that didn't work, try extracting from the failures section
if [ -z "$FAILED_TESTS" ] || [ "$FAILED_TESTS" = " " ]; then
    log_info "Trying alternative extraction method..."
    # Look for lines after "failures:" that contain test names
    FAILED_TESTS=$(awk '/failures:/{flag=1; next} flag && /testing::/{print; flag=0}' /tmp/nextest-output.log | \
        sed -E 's/.*testing::([^ ]+) ([^ ]+).*/\1::\2/' | \
        sort -u | tr '\n' ' ' || true)
fi

# Last resort: extract from any line mentioning "testing::" and a test name
if [ -z "$FAILED_TESTS" ] || [ "$FAILED_TESTS" = " " ]; then
    log_info "Trying final extraction method..."
    FAILED_TESTS=$(grep -E "testing::.*test_" /tmp/nextest-output.log | \
        sed -E 's/.*testing::([^ ]+) (test_[^ ]+).*/\1::\2/' | \
        sort -u | tr '\n' ' ' || true)
fi

if [ -z "$FAILED_TESTS" ]; then
    log_error "Could not extract failed test names from output"
    log_info "Full output saved to /tmp/nextest-output.log"
    exit 1
fi

log_info "Failed tests: $FAILED_TESTS"

# Phase 2: Retry failed tests with lower parallelism (2 threads)
log_info "Phase 2: Retrying failed tests with 2 threads (more conservative)..."
RETRY_SUCCESS=true

# Retry each failed test individually with lower parallelism
for test in $FAILED_TESTS; do
    log_info "Retrying: $test"
    # Extract test file and test name (format: test_file::test_name)
    TEST_FILE=$(echo "$test" | cut -d'::' -f1)
    TEST_NAME=$(echo "$test" | cut -d'::' -f2)
    
    if ! cargo nextest run --package testing --test "$TEST_FILE" --test-threads 2 -- "$TEST_NAME" 2>&1; then
        log_warning "Test $test still failing with 2 threads, trying with 1 thread..."
        # Last resort: try with single thread
        if ! cargo nextest run --package testing --test "$TEST_FILE" --test-threads 1 -- "$TEST_NAME" 2>&1; then
            log_error "Test $test still failing after retry"
            RETRY_SUCCESS=false
        else
            log_success "Test $test passed with 1 thread!"
        fi
    else
        log_success "Test $test passed on retry with 2 threads!"
    fi
done

if [ "$RETRY_SUCCESS" = true ]; then
    log_success "All failed tests passed on retry!"
    exit 0
else
    log_error "Some tests still failing after retry"
    exit 1
fi
