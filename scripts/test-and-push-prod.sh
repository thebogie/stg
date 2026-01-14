#!/bin/bash

# Test, Build, and Push Production Docker Images
# This script runs all tests, and if they pass, builds and pushes production images to Docker Hub
# Usage: ./scripts/test-and-push-prod.sh [DOCKER_HUB_USERNAME]
#
# Example:
#   ./scripts/test-and-push-prod.sh therealbogie
#
# If DOCKER_HUB_USERNAME is not provided, it will try to detect from existing images
# or prompt you to set it via DOCKER_HUB_USER environment variable

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
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}▶ $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

# Check if we're in the project root
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    log_error "Must be run from project root"
    exit 1
fi

cd "$PROJECT_ROOT"

# Get Docker Hub username
DOCKER_HUB_USER="${1:-${DOCKER_HUB_USER}}"

# Try to detect from existing images if not provided
if [ -z "$DOCKER_HUB_USER" ]; then
    EXISTING_IMAGE=$(docker images --format "{{.Repository}}" | grep -E "^[^/]+/stg_rd" | head -1 | cut -d'/' -f1 || true)
    if [ -n "$EXISTING_IMAGE" ]; then
        DOCKER_HUB_USER="$EXISTING_IMAGE"
        log_info "Detected Docker Hub username from existing images: $DOCKER_HUB_USER"
    else
        log_error "Docker Hub username not provided and could not be detected"
        log_info "Usage: $0 [DOCKER_HUB_USERNAME]"
        log_info "Or set: export DOCKER_HUB_USER=your-username"
        exit 1
    fi
fi

log_info "Using Docker Hub username: $DOCKER_HUB_USER"

# Check if docker is logged in
if ! docker info >/dev/null 2>&1; then
    log_error "Docker is not running or you're not logged in"
    log_info "Please run: docker login"
    exit 1
fi

# Check Docker Hub login
if ! docker pull "$DOCKER_HUB_USER/stg_rd:backend-latest" >/dev/null 2>&1 && \
   ! docker images "$DOCKER_HUB_USER/stg_rd" | grep -q .; then
    log_warning "Could not verify Docker Hub access. Make sure you're logged in:"
    log_info "  docker login"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# ============================================================================
# STEP 1: Run All Tests
# ============================================================================
log_step "STEP 1: Running All Tests"

log_info "Running backend unit tests..."
if ! cargo nextest run --workspace --lib --tests; then
    log_error "Backend unit tests failed!"
    exit 1
fi
log_success "Backend unit tests passed"

log_info "Running integration tests with 3-tier strategy..."
# 3-Tier approach: Fast tests (4 threads) -> Retry failures (2 threads) -> Slow tests (1 thread)
if ! ./scripts/test-integration-3tier.sh; then
    log_error "Integration tests failed!"
    exit 1
fi
log_success "Integration tests passed"

log_info "Running frontend E2E tests..."
# Check if E2E images need to be built
if ! docker images | grep -q "stg_rd-frontend-e2e"; then
    log_warning "E2E images not found. Building them now..."
    ./scripts/build-e2e-images.sh
fi

if ! npx playwright test; then
    log_error "Frontend E2E tests failed!"
    exit 1
fi
log_success "Frontend E2E tests passed"

log_success "All tests passed! ✅"

# ============================================================================
# STEP 2: Build Production Images
# ============================================================================
log_step "STEP 2: Building Production Docker Images"

if ! ./scripts/build-prod-images.sh; then
    log_error "Failed to build production images!"
    exit 1
fi

# Load version info from build
if [ ! -f "_build/.build-version" ]; then
    log_error "Version file not found after build!"
    exit 1
fi

source "_build/.build-version"
log_success "Production images built successfully"
log_info "Version Tag: $VERSION_TAG"

# ============================================================================
# STEP 3: Tag Images for Docker Hub
# ============================================================================
log_step "STEP 3: Tagging Images for Docker Hub"

# Get actual image names - try multiple possibilities
# The build script uses compose project name, but images might be tagged differently
COMPOSE_PROJECT_NAME=$(basename "$PROJECT_ROOT" | tr '[:upper:]' '[:lower:]' | tr -cd '[:alnum:]-')
FRONTEND_FULL="${COMPOSE_PROJECT_NAME}-frontend"
BACKEND_FULL="${COMPOSE_PROJECT_NAME}-backend"

# Function to check if an image exists
image_exists() {
    local img_name="$1"
    docker image inspect "$img_name" >/dev/null 2>&1
}

# Try to find the actual image names by checking what exists
FRONTEND_LOCAL=""
BACKEND_LOCAL=""

# Check for frontend image in order of likelihood
for img in "stg_rd-frontend:latest" "${FRONTEND_FULL}:latest" "stg-frontend:latest"; do
    if image_exists "$img"; then
        FRONTEND_LOCAL="$img"
        log_info "Found frontend image: $img"
        break
    fi
