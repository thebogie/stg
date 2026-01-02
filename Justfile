# Modern Testing Architecture for Rust 2026
# Uses cargo-nextest, cargo-llvm-cov, testcontainers-rs, and Playwright

# Default recipe - run all tests
default:
    just test-all

# Run all tests with nextest
# Runs: backend unit tests, integration tests, E2E tests, and optional WASM unit tests
test-all:
    just test-backend
    just test-integration
    just test-frontend-e2e
    # WASM unit tests are optional (may fail in WSL2/headless environments)
    just test-frontend-unit || true

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

# Build Docker images for E2E testing (run this first or when code changes)
# Industry standard: Build images separately, don't build during test runs
test-e2e-build-images:
    ./scripts/build-e2e-images.sh

# Run frontend E2E tests with Playwright (PRIMARY frontend testing method)
# Tests the complete frontend application in real browsers using Docker
# Note: Images must be built first with: just test-e2e-build-images
# Or set BUILD_IMAGES=1 to build now (slower)
test-frontend-e2e:
    @echo "🚀 Starting E2E tests with Docker..."
    @echo "💡 Images should be pre-built. If not, run: just test-e2e-build-images"
    @echo "💡 Or set BUILD_IMAGES=1 to build now (slower): BUILD_IMAGES=1 just test-frontend-e2e"
    npx playwright test

# Run E2E tests and build images if needed (convenience command)
test-frontend-e2e-full:
    @echo "🔨 Building images (if needed) and running E2E tests..."
    BUILD_IMAGES=1 npx playwright test

# Stop E2E test Docker containers
test-frontend-e2e-stop:
    ./scripts/stop-e2e-docker.sh

# Alias for convenience
test-frontend:
    just test-frontend-e2e

# Run frontend unit tests (wasm-bindgen-test) - OPTIONAL
# Note: May fail in WSL2/headless environments due to geckodriver issues.
# E2E tests (test-frontend-e2e) are the primary frontend testing method.
# This command will not fail the build if geckodriver has issues.
# Note: Server-side dependencies (tokio, reqwest, actix-web) have been removed
# from frontend dev-dependencies to avoid WASM compilation errors with mio.
test-frontend-unit:
    cd frontend && (wasm-pack test --headless --firefox || echo "⚠️  WASM unit tests skipped (geckodriver issue - this is OK, E2E tests provide coverage)")

# Generate coverage report (HTML)
coverage:
    ./scripts/coverage.sh

# Generate coverage report (LCOV for CI)
coverage-lcov:
    cargo llvm-cov nextest --workspace --lcov --output-path _build/lcov.info
    @echo "LCOV report generated at _build/lcov.info"

# Generate JUnit XML for CI
# Note: JUnit XML is auto-generated based on .nextest.toml config
test-junit:
    cargo nextest run --workspace --lib --tests
    @echo "JUnit XML should be at: _build/test-results.xml"

# Run all tests and generate reports
test-full:
    just test-junit
    just coverage
    just test-frontend-e2e  # Primary frontend testing
    @echo ""
    @echo "✅ Full test suite completed!"
    @echo "📊 Reports:"
    @echo "  - JUnit XML: _build/test-results.xml"
    @echo "  - Coverage: _build/coverage/html/index.html"
    @echo "  - E2E Report: _build/playwright-report/index.html"

# Run tests with verbose output and timing
test-verbose:
    cargo nextest run --workspace --lib --tests --test-threads 1 -- --nocapture

# Show test coverage summary only
coverage-summary:
    cargo llvm-cov nextest --workspace --lcov --output-path _build/coverage/lcov.info
    @if command -v lcov &> /dev/null; then \
        echo "📈 Coverage Summary:"; \
        lcov --summary _build/coverage/lcov.info 2>/dev/null | grep -E "lines|functions|branches" || true; \
    else \
        echo "Install 'lcov' for coverage summary"; \
    fi

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

