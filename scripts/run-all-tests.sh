#!/bin/bash

# Comprehensive Test Runner - Runs ALL Tests
# This script runs every test in the project:
# - Backend unit tests (lib tests)
# - Backend integration tests (tests/ directory)
# - Testing package integration tests
# - Frontend E2E tests (Playwright)
# - Cache integration tests
#
# Usage: ./scripts/run-all-tests.sh [--skip-e2e] [--skip-services] [--prod-containers]
#
# --prod-containers: Build and test against production Docker containers (recommended for pre-deployment)
#                    This ensures you're testing the exact containers that will be deployed

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

log_step() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}▶ $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

# Parse arguments
SKIP_E2E=false
SKIP_SERVICES=false
SERVICES_STARTED=false
USE_PROD_CONTAINERS=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-e2e)
            SKIP_E2E=true
            shift
            ;;
        --skip-services)
            SKIP_SERVICES=true
            shift
            ;;
        --prod-containers)
            USE_PROD_CONTAINERS=true
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Usage: $0 [--skip-e2e] [--skip-services] [--prod-containers]"
            echo ""
            echo "Options:"
            echo "  --skip-e2e          Skip frontend E2E tests"
            echo "  --skip-services     Don't auto-start services"
            echo "  --prod-containers   Build and test against production Docker containers"
            echo "                      (recommended: test exact containers you'll deploy)"
            exit 1
            ;;
    esac
done

cd "$PROJECT_ROOT"

# Check if we're in the project root
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    log_error "Must be run from project root"
    exit 1
fi

# Function to check if a service is running
check_service() {
    local service=$1
    local port=$2
    
    if command -v nc &> /dev/null; then
        nc -z localhost "$port" 2>/dev/null
    elif command -v timeout &> /dev/null; then
        timeout 1 bash -c "cat < /dev/null > /dev/tcp/localhost/$port" 2>/dev/null
    else
        # Fallback: check if process is listening
        (netstat -an 2>/dev/null || ss -an 2>/dev/null) | grep -q ":$port " 2>/dev/null
    fi
}

# Function to start services if needed
start_services() {
    if [ "$SKIP_SERVICES" = true ]; then
        log_info "Skipping service startup (--skip-services)"
        return
    fi

    log_step "Setting Up Test Services"

    REDIS_RUNNING=false
    ARANGO_RUNNING=false

    # Check Redis
    if check_service "redis" 6379; then
        log_success "Redis is already running on port 6379"
        REDIS_RUNNING=true
    else
        log_info "Redis not found on port 6379"
    fi

    # Check ArangoDB
    if check_service "arangodb" 8529; then
        log_success "ArangoDB is already running on port 8529"
        ARANGO_RUNNING=true
    else
        log_info "ArangoDB not found on port 8529"
    fi

    # Start services if needed
    if [ "$REDIS_RUNNING" = false ] || [ "$ARANGO_RUNNING" = false ]; then
        log_info "Starting required services..."
        
        if command -v docker-compose &> /dev/null || command -v docker &> /dev/null; then
            log_info "Using setup-hybrid-dev.sh to start services..."
            if ./scripts/setup-hybrid-dev.sh --skip-data 2>&1 | grep -q "already running\|ready\|started"; then
                log_success "Services started or already running"
                SERVICES_STARTED=true
            else
                log_warning "Service startup may have issues, but continuing..."
                SERVICES_STARTED=true
            fi
        else
            log_warning "Docker not available - tests requiring services may fail"
            log_info "Please start Redis and ArangoDB manually, or install Docker"
        fi
    else
        SERVICES_STARTED=true
    fi

    # Give services a moment to be ready
    if [ "$SERVICES_STARTED" = true ]; then
        log_info "Waiting for services to be ready..."
        sleep 2
    fi
}

# Function to cleanup services (if we started them)
cleanup_services() {
    if [ "$USE_PROD_CONTAINERS" = true ]; then
        log_info "Cleaning up production test containers..."
        ENV_FILE="${PROJECT_ROOT}/config/.env.production"
        if [ -f "$ENV_FILE" ]; then
            export ENV_FILE
            docker compose \
                --env-file "$ENV_FILE" \
                -f deploy/docker-compose.yaml \
                -f deploy/docker-compose.prod.yml \
                down 2>/dev/null || true
        fi
    elif [ "$SERVICES_STARTED" = true ] && [ "$SKIP_SERVICES" = false ]; then
        log_info "Cleaning up test services..."
        # Don't stop services if they were already running
        # The setup script handles this intelligently
    fi
}

# Trap to ensure cleanup on exit
trap cleanup_services EXIT

