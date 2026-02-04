#!/bin/bash
# Verify that E2E API tests are actually testing real functionality
# This script checks for common false positive patterns in E2E API tests

set -euo pipefail

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

log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }
log_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
log_error() { echo -e "${RED}❌ $1${NC}"; }
log_section() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}▶ $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

log_section "🔍 Verifying E2E API Tests for False Positives"

ISSUES_FOUND=0

# Check 1: Look for tests with weak assertions (only checking HTTP status)
log_info "1. Checking for weak assertions (only HTTP status checks)..."
STATUS_ONLY=$(grep -rn "res.status().is_success()" testing/tests/*_e2e.rs | grep -v "assert!" | wc -l | tr -d ' \n' || echo "0")
STATUS_ONLY=${STATUS_ONLY:-0}
CONTENT_CHECKS=$(grep -rnE "assert_eq!|assert!.*\.len\(\)|assert!.*\.contains\(|assert!.*==|assert!.*!=" testing/tests/*_e2e.rs | wc -l | tr -d ' \n' || echo "0")
CONTENT_CHECKS=${CONTENT_CHECKS:-0}

if [ "$CONTENT_CHECKS" -lt 10 ]; then
    log_warning "   ⚠️  Only ${CONTENT_CHECKS} content/data assertions found"
    log_info "      Tests should verify actual data, not just HTTP status codes"
    ISSUES_FOUND=$((ISSUES_FOUND + 1))
else
    log_success "   ✅ Found ${CONTENT_CHECKS} content/data assertions - good!"
fi
echo ""

# Check 2: Verify tests check actual filtering behavior
log_info "2. Checking for filtering behavior verification..."
FILTER_CHECKS=$(grep -rnE "filtered.*<=.*all|filtered.*<.*all|should be.*subset|actually.*filter" testing/tests/*_e2e.rs -i | wc -l | tr -d ' \n' || echo "0")
FILTER_CHECKS=${FILTER_CHECKS:-0}
if [ "$FILTER_CHECKS" -lt 3 ]; then
    log_warning "   ⚠️  Only ${FILTER_CHECKS} filtering behavior checks found"
    log_info "      Search/filter tests should verify results are actually filtered"
    ISSUES_FOUND=$((ISSUES_FOUND + 1))
else
    log_success "   ✅ Found ${FILTER_CHECKS} filtering behavior checks - good!"
fi
echo ""

# Check 3: Verify tests check actual data content
log_info "3. Checking for data content verification..."
DATA_CHECKS=$(grep -rnE "assert_eq!.*\.id|assert_eq!.*\.name|assert_eq!.*\.email|assert_eq!.*\.display_name|matches.*search_term" testing/tests/*_e2e.rs | wc -l | tr -d ' \n' || echo "0")
DATA_CHECKS=${DATA_CHECKS:-0}
if [ "$DATA_CHECKS" -lt 5 ]; then
    log_warning "   ⚠️  Only ${DATA_CHECKS} data content checks found"
    log_info "      Tests should verify actual field values, not just structure"
    ISSUES_FOUND=$((ISSUES_FOUND + 1))
else
    log_success "   ✅ Found ${DATA_CHECKS} data content checks - good!"
fi
echo ""

# Check 4: Verify tests use real API endpoints (not mocks)
log_info "4. Checking for real API endpoint usage..."
API_ENDPOINTS=$(grep -rnE "/api/(players|contests|venues|games)" testing/tests/*_e2e.rs | wc -l | tr -d ' \n' || echo "0")
API_ENDPOINTS=${API_ENDPOINTS:-0}
if [ "$API_ENDPOINTS" -lt 10 ]; then
    log_warning "   ⚠️  Only ${API_ENDPOINTS} API endpoint calls found"
    log_info "      Tests should make real API calls to production backend"
    ISSUES_FOUND=$((ISSUES_FOUND + 1))
else
    log_success "   ✅ Found ${API_ENDPOINTS} API endpoint calls - good!"
fi
echo ""

# Check 5: Verify tests check error cases
log_info "5. Checking for error case testing..."
ERROR_CHECKS=$(grep -rnE "is_client_error|is_server_error|404|401|400|invalid|nonexistent|wrong" testing/tests/*_e2e.rs -i | wc -l | tr -d ' \n' || echo "0")
ERROR_CHECKS=${ERROR_CHECKS:-0}
if [ "$ERROR_CHECKS" -lt 3 ]; then
    log_warning "   ⚠️  Only ${ERROR_CHECKS} error case checks found"
    log_info "      Tests should verify error handling (404, 401, invalid input, etc.)"
    ISSUES_FOUND=$((ISSUES_FOUND + 1))
else
    log_success "   ✅ Found ${ERROR_CHECKS} error case checks - good!"
fi
echo ""

# Check 6: Verify tests use production data
log_info "6. Checking for production data usage..."
PROD_DATA=$(grep -rnE "test_env_with_prod_data|production.*data|prod.*snapshot" testing/tests/*_e2e.rs -i | wc -l | tr -d ' \n' || echo "0")
PROD_DATA=${PROD_DATA:-0}
if [ "$PROD_DATA" -gt 0 ]; then
    log_success "   ✅ Tests use production snapshot data - excellent!"
else
    log_info "   ℹ️  Tests may use test data (this is OK, but production data is better)"
fi
echo ""

# Check 7: Verify authentication is tested
log_info "7. Checking for authentication testing..."
AUTH_CHECKS=$(grep -rnE "get_authenticated_session|Authorization.*Bearer|session_id|login|logout" testing/tests/*_e2e.rs | wc -l | tr -d ' \n' || echo "0")
AUTH_CHECKS=${AUTH_CHECKS:-0}
if [ "$AUTH_CHECKS" -lt 5 ]; then
    log_warning "   ⚠️  Only ${AUTH_CHECKS} authentication-related checks found"
    log_info "      Tests should verify authentication flows"
    ISSUES_FOUND=$((ISSUES_FOUND + 1))
else
    log_success "   ✅ Found ${AUTH_CHECKS} authentication-related checks - good!"
fi
echo ""

# Check 8: Review actual test assertions
log_info "8. Sample of actual assertions in tests..."
log_info "   Reviewing key test files for assertion quality..."
echo ""
log_info "   Contest Filters Tests:"
CONTEST_ASSERTIONS=$(grep -n "assert" testing/tests/contest_filters_e2e.rs | head -5 | sed 's/^/      /' || echo "      (none found)")
echo "$CONTEST_ASSERTIONS"
echo ""
log_info "   Auth Tests:"
AUTH_ASSERTIONS=$(grep -n "assert" testing/tests/auth_flows_e2e.rs | head -5 | sed 's/^/      /' || echo "      (none found)")
echo "$AUTH_ASSERTIONS"
echo ""

log_section "📊 Summary"

if [ "$ISSUES_FOUND" -eq 0 ]; then
    log_success "✅ All checks passed! Tests appear to be validating real functionality."
    echo ""
    log_info "To manually verify tests catch real bugs:"
    log_info "  1. Temporarily break a filter (e.g., comment out filter logic in backend)"
    log_info "  2. Run tests: cargo nextest run --package testing --test '*_e2e'"
    log_info "  3. Tests should FAIL - if they still pass, they're false positives"
    log_info "  4. Restore the code and verify tests pass again"
    echo ""
    exit 0
else
    log_warning "⚠️  Found ${ISSUES_FOUND} potential issues"
    echo ""
    log_info "Recommendations:"
    log_info "  1. Add more data content assertions (verify actual field values)"
    log_info "  2. Verify filtering actually reduces result sets"
    log_info "  3. Test error cases (404, 401, invalid input)"
    log_info "  4. Ensure tests use real API endpoints (not mocks)"
    echo ""
    log_info "To manually verify tests catch real bugs:"
    log_info "  1. Temporarily break functionality (e.g., disable a filter)"
    log_info "  2. Run tests and verify they FAIL"
    log_info "  3. If tests still pass, they're false positives"
    echo ""
    exit 1
fi
