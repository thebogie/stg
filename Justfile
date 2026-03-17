# Modern Testing Architecture for Rust 2026
# Uses cargo-nextest, cargo-llvm-cov, testcontainers-rs, and Playwright

# Default recipe - run all tests
default:
    just test-all

# Run all tests with nextest
# Runs: backend unit tests, integration tests, and E2E tests
test-all:
    just test-backend
    just test-integration
    just test-frontend-e2e

# Run ALL tests comprehensively (including cache tests, auto-starts services)
# This is the most complete test runner - runs everything
test-everything:
    ./scripts/run-all-tests.sh

# Run ALL tests against PRODUCTION Docker containers
# Builds production images with build version, runs unit/integration/e2e, writes _build/<build_version>/
# Recommended before commit; then push to main triggers GHCR to build the same image
test-everything-prod:
    ./scripts/full-prod-test.sh

# Run backend tests with nextest
test-backend:
    cargo nextest run --workspace --lib --tests

# Run backend tests with coverage
test-backend-coverage:
    cargo llvm-cov nextest --workspace --lcov --output-path _build/lcov.info
    cargo llvm-cov nextest --workspace --html --output-dir _build/coverage/html

# Run integration tests (uses testcontainers)
# 3-Tier strategy: Fast (4 threads) -> Retry failures (2 threads) -> Slow tests (1 thread)
test-integration:
    ./scripts/test-integration-3tier.sh

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

# Start production stack locally for visual testing (run after workflow.sh or build.sh)
# Opens frontend at http://localhost:50003 (or FRONTEND_PORT from config)
start-stack-local:
    ./scripts/start-stack-local.sh

# Same as above but load production data into ArangoDB
start-stack-local-with-data:
    ./scripts/start-stack-local.sh --load-prod-data

# Stop the production stack started with start-stack-local.sh
stop-stack-local:
    ./scripts/stop-stack-local.sh

# Quick compile check for backend (no run, no tests). Use for instant feedback while editing.
backend-check:
    cargo check -p backend

# Run backend with auto-restart on file change. Loads config/.env.dev by default. Requires: cargo install cargo-watch
# Watches back/api and shared only so frontend edits don't restart the backend.
# For admin tab: set ADMIN_EMAILS=your@email.com in config/.env.dev
backend-watch:
    ./scripts/backend-watch.sh

# Same as backend-watch but loads config/.env.prod (e.g. for ADMIN_EMAILS from .env.prod)
backend-watch-prod:
    ./scripts/backend-watch.sh prod

# Start only SurrealDB + Redis (no backend container). Then run just backend-watch and ./scripts/start-tauri.sh in two other terminals.
# See docs/QUICK_ITERATION.md for full steps.
dev-deps:
    ./scripts/start-deps.sh

# Clean Docker build cache and optional project images so next build is fresh
# Use after Dockerfile changes or when you suspect stale layers
clean-docker:
    ./scripts/clean-docker-for-build.sh

# Same as clean-docker but also remove all local stg_rd-frontend/backend images
clean-docker-aggressive:
    ./scripts/clean-docker-for-build.sh --aggressive

# Alias for convenience
test-frontend:
    just test-frontend-e2e

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
    cargo install --locked cargo-nextest
    cargo install cargo-llvm-cov cargo-watch
    npx playwright install --with-deps
    @echo "✅ All testing tools installed!"

# Export/sanitize tasks removed (scripts crate removed). For Arango→Surreal use tools/arango-to-surreal.

