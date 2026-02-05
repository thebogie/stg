#!/bin/bash

# Build and Push Production Containers to Docker Hub (No Tests)
# This script builds production Docker images and pushes them to Docker Hub
# without running any tests. Use this when you need to push images quickly
# or when tests are already passing.
#
# Usage: ./scripts/build-push-only.sh
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

# Create log directory
mkdir -p "$PROJECT_ROOT/_build/logs"

# Set up logging - save all output to a log file with timestamp
LOG_FILE="$PROJECT_ROOT/_build/logs/build-push-only-$(date +%Y%m%d-%H%M%S).log"
exec > >(tee -a "$LOG_FILE") 2>&1

log_info "Logging all output to: $LOG_FILE"

# Check prerequisites
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    log_error "Must be run from project root"
    exit 1
fi

# Get Docker Hub username
DOCKER_HUB_USER="${DOCKER_HUB_USER:-therealbogie}"
if [ -z "$DOCKER_HUB_USER" ]; then
    log_error "DOCKER_HUB_USER not set. Please set it as an environment variable or update the script."
    exit 1
fi

log_info "Using Docker Hub username: $DOCKER_HUB_USER"

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
log_info "Build output will be saved to: $LOG_FILE"

# Build images and save output
BUILD_LOG="$PROJECT_ROOT/_build/logs/build-images-$(date +%Y%m%d-%H%M%S).log"
if ! ./scripts/build-prod-images.sh 2>&1 | tee "$BUILD_LOG"; then
    log_error "Failed to build production images!"
    log_error "Check build log: $BUILD_LOG"
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
# STEP 2: Push Images to Docker Hub
# ============================================================================
log_step "STEP 2: Pushing Images to Docker Hub"

# Find the built images
FRONTEND_LOCAL=""
BACKEND_LOCAL=""

if docker image inspect "stg_rd-frontend:${VERSION_TAG}" > /dev/null 2>&1; then
    FRONTEND_LOCAL="stg_rd-frontend:${VERSION_TAG}"
elif docker image inspect "stg_rd-frontend:tested" > /dev/null 2>&1; then
    FRONTEND_LOCAL="stg_rd-frontend:tested"
else
    log_error "Frontend image not found!"
    log_info "Available images:"
    docker images | grep -E "frontend|stg" | head -10
    exit 1
fi

if docker image inspect "stg_rd-backend:${VERSION_TAG}" > /dev/null 2>&1; then
    BACKEND_LOCAL="stg_rd-backend:${VERSION_TAG}"
elif docker image inspect "stg_rd-backend:tested" > /dev/null 2>&1; then
    BACKEND_LOCAL="stg_rd-backend:tested"
else
    log_error "Backend image not found!"
    log_info "Available images:"
    docker images | grep -E "backend|stg" | head -10
    exit 1
fi

# Tag for Docker Hub
FRONTEND_HUB="${DOCKER_HUB_USER}/stg_rd:frontend-${VERSION_TAG}"
BACKEND_HUB="${DOCKER_HUB_USER}/stg_rd:backend-${VERSION_TAG}"

log_info "Tagging images for Docker Hub..."
docker tag "$FRONTEND_LOCAL" "$FRONTEND_HUB"
docker tag "$BACKEND_LOCAL" "$BACKEND_HUB"
log_success "Images tagged"

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
# STEP 3: Summary
# ============================================================================
log_step "STEP 3: Summary"

log_success "Images pushed to Docker Hub successfully!"
log_info ""
log_info "Frontend: $FRONTEND_HUB"
log_info "Backend: $BACKEND_HUB"
log_info ""
log_info "Version Tag: $VERSION_TAG"
log_info "Git Commit: $GIT_COMMIT"
log_info "Build Date: $BUILD_DATE"
log_info ""
log_info "Log files saved:"
log_info "  Main log: $LOG_FILE"
if [ -n "${BUILD_LOG:-}" ]; then
    log_info "  Build log: $BUILD_LOG"
fi
log_info ""
log_info "To deploy to production:"
log_info "  ./scripts/deploy-production.sh --version $VERSION_TAG"
