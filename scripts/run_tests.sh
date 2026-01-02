#!/bin/bash

echo "🧪 Running STG RD Tests"
echo "========================"

echo ""
echo "🔧 Backend Tests"
echo "----------------"
cd backend
echo "Running Glicko2 algorithm tests..."
cargo test ratings_tests::glicko_tests --lib --verbose

echo ""
echo "Running all backend tests..."
cargo test --verbose

cd ..

echo ""
echo "🎨 Frontend Tests"
echo "-----------------"
cd frontend
echo "Running frontend tests..."
wasm-pack test --headless --firefox

cd ..

echo ""
echo "✅ All tests completed!"
echo ""
echo "📊 Test Summary:"
echo "- Backend: Glicko2 algorithm tests, integration tests"
echo "- Frontend: Component tests, admin page tests"
echo ""
echo "💡 To run specific test suites:"
echo "  Backend: cd backend && cargo test <test_name>"
echo "  Frontend: cd frontend && wasm-pack test --headless --firefox"