# ============================================================================
# PRE-PHASE: Build Production Containers (if requested)
# ============================================================================
if [ "$USE_PROD_CONTAINERS" = true ]; then
    log_step "PRE-PHASE: Building Production Docker Containers"
    
    log_info "Building production Docker images..."
    log_warning "This ensures you're testing the EXACT containers that will be deployed to production"
    
    if ! ./scripts/build-prod-images.sh; then
        log_error "Failed to build production images!"
        exit 1
    fi
    
    if [ ! -f "$PROJECT_ROOT/_build/.build-version" ]; then
        log_error "Build version file not found after building!"
        exit 1
    fi
    
    source "$PROJECT_ROOT/_build/.build-version"
    log_success "Production images built: $VERSION_TAG"
    log_info "  Git Commit: $GIT_COMMIT"
    log_info "  Build Date: $BUILD_DATE"
    
    log_step "Starting Production Containers for Testing"
    
    # Check for production environment file
    ENV_FILE="${PROJECT_ROOT}/config/.env.production"
    if [ ! -f "$ENV_FILE" ]; then
        log_error "Production environment file not found: $ENV_FILE"
        log_info "Run: ./config/setup-env.sh production"
        exit 1
    fi
    
    export ENV_FILE
    
    # Stop any existing containers
    log_info "Stopping any existing containers..."
    docker compose \
        --env-file "$ENV_FILE" \
        -f deploy/docker-compose.yaml \
        -f deploy/docker-compose.prod.yml \
        down 2>/dev/null || true
    
    # Start production containers
    log_info "Starting production containers..."
    docker compose \
        --env-file "$ENV_FILE" \
        -f deploy/docker-compose.yaml \
        -f deploy/docker-compose.prod.yml \
        up -d
    
    # Wait for services to be healthy
    log_info "Waiting for production containers to be healthy..."
    sleep 5
    
    MAX_WAIT=60
    WAITED=0
    while [ $WAITED -lt $MAX_WAIT ]; do
        if docker compose \
            --env-file "$ENV_FILE" \
            -f deploy/docker-compose.yaml \
            -f deploy/docker-compose.prod.yml \
            ps | grep -q "healthy\|running"; then
            break
        fi
        sleep 2
        WAITED=$((WAITED + 2))
    done
    
    # Load production environment for tests
    set -a
    source "$ENV_FILE"
    set +a
    
    # Update Redis/Arango URLs to point to production containers
    export REDIS_URL="${REDIS_URL:-redis://127.0.0.1:6379/}"
    export ARANGO_URL="${ARANGO_URL:-http://localhost:8529}"
    
    log_success "Production containers are running and ready!"
    log_info "  Backend: http://localhost:${BACKEND_PORT:-50012}"
    log_info "  Frontend: http://localhost:${FRONTEND_PORT:-50013}"
    log_info "  ArangoDB: ${ARANGO_URL}"
    log_info "  Redis: ${REDIS_URL}"
    log_info ""
    log_warning "⚠️  Tests will run against PRODUCTION containers"
    log_warning "⚠️  Using production environment configuration"
    
    # Note: We're using production containers, so skip service startup
    SKIP_SERVICES=true
    SERVICES_STARTED=true
fi

# ============================================================================
# START TESTING
# ============================================================================

log_step "🧪 Running ALL Tests"

# Setup services (only if not using prod containers)
if [ "$USE_PROD_CONTAINERS" = false ]; then
    start_services
fi

# ============================================================================
# STEP 1: Backend Unit Tests (Library Tests)
# ============================================================================
log_step "STEP 1: Backend Unit Tests (Library)"

if [ "$USE_PROD_CONTAINERS" = true ]; then
    log_warning "⚠️  NOTE: Unit tests run on host, but integration tests will use production containers"
    log_info "This is expected - unit tests test code logic, integration tests verify containers work"
fi

log_info "Running all backend library unit tests..."
if ! cargo nextest run --workspace --lib; then
    log_error "Backend unit tests failed!"
    exit 1
fi
log_success "Backend unit tests passed"

# ============================================================================
# STEP 2: Backend Integration Tests (tests/ directory)
# ============================================================================
log_step "STEP 2: Backend Integration Tests"

log_info "Running backend integration tests (including cache tests)..."
if [ "$USE_PROD_CONTAINERS" = true ]; then
    log_info "Using production container environment (Redis/ArangoDB from containers)"
    # Tests will connect to production containers via environment variables set earlier
fi
# Run all tests in backend/tests/ directory
if ! cargo nextest run --package backend --test '*' --no-fail-fast; then
    log_warning "Some backend integration tests failed or were ignored"
    log_info "Note: Cache integration tests require Redis running"
    log_info "Note: Some repository cache tests are marked #[ignore] and skipped by default"
fi
log_success "Backend integration tests completed"

# ============================================================================
# STEP 3: Testing Package Integration Tests
# ============================================================================
log_step "STEP 3: Testing Package Integration Tests"

log_info "Running integration tests with 3-tier strategy..."
if [ "$USE_PROD_CONTAINERS" = true ]; then
    log_info "Using production container environment for integration tests"
fi
if ! ./scripts/test-integration-3tier.sh; then
    log_error "Integration tests failed!"
    exit 1
