# Modern Testing Architecture for Rust 2026
# Uses cargo-nextest, cargo-llvm-cov, testcontainers-rs, and Playwright

# Default recipe - run all tests
default:
    just test-all

# Run all tests with nextest
test-all:
    just test-backend
    just test-frontend-unit
    just test-frontend-e2e

# Run backend tests with nextest
test-backend:
    cargo nextest run --workspace --lib --tests

# Run backend tests with coverage
test-backend-coverage:
    cargo llvm-cov nextest --workspace --lcov --output-path _build/lcov.info
    cargo llvm-cov nextest --workspace --html --output-dir _build/coverage/html

# Run integration tests (uses testcontainers)
test-integration:
    cargo nextest run --package testing --test '*'

# Run frontend unit tests (wasm-bindgen-test)
test-frontend-unit:
    cd frontend && wasm-pack test --headless --firefox

# Run frontend E2E tests with Playwright
test-frontend-e2e:
    npx playwright test

# Generate coverage report (HTML)
coverage:
    cargo llvm-cov nextest --workspace --html --output-dir _build/coverage/html
    @echo "Coverage report generated at _build/coverage/html/index.html"

# Generate coverage report (LCOV for CI)
coverage-lcov:
    cargo llvm-cov nextest --workspace --lcov --output-path _build/lcov.info

# Generate JUnit XML for CI
# Note: JUnit XML is auto-generated based on .nextest.toml config
test-junit:
    cargo nextest run --workspace --lib --tests
    @echo "JUnit XML should be at: _build/test-results.xml"

# Run all tests and generate reports
test-full:
    just test-junit
    just coverage
    just test-frontend-e2e
    @echo "✅ Full test suite completed!"
    @echo "📊 Reports:"
    @echo "  - JUnit XML: _build/test-results.xml"
    @echo "  - Coverage: _build/coverage/html/index.html"

# Watch mode for backend tests
test-watch:
    cargo watch -x "nextest run --workspace --lib --tests"

# Run specific test pattern
test-pattern PATTERN:
    cargo nextest run --workspace --lib --tests --test-threads 1 -- PATTERN

# Clean test artifacts
clean:
    cargo clean
    rm -rf _build/
    npx playwright clean

# Setup: Install all required tools
setup:
    cargo install cargo-nextest cargo-llvm-cov
    npx playwright install --with-deps
    @echo "✅ All testing tools installed!"

# Export production data for sanitization
export-prod-data:
    @echo "Exporting production ArangoDB data..."
    @echo "⚠️  This requires ARANGO_URL, ARANGO_USERNAME, ARANGO_PASSWORD to be set"
    mkdir -p _build/dumps
    cargo run --package scripts --bin export_prod_data -- --output _build/dumps/dump.json

# Sanitize exported production data
sanitize-data INPUT="dump.json" OUTPUT="dump.sanitized.json.gz":
    cargo run --package scripts --bin sanitize_data -- --input {{INPUT}} --output {{OUTPUT}}

# Prepare test data dump (export + sanitize)
prepare-test-data:
    just export-prod-data
    just sanitize-data _build/dumps/dump.json _build/dumps/dump.sanitized.json.gz
    @echo "✅ Test data prepared at _build/dumps/dump.sanitized.json.gz"

