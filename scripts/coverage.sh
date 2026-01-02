#!/bin/bash
# Generate test coverage report and display summary

set -e

echo "📊 Generating test coverage report..."

# Create output directory
mkdir -p _build/coverage

# Generate LCOV coverage report
echo "Running tests with coverage..."
cargo llvm-cov nextest --workspace --lcov --output-path _build/coverage/lcov.info

# Generate HTML coverage report
echo "Generating HTML report..."
cargo llvm-cov nextest --workspace --html --output-dir _build/coverage/html

# Extract coverage percentage from LCOV file if available
if command -v lcov &> /dev/null; then
    echo ""
    echo "📈 Coverage Summary:"
    lcov --summary _build/coverage/lcov.info 2>/dev/null | grep -E "lines|functions|branches" || echo "Install 'lcov' for detailed summary"
else
    echo ""
    echo "💡 Install 'lcov' for detailed coverage summary:"
    echo "   Ubuntu/Debian: sudo apt-get install lcov"
    echo "   macOS: brew install lcov"
fi

echo ""
echo "✅ Coverage report generated:"
echo "   - HTML: _build/coverage/html/index.html"
echo "   - LCOV: _build/coverage/lcov.info"
echo ""
echo "Open the HTML report in your browser to see detailed coverage."