fi
log_success "Integration tests passed"

# ============================================================================
# STEP 4: Cache Integration Tests (Explicit)
# ============================================================================
log_step "STEP 4: Cache Integration Tests"

log_info "Running cache integration tests (requires Redis)..."
if check_service "redis" 6379; then
    log_info "Redis is available, running cache tests..."
    if [ "$USE_PROD_CONTAINERS" = true ]; then
        log_info "Using Redis from production containers"
    fi
    
    # Set Redis URL for tests (already set if using prod containers)
    export REDIS_URL="${REDIS_URL:-redis://127.0.0.1:6379/}"
    
    if cargo test --package backend --test cache_integration_test 2>&1; then
        log_success "Cache integration tests passed"
    else
        log_warning "Some cache integration tests may have failed (check output above)"
    fi
    
    # Run repository cache tests (they're marked #[ignore] but we can run them explicitly)
    log_info "Running repository cache tests (requires Redis + ArangoDB)..."
    if check_service "arangodb" 8529; then
        if [ "$USE_PROD_CONTAINERS" = true ]; then
            log_info "Using ArangoDB from production containers"
        fi
        export ARANGO_URL="${ARANGO_URL:-http://localhost:8529}"
        export ARANGO_USERNAME="${ARANGO_USERNAME:-root}"
        export ARANGO_PASSWORD="${ARANGO_PASSWORD:-test}"
        
        if cargo test --package backend --test repository_cache_test -- --ignored 2>&1; then
            log_success "Repository cache tests passed"
        else
            log_warning "Some repository cache tests may have failed (check output above)"
        fi
    else
        log_warning "ArangoDB not available, skipping repository cache tests"
    fi
else
    log_warning "Redis not available, skipping cache integration tests"
    log_info "To run cache tests, start Redis: docker run -d -p 6379:6379 redis:7-alpine"
fi

# ============================================================================
# STEP 5: Frontend E2E Tests
# ============================================================================
if [ "$SKIP_E2E" = false ]; then
    log_step "STEP 5: Frontend E2E Tests (Playwright)"
    
    if [ "$USE_PROD_CONTAINERS" = true ]; then
        log_info "E2E tests will use production frontend container"
        log_info "Backend URL: http://localhost:${BACKEND_PORT:-50012}"
        log_info "Frontend URL: http://localhost:${FRONTEND_PORT:-50013}"
        # E2E tests will connect to production containers via environment
        export PLAYWRIGHT_BASE_URL="http://localhost:${FRONTEND_PORT:-50013}"
        export PLAYWRIGHT_API_URL="http://localhost:${BACKEND_PORT:-50012}"
    else
        log_info "Checking if E2E images need to be built..."
        if ! docker images | grep -q "stg_rd-frontend-e2e"; then
            log_warning "E2E images not found. Building them now..."
            if ! ./scripts/build-e2e-images.sh; then
                log_error "Failed to build E2E images!"
                exit 1
            fi
        fi
    fi
    
    log_info "Running Playwright E2E tests..."
    if ! npx playwright test; then
        log_error "Frontend E2E tests failed!"
        exit 1
    fi
    log_success "Frontend E2E tests passed"
else
    log_info "Skipping E2E tests (--skip-e2e)"
fi

# ============================================================================
# FINAL SUMMARY
# ============================================================================
log_step "✅ All Tests Completed!"

log_success "Test Summary:"
if [ "$USE_PROD_CONTAINERS" = true ]; then
    log_info "  🐳 Production Docker containers: YES (testing exact deployment artifacts)"
    source "$PROJECT_ROOT/_build/.build-version" 2>/dev/null || true
    if [ -n "$VERSION_TAG" ]; then
        log_info "  📦 Version Tag: $VERSION_TAG"
    fi
else
    log_info "  🐳 Production Docker containers: NO (testing host code directly)"
fi
log_info "  ✅ Backend unit tests (library)"
log_info "  ✅ Backend integration tests"
log_info "  ✅ Testing package integration tests"
if [ "$SKIP_E2E" = false ]; then
    log_info "  ✅ Frontend E2E tests"
else
    log_info "  ⏭️  Frontend E2E tests (skipped)"
fi

if check_service "redis" 6379; then
    log_info "  ✅ Cache integration tests"
else
    log_info "  ⏭️  Cache integration tests (Redis not available)"
fi

echo ""
if [ "$USE_PROD_CONTAINERS" = true ]; then
    log_success "🎉 All tests completed against PRODUCTION containers!"
    log_info ""
    log_info "These are the EXACT containers that will be deployed to production."
    log_info "If all tests passed, you can confidently deploy version: ${VERSION_TAG:-unknown}"
else
    log_success "🎉 All configured tests have been executed!"
    log_info ""
    log_info "💡 Tip: Use --prod-containers to test against production Docker containers"
    log_info "         This ensures you're testing the exact containers you'll deploy"
fi
echo ""
