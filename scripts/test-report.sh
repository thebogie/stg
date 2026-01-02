#!/bin/bash
# Generate comprehensive test report

set -e

echo "🧪 Running Test Suite and Generating Report..."
echo ""

# Create output directory
mkdir -p _build/test-results

# Run tests with JUnit XML output
echo "📋 Running unit and integration tests..."
cargo nextest run --workspace --lib --tests --junit-xml _build/test-results/rust-tests.xml || TEST_EXIT=$?

# Run E2E tests if available
if command -v npx &> /dev/null; then
    echo ""
    echo "🌐 Running E2E tests..."
    npx playwright test --reporter=junit --output-dir=_build/test-results/e2e || E2E_EXIT=$?
fi

echo ""
echo "📊 Test Summary:"
echo "================"

# Count test results from JUnit XML if available
if [ -f "_build/test-results/rust-tests.xml" ]; then
    echo "Rust Tests:"
    # Extract test counts (basic parsing)
    TESTS=$(grep -o 'tests="[0-9]*"' _build/test-results/rust-tests.xml | head -1 | grep -o '[0-9]*' || echo "0")
    FAILURES=$(grep -o 'failures="[0-9]*"' _build/test-results/rust-tests.xml | head -1 | grep -o '[0-9]*' || echo "0")
    ERRORS=$(grep -o 'errors="[0-9]*"' _build/test-results/rust-tests.xml | head -1 | grep -o '[0-9]*' || echo "0")
    
    echo "  Total: $TESTS"
    echo "  Failures: $FAILURES"
    echo "  Errors: $ERRORS"
    
    if [ "$FAILURES" -gt 0 ] || [ "$ERRORS" -gt 0 ]; then
        echo "  ⚠️  Some tests failed!"
    else
        echo "  ✅ All tests passed!"
    fi
fi

if [ -f "_build/test-results/e2e/e2e-results.xml" ]; then
    echo ""
    echo "E2E Tests:"
    E2E_TESTS=$(grep -o 'tests="[0-9]*"' _build/test-results/e2e/e2e-results.xml | head -1 | grep -o '[0-9]*' || echo "0")
    E2E_FAILURES=$(grep -o 'failures="[0-9]*"' _build/test-results/e2e/e2e-results.xml | head -1 | grep -o '[0-9]*' || echo "0")
    
    echo "  Total: $E2E_TESTS"
    echo "  Failures: $E2E_FAILURES"
    
    if [ "$E2E_FAILURES" -gt 0 ]; then
        echo "  ⚠️  Some E2E tests failed!"
    else
        echo "  ✅ All E2E tests passed!"
    fi
fi

echo ""
echo "📁 Test Reports:"
echo "  - Rust JUnit XML: _build/test-results/rust-tests.xml"
if [ -f "_build/test-results/e2e/e2e-results.xml" ]; then
    echo "  - E2E JUnit XML: _build/test-results/e2e/e2e-results.xml"
fi
if [ -d "_build/playwright-report" ]; then
    echo "  - Playwright HTML: _build/playwright-report/index.html"
fi

# Exit with error if any tests failed
if [ "${TEST_EXIT:-0}" -ne 0 ] || [ "${E2E_EXIT:-0}" -ne 0 ]; then
    exit 1
fi

