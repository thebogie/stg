#!/bin/bash
# Quick test to verify E2E tests catch real bugs (not false positives)
# This temporarily breaks functionality and verifies tests fail

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

echo "🧪 Testing if E2E tests catch real bugs..."
echo ""
echo "This will:"
echo "  1. Make a small change to break filtering"
echo "  2. Run the contest filter tests"
echo "  3. Verify they FAIL (proving they're not false positives)"
echo "  4. Restore the code"
echo ""
read -p "Continue? (y/n) " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled"
    exit 0
fi

# Find the filter logic in contest repository
FILTER_FILE="backend/src/contest/repository.rs"
BACKUP_FILE="${FILTER_FILE}.backup"

# Create backup
cp "$FILTER_FILE" "$BACKUP_FILE"
echo "✅ Created backup: $BACKUP_FILE"

# Find and comment out the player filter
echo "🔧 Temporarily breaking player filter..."
sed -i 's/if filter_player_full.is_some() {/\/\/ TEMP DISABLED FOR TEST: if filter_player_full.is_some() {/' "$FILTER_FILE" || true
sed -i 's/filters.push("LENGTH(FOR r IN resulted_in FILTER r._from == contest._id AND r._to == @filter_player_id RETURN 1) > 0".to_string());/\/\/ TEMP DISABLED: filters.push("LENGTH(FOR r IN resulted_in FILTER r._from == contest._id AND r._to == @filter_player_id RETURN 1) > 0".to_string());/' "$FILTER_FILE" || true

echo "✅ Filter temporarily disabled"
echo ""
echo "Running contest filter tests (they should FAIL)..."
echo ""

# Run the tests
if cargo nextest run --package testing --test 'contest_filters_e2e' 2>&1 | tee /tmp/false-positive-test.log; then
    echo ""
    echo "❌ PROBLEM: Tests PASSED even though filter is broken!"
    echo "   This indicates FALSE POSITIVES - tests aren't catching real bugs"
    echo ""
    echo "   Check the test assertions - they may only be checking HTTP status"
    echo "   instead of actual filtering behavior"
else
    echo ""
    echo "✅ GOOD: Tests FAILED when filter was broken"
    echo "   This proves tests are NOT false positives - they catch real bugs!"
fi

# Restore backup
echo ""
echo "🔧 Restoring original code..."
mv "$BACKUP_FILE" "$FILTER_FILE"
echo "✅ Code restored"

echo ""
echo "📊 Review the test output above to see what failed"
echo "   If tests failed → they're working correctly (not false positives)"
echo "   If tests passed → they're false positives (need better assertions)"
