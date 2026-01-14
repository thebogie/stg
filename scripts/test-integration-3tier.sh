#!/bin/bash

# 3-Tier Integration Test Runner
# Tier 1: Run all tests with 4 threads (fast)
# Tier 2: Retry failures with 2 threads (moderate)
# Tier 3: Run slow/failing tests sequentially (1 thread, most conservative)
# Usage: ./scripts/test-integration-3tier.sh

# Don't use set -e here - we want to handle errors manually
# set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
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

log_tier() {
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}▶ $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

# Known slow tests that should run in Tier 3 (sequential)
# These tests consistently take a long time or are prone to resource contention
SLOW_TESTS=(
    "venue_api_tests::test_delete_venue_not_found"
    "venue_api_tests::test_get_venue_not_found"
    "venue_api_tests::test_update_venue_not_found"
    "venue_integration_tests::test_delete_venue"
    "search_integration_tests::test_search_games"
)

# Extract test file and name from full test path
extract_test_parts() {
    local full_test="$1"
    # Format: test_file::test_name or test_file test_name
    if [[ "$full_test" == *"::"* ]]; then
        # Use awk or sed to split on :: since cut only accepts single character
        TEST_FILE=$(echo "$full_test" | awk -F'::' '{print $1}')
        TEST_NAME=$(echo "$full_test" | awk -F'::' '{print $2}')
    else
        # Try to extract from space-separated format
        TEST_FILE=$(echo "$full_test" | awk '{print $1}')
        TEST_NAME=$(echo "$full_test" | awk '{print $2}')
    fi
}

# ============================================================================
# TIER 1: Fast Run (4 threads) - All tests
# ============================================================================
log_tier "TIER 1: Running all tests with 4 threads"

TIER1_OUTPUT="/tmp/nextest-tier1.log"
TIER1_SUCCESS=true

# Run all tests with 4 threads
if cargo nextest run --package testing --test '*' --test-threads 4 --no-fail-fast 2>&1 | tee "$TIER1_OUTPUT"; then
    log_success "All tests passed in Tier 1!"
    # Still run Tier 3 for known slow tests to ensure they pass
    TIER1_SUCCESS=true
else
    TIER1_SUCCESS=false
fi

# Extract failed tests from Tier 1
FAILED_TESTS=""
if [ "$TIER1_SUCCESS" = false ]; then
    log_warning "Some tests failed in Tier 1. Extracting failed test names..."
    FAILED_TESTS=$(grep -E "^\s+FAIL.*testing::" "$TIER1_OUTPUT" | \
        sed -E 's/.*testing::([^ ]+) ([^ ]+).*/\1::\2/' | \
        sort -u | tr '\n' ' ' || true)
    
    if [ -n "$FAILED_TESTS" ] && [ "$FAILED_TESTS" != " " ]; then
        log_info "Failed tests from Tier 1: $FAILED_TESTS"
    fi
else
    log_info "All tests passed in Tier 1, but will still verify slow tests in Tier 3"
fi

# ============================================================================
# TIER 2: Moderate Retry (2 threads) - Failed tests from Tier 1
# ============================================================================
TIER2_FAILED=""
if [ -n "$FAILED_TESTS" ] && [ "$FAILED_TESTS" != " " ]; then
    log_tier "TIER 2: Retrying failed tests with 2 threads"
    
    for test in $FAILED_TESTS; do
        extract_test_parts "$test"
        log_info "Retrying: $test"
        
        if ! cargo nextest run --package testing --test "$TEST_FILE" --test-threads 2 -- "$TEST_NAME" 2>&1; then
            log_warning "Test $test still failing with 2 threads, will retry in Tier 3"
            TIER2_FAILED="$TIER2_FAILED $test"
        else
            log_success "Test $test passed on Tier 2 retry!"
        fi
    done
fi

# ============================================================================
# TIER 3: Sequential Run (1 thread) - Slow tests + Tier 2 failures
# ============================================================================
log_tier "TIER 3: Running slow tests and Tier 2 failures sequentially (1 thread)"

# Build list of Tier 3 tests: known slow tests + Tier 2 failures
TIER3_TESTS_LIST=()

# Add known slow tests
for slow_test in "${SLOW_TESTS[@]}"; do
    TIER3_TESTS_LIST+=("$slow_test")
done

# Add Tier 2 failures (avoid duplicates)
for test in $TIER2_FAILED; do
    # Check if not already in list
    if [[ ! " ${TIER3_TESTS_LIST[@]} " =~ " ${test} " ]]; then
        TIER3_TESTS_LIST+=("$test")
    fi
done

TIER3_SUCCESS=true

if [ ${#TIER3_TESTS_LIST[@]} -gt 0 ]; then
    log_info "Running ${#TIER3_TESTS_LIST[@]} tests sequentially in Tier 3"
    
    for test in "${TIER3_TESTS_LIST[@]}"; do
        extract_test_parts "$test"
        log_info "Running sequentially: $test"
        
        if ! cargo nextest run --package testing --test "$TEST_FILE" --test-threads 1 -- "$TEST_NAME" 2>&1; then
            log_error "Test $test failed even with sequential execution"
            TIER3_SUCCESS=false
        else
            log_success "Test $test passed with sequential execution!"
        fi
    done
else
    log_info "No tests to run in Tier 3"
fi

# ============================================================================
# Final Summary
# ============================================================================
echo ""
log_tier "Test Execution Summary"

FINAL_SUCCESS=true

if [ "$TIER1_SUCCESS" = false ]; then
    log_error "Some tests failed in Tier 1"
    FINAL_SUCCESS=false
fi

if [ -n "$TIER2_FAILED" ] && [ "$TIER2_FAILED" != " " ]; then
    log_warning "Some tests failed in Tier 2: $TIER2_FAILED"
    FINAL_SUCCESS=false
fi

if [ "$TIER3_SUCCESS" = false ]; then
    log_error "Some tests failed in Tier 3"
    FINAL_SUCCESS=false
fi

if [ "$FINAL_SUCCESS" = true ]; then
    log_success "All tests passed across all tiers! ✅"
    exit 0
else
    log_error "Some tests failed. See details above."
    exit 1
fi
