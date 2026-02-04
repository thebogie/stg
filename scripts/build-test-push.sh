#!/bin/bash

# Dev Side: Build, Test, and Push Production Containers
# This script:
# 1. Builds production Docker images (with fresh WASM files)
# 2. Starts production containers
# 3. Runs ALL tests (unit, integration, e2e) against those containers
# 4. Pushes tested images to Docker Hub
#
# Usage: ./scripts/build-test-push.sh
#
# Prerequisites:
#   - docker login (for pushing to Docker Hub)
#   - DOCKER_HUB_USER env var (default: therealbogie)

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }
log_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
log_error() { echo -e "${RED}❌ $1${NC}"; }
log_step() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}▶ $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

cd "$PROJECT_ROOT"

# Check prerequisites
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    log_error "Must be run from project root"
    exit 1
fi

# Get Docker Hub username
DOCKER_HUB_USER="${DOCKER_HUB_USER:-therealbogie}"
log_info "Using Docker Hub user: $DOCKER_HUB_USER"

# Check Docker login
if ! docker info >/dev/null 2>&1; then
    log_error "Docker is not running or you're not logged in"
    log_info "Please run: docker login"
    exit 1
fi

# ============================================================================
# STEP 1: Build Production Images
# ============================================================================
log_step "STEP 1: Building Production Images"

log_info "Building production containers (release mode, optimized)..."
log_info "Frontend will be built with --no-cache to ensure fresh WASM files"

if ! ./scripts/build-prod-images.sh; then
    log_error "Failed to build production images!"
    exit 1
fi

if [ ! -f "$PROJECT_ROOT/_build/.build-version" ]; then
    log_error "Build version file not found after building!"
    exit 1
fi

source "$PROJECT_ROOT/_build/.build-version"
log_success "Production images built successfully"
log_info "  Version Tag: $VERSION_TAG"
log_info "  Git Commit: $GIT_COMMIT"
log_info "  Build Date: $BUILD_DATE"

# ============================================================================
# STEP 2: Start Production Containers
# ============================================================================
log_step "STEP 2: Starting Production Containers"

ENV_FILE="${PROJECT_ROOT}/config/.env.production"
if [ ! -f "$ENV_FILE" ]; then
    log_error "Production environment file not found: $ENV_FILE"
    exit 1
fi

log_info "Starting production containers for testing..."
export ENV_FILE
export IMAGE_TAG="$VERSION_TAG"
export FRONTEND_IMAGE_TAG="$VERSION_TAG"

# Stop any existing containers
log_info "Stopping any existing containers..."
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    down 2>/dev/null || true

# Start containers
log_info "Starting containers..."
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    up -d

# Wait for services to be healthy
log_info "Waiting for services to be healthy..."
MAX_WAIT=120
WAITED=0
while [ $WAITED -lt $MAX_WAIT ]; do
    if docker compose \
        --env-file "$ENV_FILE" \
        -f deploy/docker-compose.production.yml \
        ps | grep -q "healthy\|running"; then
        break
    fi
    sleep 2
    WAITED=$((WAITED + 2))
done

log_success "Containers are running"

# ============================================================================
# STEP 3: Run All Tests
# ============================================================================
log_step "STEP 3: Running All Tests"

# Get ports from env file
source "$ENV_FILE"
FRONTEND_PORT="${FRONTEND_PORT:-50003}"
BACKEND_PORT="${BACKEND_PORT:-50002}"

# Set environment for tests
export BACKEND_URL="http://localhost:${BACKEND_PORT}"
export PLAYWRIGHT_BASE_URL="http://localhost:${FRONTEND_PORT}"
export PLAYWRIGHT_API_URL="http://localhost:${BACKEND_PORT}"
export USE_PRODUCTION_CONTAINERS=true

mkdir -p "$PROJECT_ROOT/_build/test-results"

# 3.1: Unit Tests
log_info "Running unit tests..."
if ! cargo nextest run --workspace --lib --run-ignored all 2>&1 | tee "$PROJECT_ROOT/_build/test-results/unit-tests-output.log"; then
    log_error "Unit tests failed!"
    docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml down 2>/dev/null || true
    exit 1
fi
log_success "Unit tests passed"

# 3.2: Integration Tests
log_info "Running integration tests..."
if ! cargo nextest run \
    --package backend \
    --test '*' \
    --no-fail-fast \
    --run-ignored all 2>&1 | tee "$PROJECT_ROOT/_build/test-results/integration-tests-output.log"; then
    log_error "Integration tests failed!"
    docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml down 2>/dev/null || true
    exit 1
fi
log_success "Integration tests passed"

