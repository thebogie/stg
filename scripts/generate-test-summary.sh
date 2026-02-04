#!/bin/bash
# Generate comprehensive test summary from all test result files
# This script parses JUnit XML files and generates a summary report

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

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

log_section() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}▶ $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

# Function to parse JUnit XML and extract test counts
parse_junit_xml() {
    local xml_file="$1"
    if [ ! -f "$xml_file" ]; then
        echo "0 0 0 0"  # tests failures errors skipped
        return
    fi
    
    # Extract test counts from XML
    local tests=$(grep -o 'tests="[0-9]*"' "$xml_file" | head -1 | grep -o '[0-9]*' || echo "0")
    local failures=$(grep -o 'failures="[0-9]*"' "$xml_file" | head -1 | grep -o '[0-9]*' || echo "0")
    local errors=$(grep -o 'errors="[0-9]*"' "$xml_file" | head -1 | grep -o '[0-9]*' || echo "0")
    local skipped=$(grep -o 'skipped="[0-9]*"' "$xml_file" | head -1 | grep -o '[0-9]*' || echo "0")
    
    echo "$tests $failures $errors $skipped"
}

log_section "📊 Comprehensive Test Summary"

TOTAL_TESTS=0
TOTAL_FAILURES=0
TOTAL_ERRORS=0
TOTAL_SKIPPED=0
HAS_FAILURES=false

# 1. Backend Unit Tests
log_info "Backend Unit Tests:"
if [ -f "_build/test-results/rust-unit-tests.xml" ]; then
    read tests failures errors skipped <<< "$(parse_junit_xml _build/test-results/rust-unit-tests.xml)"
    TOTAL_TESTS=$((TOTAL_TESTS + tests))
    TOTAL_FAILURES=$((TOTAL_FAILURES + failures))
    TOTAL_ERRORS=$((TOTAL_ERRORS + errors))
    TOTAL_SKIPPED=$((TOTAL_SKIPPED + skipped))
    
    if [ "$failures" -gt 0 ] || [ "$errors" -gt 0 ]; then
        HAS_FAILURES=true
        log_error "  ❌ Failed: $failures failures, $errors errors"
    else
        log_success "  ✅ Passed: $tests tests"
    fi
    log_info "    Total: $tests | Failures: $failures | Errors: $errors | Skipped: $skipped"
else
    log_warning "  ⚠️  No test results found: _build/test-results/rust-unit-tests.xml"
fi

# 2. E2E-style Integration Tests (from cargo nextest output)
log_info ""
log_info "E2E-style Integration Tests:"
log_info "  (These run via cargo nextest - check console output for results)"
log_info "  Test files:"
log_info "    - db_search_integration_test.rs"
log_info "    - venue_update_integration_test.rs"
log_info "    - contest_search_integration_test.rs"
log_info "    - cache_integration_test.rs"

# 3. Frontend E2E Tests (Playwright)
log_info ""
log_info "Frontend E2E Tests (Playwright):"
if [ -f "_build/test-results/e2e-results.xml" ]; then
    read tests failures errors skipped <<< "$(parse_junit_xml _build/test-results/e2e-results.xml)"
    TOTAL_TESTS=$((TOTAL_TESTS + tests))
    TOTAL_FAILURES=$((TOTAL_FAILURES + failures))
    TOTAL_ERRORS=$((TOTAL_ERRORS + errors))
    TOTAL_SKIPPED=$((TOTAL_SKIPPED + skipped))
    
    if [ "$failures" -gt 0 ] || [ "$errors" -gt 0 ]; then
        HAS_FAILURES=true
        log_error "  ❌ Failed: $failures failures, $errors errors"
        
        # Check if failures are visual regression only
        if grep -qiE "toHaveScreenshot|visual.*snapshot" "_build/test-results/e2e-results.xml" 2>/dev/null; then
            log_warning "  ⚠️  Failures appear to be visual regression tests (non-blocking)"
        fi
    else
        log_success "  ✅ Passed: $tests tests"
    fi
    log_info "    Total: $tests | Failures: $failures | Errors: $errors | Skipped: $skipped"
else
    log_warning "  ⚠️  No test results found: _build/test-results/e2e-results.xml"
fi

# 4. E2E API Tests (the new ones we added)
log_info ""
log_info "E2E API Tests (Rust-based, require BACKEND_URL):"
if [ -f "_build/test-results/rust-e2e-api-tests.xml" ]; then
    read tests failures errors skipped <<< "$(parse_junit_xml _build/test-results/rust-e2e-api-tests.xml)"
    TOTAL_TESTS=$((TOTAL_TESTS + tests))
    TOTAL_FAILURES=$((TOTAL_FAILURES + failures))
    TOTAL_ERRORS=$((TOTAL_ERRORS + errors))
    TOTAL_SKIPPED=$((TOTAL_SKIPPED + skipped))
    
    if [ "$failures" -gt 0 ] || [ "$errors" -gt 0 ]; then
        HAS_FAILURES=true
        log_error "  ❌ Failed: $failures failures, $errors errors"
    else
        log_success "  ✅ Passed: $tests tests"
    fi
    log_info "    Total: $tests | Failures: $failures | Errors: $errors | Skipped: $skipped"
else
    log_warning "  ⚠️  No test results found: _build/test-results/rust-e2e-api-tests.xml"
    log_info "  Test files:"
    log_info "    - contest_filters_e2e.rs"
    log_info "    - venue_search_e2e.rs"
    log_info "    - game_search_e2e.rs"
    log_info "    - player_search_e2e.rs"
    log_info "    - auth_flows_e2e.rs"
    log_info "    - crud_operations_e2e.rs"
    log_info "  Note: These tests require BACKEND_URL and will skip if not set"
fi

# Final Summary
log_section "📈 Final Summary"

if [ "$TOTAL_TESTS" -eq 0 ]; then
    log_warning "⚠️  No test results found!"
    log_info "Make sure tests have been run and result files exist in _build/test-results/"
    exit 1
fi

log_info "Total Tests Run: $TOTAL_TESTS"
log_info "Total Failures: $TOTAL_FAILURES"
log_info "Total Errors: $TOTAL_ERRORS"
log_info "Total Skipped: $TOTAL_SKIPPED"

if [ "$HAS_FAILURES" = true ]; then
    log_error ""
    log_error "❌ Some tests failed!"
    exit 1
else
    log_success ""
    log_success "✅ All tests passed!"
    exit 0
fi
