#!/bin/bash

# This script runs unit tests directly on the host machine (not in Docker containers).
# It tests your local code changes before building Docker images.
#
# To test production Docker containers:
#   1. Build production images: ./scripts/build-prod-images.sh
#   2. Test containers: ./scripts/test-prod-containers.sh --run-tests
#
# For E2E tests that use Docker (closer to production):
#   just test-frontend-e2e

echo "🧪 Running STG RD Tests"
echo "========================"
echo "⚠️  Note: These tests run on your host machine, NOT in production Docker containers"
echo ""

BACKEND_FAILED=0

echo ""
echo "🔧 Backend Tests"
echo "----------------"
cd backend
echo "Running Glicko2 algorithm tests..."
if ! cargo test ratings_tests::glicko_tests --lib --verbose; then
    echo "❌ Glicko2 algorithm tests failed!"
    BACKEND_FAILED=1
fi

echo ""
echo "Running all backend tests..."
if ! cargo test --verbose; then
    echo "❌ Backend tests failed!"
    BACKEND_FAILED=1
fi

cd ..

echo ""
if [ $BACKEND_FAILED -eq 0 ]; then
    echo "✅ All tests completed successfully!"
    echo ""
    echo "📊 Test Summary:"
    echo "- Backend: Glicko2 algorithm tests, integration tests"
else
    echo "⚠️  Test Summary:"
    echo "❌ Backend: Some tests failed"
fi

echo ""
echo "💡 To run specific test suites:"
echo "  Backend: cd backend && cargo test <test_name>"
echo "  Frontend E2E (recommended): just test-frontend-e2e"
echo ""
echo "Note: Frontend testing is done via E2E tests (just test-frontend-e2e)"
echo "      WASM unit tests are not supported in WSL2/headless environments"

# Exit with error if backend tests failed
if [ $BACKEND_FAILED -ne 0 ]; then
    exit 1
fi
exit 0