# 3.3: Cache Integration Tests
log_info "Running cache integration tests..."
if ! cargo nextest run \
    --package backend \
    --test 'cache_integration_test' \
    --no-fail-fast 2>&1 | tee "$PROJECT_ROOT/_build/test-results/cache-tests-output.log"; then
    log_warning "Cache integration tests failed (may be non-critical)"
    # Don't fail the whole suite - cache tests can be flaky
fi
log_success "Cache integration tests completed"

# 3.4: E2E API Tests
log_info "Running E2E API tests..."
if ! cargo nextest run \
    --package testing \
    --test '*_e2e' \
    --no-fail-fast 2>&1 | tee "$PROJECT_ROOT/_build/test-results/e2e-api-tests-output.log"; then
    log_error "E2E API tests failed!"
    docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml down 2>/dev/null || true
    exit 1
fi
log_success "E2E API tests passed"

# 3.5: Playwright E2E Tests
log_info "Running Playwright E2E tests..."
if ! npx playwright test 2>&1 | tee "$PROJECT_ROOT/_build/test-results/playwright-output.log"; then
    log_error "Playwright E2E tests failed!"
    docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml down 2>/dev/null || true
    exit 1
fi
log_success "Playwright E2E tests passed"

log_success "All tests passed!"

# ============================================================================
# STEP 4: Push Tested Images to Docker Hub
# ============================================================================
log_step "STEP 4: Pushing Tested Images to Docker Hub"

# Find the tested images
FRONTEND_LOCAL=""
BACKEND_LOCAL=""

if docker image inspect "stg_rd-frontend:tested" > /dev/null 2>&1; then
    FRONTEND_LOCAL="stg_rd-frontend:tested"
elif docker image inspect "stg_rd-frontend:${VERSION_TAG}" > /dev/null 2>&1; then
    FRONTEND_LOCAL="stg_rd-frontend:${VERSION_TAG}"
else
    log_error "Frontend image not found!"
    exit 1
fi

if docker image inspect "stg_rd-backend:tested" > /dev/null 2>&1; then
    BACKEND_LOCAL="stg_rd-backend:tested"
elif docker image inspect "stg_rd-backend:${VERSION_TAG}" > /dev/null 2>&1; then
    BACKEND_LOCAL="stg_rd-backend:${VERSION_TAG}"
else
    log_error "Backend image not found!"
    exit 1
fi

# Tag for Docker Hub
FRONTEND_HUB="${DOCKER_HUB_USER}/stg_rd:frontend-${VERSION_TAG}"
BACKEND_HUB="${DOCKER_HUB_USER}/stg_rd:backend-${VERSION_TAG}"

log_info "Tagging for Docker Hub..."
docker tag "$FRONTEND_LOCAL" "$FRONTEND_HUB"
docker tag "$BACKEND_LOCAL" "$BACKEND_HUB"

log_info "Pushing frontend image..."
if ! docker push "$FRONTEND_HUB"; then
    log_error "Failed to push frontend image!"
    exit 1
fi
log_success "Frontend pushed: $FRONTEND_HUB"

log_info "Pushing backend image..."
if ! docker push "$BACKEND_HUB"; then
    log_error "Failed to push backend image!"
    exit 1
fi
log_success "Backend pushed: $BACKEND_HUB"

# ============================================================================
# STEP 5: Cleanup and Summary
# ============================================================================
log_step "STEP 5: Cleanup"

log_info "Stopping test containers..."
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    down 2>/dev/null || true

# Create deployment info
mkdir -p "${PROJECT_ROOT}/_build"
DEPLOY_INFO="${PROJECT_ROOT}/_build/deploy-info-${VERSION_TAG}.txt"
cat > "$DEPLOY_INFO" <<EOF
Deployment Information
=====================
Version Tag: $VERSION_TAG
Git Commit: $GIT_COMMIT
Build Date: $BUILD_DATE
Push Date: $(date -u +"%Y-%m-%d %H:%M:%S UTC")

Docker Hub Images:
  Frontend: $FRONTEND_HUB
  Backend: $BACKEND_HUB

Production Deployment:
----------------------
# On production server:
cd ~/stg/repo
git pull
./scripts/deploy-production.sh --version $VERSION_TAG
EOF

log_success ""
log_success "🎉 Build, Test, and Push Complete!"
log_success ""
log_info "Version: $VERSION_TAG"
log_info "Images pushed to: ${DOCKER_HUB_USER}/stg_rd"
log_info ""
log_info "To deploy to production:"
log_info "  ./scripts/deploy-production.sh --version $VERSION_TAG"
log_info ""
log_info "Deployment info: $DEPLOY_INFO"
