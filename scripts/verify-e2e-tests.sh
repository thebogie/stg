#!/bin/bash
# Verify that E2E tests are actually testing real functionality
# This script checks for common false positive patterns in E2E tests

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

echo "🔍 Verifying E2E Tests for False Positives"
echo "=========================================="
echo ""

ISSUES_FOUND=0

# Check 1: Look for tests with weak assertions
echo "1. Checking for weak assertions (expect(true).toBeTruthy(), etc.)..."
WEAK_ASSERTIONS=$(grep -rn "expect(true)" testing/e2e/ || true)
if [ -n "$WEAK_ASSERTIONS" ]; then
    echo "   ⚠️  Found weak assertions:"
    echo "$WEAK_ASSERTIONS" | while IFS= read -r line; do
        echo "      $line"
    done
    ISSUES_FOUND=$((ISSUES_FOUND + 1))
else
    echo "   ✅ No weak assertions found"
fi
echo ""

# Check 2: Look for tests that only check visibility without functionality
echo "2. Checking for tests that only verify visibility..."
VISIBILITY_ONLY=$(grep -rn "toBeVisible" testing/e2e/ 2>/dev/null | grep -v "expect.*toBeVisible" | wc -l | tr -d ' \n' || echo "0")
VISIBILITY_ONLY=${VISIBILITY_ONLY:-0}
if [ "$VISIBILITY_ONLY" -gt 10 ]; then
    echo "   ⚠️  Many tests only check visibility (${VISIBILITY_ONLY} instances)"
    echo "      Consider adding functional checks (clicks, form submissions, etc.)"
    ISSUES_FOUND=$((ISSUES_FOUND + 1))
else
    echo "   ✅ Visibility checks are reasonable"
fi
echo ""

# Check 3: Look for tests that skip on failure
echo "3. Checking for tests that skip assertions on failure..."
SKIP_PATTERNS=$(grep -rn "if.*count.*> 0" testing/e2e/ | wc -l | tr -d ' ' || echo "0")
SKIP_PATTERNS=${SKIP_PATTERNS:-0}
if [ "$SKIP_PATTERNS" -gt 5 ]; then
    echo "   ⚠️  Found ${SKIP_PATTERNS} conditional checks that might skip assertions"
    echo "      These tests might pass even when functionality is broken"
    ISSUES_FOUND=$((ISSUES_FOUND + 1))
else
    echo "   ✅ Conditional checks are minimal"
fi
echo ""

# Check 4: Verify tests actually interact with the page
echo "4. Checking for interactive tests (clicks, form fills, etc.)..."
INTERACTIVE_TESTS=$(grep -rnE "\.click\(|\.fill\(|\.type\(|\.selectOption\(" testing/e2e/ | wc -l | tr -d ' ' || echo "0")
INTERACTIVE_TESTS=${INTERACTIVE_TESTS:-0}
if [ "$INTERACTIVE_TESTS" -lt 5 ]; then
    echo "   ⚠️  Only ${INTERACTIVE_TESTS} interactive actions found"
    echo "      Tests should click buttons, fill forms, and interact with the UI"
    ISSUES_FOUND=$((ISSUES_FOUND + 1))
else
    echo "   ✅ Found ${INTERACTIVE_TESTS} interactive actions - good!"
fi
echo ""

# Check 5: Verify tests check actual content, not just presence
echo "5. Checking for content verification (textContent, innerText, etc.)..."
CONTENT_CHECKS=$(grep -rnE "textContent|innerText|getAttribute|hasText" testing/e2e/ | wc -l | tr -d ' ' || echo "0")
CONTENT_CHECKS=${CONTENT_CHECKS:-0}
if [ "$CONTENT_CHECKS" -lt 10 ]; then
    echo "   ⚠️  Only ${CONTENT_CHECKS} content checks found"
    echo "      Tests should verify actual content, not just that elements exist"
    ISSUES_FOUND=$((ISSUES_FOUND + 1))
else
    echo "   ✅ Found ${CONTENT_CHECKS} content checks - good!"
fi
echo ""

# Check 6: Verify API calls are being made
echo "6. Checking for API interaction tests..."
API_TESTS=$(grep -rnE "api/|/api/|fetch|axios" testing/e2e/ 2>/dev/null | wc -l | tr -d ' \n' || echo "0")
API_TESTS=${API_TESTS:-0}
if [ "$API_TESTS" -eq 0 ]; then
    echo "   ℹ️  No explicit API checks found (this is OK if tests verify UI that calls APIs)"
else
    echo "   ✅ Found ${API_TESTS} API-related checks"
fi
echo ""

# Check 7: Check test execution times (too fast might indicate they're not running)
echo "7. Analyzing test execution times from last run..."
if [ -f "_build/test-results/e2e-results.xml" ]; then
    # Extract test times
    TIMES=$(grep -oE 'time="[0-9.]+"' _build/test-results/e2e-results.xml | grep -oE '[0-9.]+' || echo "")
    if [ -n "$TIMES" ]; then
        FAST_TESTS=$(echo "$TIMES" | awk '$1 < 0.5' | wc -l | tr -d ' ' || echo "0")
        FAST_TESTS=${FAST_TESTS:-0}
        if [ "$FAST_TESTS" -gt 5 ]; then
            echo "   ⚠️  Found ${FAST_TESTS} tests that completed in < 0.5s"
            echo "      These might be false positives (tests that don't wait for real interactions)"
            ISSUES_FOUND=$((ISSUES_FOUND + 1))
        else
            echo "   ✅ Test execution times look reasonable"
        fi
    fi
else
    echo "   ℹ️  No test results found - run tests first"
fi
echo ""

# Summary
echo "=========================================="
if [ "$ISSUES_FOUND" -eq 0 ]; then
    echo "✅ All checks passed! Tests appear to be validating real functionality."
    exit 0
else
    echo "⚠️  Found ${ISSUES_FOUND} potential issues"
    echo ""
    echo "Recommendations:"
    echo "  1. Review tests with weak assertions and add real checks"
    echo "  2. Add more interactive tests (clicks, form submissions)"
    echo "  3. Verify actual content, not just element presence"
    echo "  4. Check that tests wait for real user interactions"
    echo ""
    echo "To verify tests are actually running:"
    echo "  1. Check _build/test-results/playwright-output.log for detailed output"
    echo "  2. Review _build/playwright-report/index.html for test details"
    echo "  3. Manually verify one test by running: npx playwright test testing/e2e/example.spec.ts"
    exit 1
fi