done

# Check for backend image in order of likelihood
for img in "stg_rd-backend:latest" "${BACKEND_FULL}:latest" "stg-backend:latest"; do
    if image_exists "$img"; then
        BACKEND_LOCAL="$img"
        log_info "Found backend image: $img"
        break
    fi
done

# Verify we found the images
if [ -z "$FRONTEND_LOCAL" ]; then
    log_error "Could not find frontend image!"
    log_info "Searched for: stg_rd-frontend:latest, ${FRONTEND_FULL}:latest, stg-frontend:latest"
    log_info "Available images with 'frontend' or 'backend' in name:"
    docker images | grep -iE "frontend|backend" | head -10 || true
    exit 1
fi

if [ -z "$BACKEND_LOCAL" ]; then
    log_error "Could not find backend image!"
    log_info "Searched for: stg_rd-backend:latest, ${BACKEND_FULL}:latest, stg-backend:latest"
    log_info "Available images with 'frontend' or 'backend' in name:"
    docker images | grep -iE "frontend|backend" | head -10 || true
    exit 1
fi

log_info "Using images:"
log_info "  Frontend: $FRONTEND_LOCAL"
log_info "  Backend: $BACKEND_LOCAL"

# Tag for Docker Hub
FRONTEND_HUB="${DOCKER_HUB_USER}/stg_rd:frontend-${VERSION_TAG}"
BACKEND_HUB="${DOCKER_HUB_USER}/stg_rd:backend-${VERSION_TAG}"

log_info "Tagging frontend: $FRONTEND_LOCAL -> $FRONTEND_HUB"
docker tag "$FRONTEND_LOCAL" "$FRONTEND_HUB"

log_info "Tagging backend: $BACKEND_LOCAL -> $BACKEND_HUB"
docker tag "$BACKEND_LOCAL" "$BACKEND_HUB"

log_success "Images tagged for Docker Hub"

# ============================================================================
# STEP 4: Push to Docker Hub
# ============================================================================
log_step "STEP 4: Pushing Images to Docker Hub"

log_info "Pushing frontend image..."
if ! docker push "$FRONTEND_HUB"; then
    log_error "Failed to push frontend image!"
    exit 1
fi
log_success "Frontend image pushed: $FRONTEND_HUB"

log_info "Pushing backend image..."
if ! docker push "$BACKEND_HUB"; then
    log_error "Failed to push backend image!"
    exit 1
fi
log_success "Backend image pushed: $BACKEND_HUB"

# ============================================================================
# STEP 5: Summary and Next Steps
# ============================================================================
log_step "STEP 5: Deployment Ready! 🚀"

log_success "All tests passed and images pushed to Docker Hub!"
echo ""
log_info "Version Information:"
log_info "  Version Tag: $VERSION_TAG"
log_info "  Git Commit: $GIT_COMMIT"
log_info "  Build Date: $BUILD_DATE"
echo ""
log_info "Docker Hub Images:"
log_info "  Frontend: $FRONTEND_HUB"
log_info "  Backend: $BACKEND_HUB"
echo ""
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "📋 NEXT STEPS - On Your Production Server:"
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
log_info "1. Login to Docker Hub:"
log_info "   docker login"
echo ""
log_info "2. Pull the tested images:"
log_info "   docker pull $FRONTEND_HUB"
log_info "   docker pull $BACKEND_HUB"
echo ""
log_info "3. Tag them for deployment:"
log_info "   docker tag $FRONTEND_HUB stg_rd-frontend:latest"
log_info "   docker tag $BACKEND_HUB stg_rd-backend:latest"
echo ""
log_info "4. Deploy using the deployment script:"
log_info "   ./scripts/deploy-tested-images.sh --version $VERSION_TAG --skip-load"
echo ""
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Save deployment info
mkdir -p "_build"
DEPLOY_INFO="_build/deploy-info-${VERSION_TAG}.txt"
cat > "$DEPLOY_INFO" <<EOF
Deployment Information
======================
Version Tag: $VERSION_TAG
Git Commit: $GIT_COMMIT
Build Date: $BUILD_DATE

Docker Hub Images:
  Frontend: $FRONTEND_HUB
  Backend: $BACKEND_HUB

Production Server Commands:
---------------------------
# Pull images
docker pull $FRONTEND_HUB
docker pull $BACKEND_HUB

# Tag for deployment
docker tag $FRONTEND_HUB stg_rd-frontend:latest
docker tag $BACKEND_HUB stg_rd-backend:latest

# Deploy
./scripts/deploy-tested-images.sh --version $VERSION_TAG --skip-load
EOF

log_success "Deployment info saved to: $DEPLOY_INFO"

