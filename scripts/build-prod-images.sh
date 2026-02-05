#!/bin/bash

# Build and tag production Docker images for testing
# This script builds production images locally and tags them with version info
# Usage: ./scripts/build-prod-images.sh

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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

# Check if we're in the project root
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    log_error "Must be run from project root"
    exit 1
fi

# Load build info
log_info "Getting build information..."
source "$PROJECT_ROOT/scripts/build-info.sh"
export GIT_COMMIT
export BUILD_DATE

# Create version tag
VERSION_TAG="v${GIT_COMMIT}-$(date +%Y%m%d-%H%M%S)"
export VERSION_TAG
# Also export as IMAGE_TAG for docker-compose
export IMAGE_TAG="$VERSION_TAG"

log_info "Build Information:"
log_info "  Git Commit: $GIT_COMMIT"
log_info "  Build Date: $BUILD_DATE"
log_info "  Version Tag: $VERSION_TAG"

# Check for production environment file
ENV_FILE="${PROJECT_ROOT}/config/.env.production"
if [ ! -f "$ENV_FILE" ]; then
    log_warning "Production environment file not found: $ENV_FILE"
    log_info "Creating from template..."
    if [ -f "${PROJECT_ROOT}/config/env.production.template" ]; then
        cp "${PROJECT_ROOT}/config/env.production.template" "$ENV_FILE"
        log_warning "Please edit $ENV_FILE with your production values"
        log_warning "Then run this script again"
        exit 1
    else
        log_error "Template file not found. Cannot proceed."
        exit 1
    fi
fi

# Export ENV_FILE for docker-compose
export ENV_FILE

log_info "Building production Docker images..."
cd "$PROJECT_ROOT"

# CRITICAL: Verify working directory matches committed code
# Docker COPY uses working directory, not git, so we must ensure they match
log_info "Verifying working directory matches committed code..."
if ! git diff --quiet HEAD -- frontend/src/pages/contests.rs; then
    log_error "Working directory has uncommitted changes to frontend/src/pages/contests.rs!"
    log_error "Docker build will use working directory files, not committed code."
    log_error "Please commit or stash your changes before building."
    git diff HEAD -- frontend/src/pages/contests.rs | head -20
    exit 1
fi

# Verify the committed code has "Players" (not "People")
if git show HEAD:frontend/src/pages/contests.rs | grep -q '"People"'; then
    log_error "Committed code still contains 'People' instead of 'Players'!"
    log_error "Please commit the fix before building."
    exit 1
fi

if ! git show HEAD:frontend/src/pages/contests.rs | grep -q '"Players"'; then
    log_error "Committed code doesn't contain 'Players'!"
    log_error "Please commit the fix before building."
    exit 1
fi

log_success "Working directory matches committed code - build will use correct source"

# Build images using docker compose
# IMPORTANT: Use --no-cache for frontend to ensure source code changes are always included
# Docker's layer caching can reuse old layers even when source files change if:
# - File timestamps haven't changed
# - File checksums are the same  
# - .dockerignore is excluding changed files
# Using --no-cache ensures a completely fresh build every time (required for CI/CD)
log_info "Building frontend (using --no-cache to ensure latest source code)..."
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    build --progress=plain --no-cache \
    --build-arg BUILD_DATE="$BUILD_DATE" \
    --build-arg GIT_COMMIT="$GIT_COMMIT" \
    frontend

log_info "Building backend..."
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    build --progress=plain \
    --build-arg BUILD_DATE="$BUILD_DATE" \
    --build-arg GIT_COMMIT="$GIT_COMMIT" \
    backend

log_success "Images built successfully"

# Tag images with version
log_info "Tagging images with version: $VERSION_TAG"

# Get image names from docker compose
FRONTEND_IMAGE=$(docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml config | grep -A 5 "frontend:" | grep "image:" | awk '{print $2}' | tr -d '"' || echo "stg_rd-frontend")
BACKEND_IMAGE=$(docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml config | grep -A 5 "backend:" | grep "image:" | awk '{print $2}' | tr -d '"' || echo "stg_rd-backend")

# Default to project name if image names not found
if [ -z "$FRONTEND_IMAGE" ] || [ "$FRONTEND_IMAGE" == "null" ]; then
    FRONTEND_IMAGE="stg_rd-frontend"
fi
if [ -z "$BACKEND_IMAGE" ] || [ "$BACKEND_IMAGE" == "null" ]; then
    BACKEND_IMAGE="stg_rd-backend"
fi

# Get actual image names (docker compose uses project name prefix)
COMPOSE_PROJECT_NAME=$(basename "$PROJECT_ROOT" | tr '[:upper:]' '[:lower:]' | tr -cd '[:alnum:]-')
FRONTEND_FULL="${COMPOSE_PROJECT_NAME}-frontend"
BACKEND_FULL="${COMPOSE_PROJECT_NAME}-backend"

# Tag with version (find which image name was actually used)
FRONTEND_SOURCE=""
if docker image inspect "${FRONTEND_FULL}:latest" > /dev/null 2>&1; then
    FRONTEND_SOURCE="${FRONTEND_FULL}:latest"
elif docker image inspect "${FRONTEND_IMAGE}:latest" > /dev/null 2>&1; then
    FRONTEND_SOURCE="${FRONTEND_IMAGE}:latest"
elif docker image inspect "stg_rd-frontend:latest" > /dev/null 2>&1; then
    FRONTEND_SOURCE="stg_rd-frontend:latest"
else
    log_error "Could not find frontend image to tag!"
    docker images | grep -i frontend | head -5
    exit 1
fi

BACKEND_SOURCE=""
if docker image inspect "${BACKEND_FULL}:latest" > /dev/null 2>&1; then
    BACKEND_SOURCE="${BACKEND_FULL}:latest"
elif docker image inspect "${BACKEND_IMAGE}:latest" > /dev/null 2>&1; then
    BACKEND_SOURCE="${BACKEND_IMAGE}:latest"
elif docker image inspect "stg_rd-backend:latest" > /dev/null 2>&1; then
    BACKEND_SOURCE="stg_rd-backend:latest"
else
    log_error "Could not find backend image to tag!"
    docker images | grep -i backend | head -5
    exit 1
fi

# Tag with version (standardize on stg_rd-* names for consistency)
docker tag "$FRONTEND_SOURCE" "stg_rd-frontend:${VERSION_TAG}"
docker tag "$BACKEND_SOURCE" "stg_rd-backend:${VERSION_TAG}"

# Also tag as 'tested' for easy reference
docker tag "stg_rd-frontend:${VERSION_TAG}" "stg_rd-frontend:tested"
docker tag "stg_rd-backend:${VERSION_TAG}" "stg_rd-backend:tested"

log_success "Images tagged:"
log_info "  Frontend: stg_rd-frontend:${VERSION_TAG} (also tagged as 'tested')"
log_info "  Backend: stg_rd-backend:${VERSION_TAG} (also tagged as 'tested')"

# Save version info to file for other scripts
mkdir -p "${PROJECT_ROOT}/_build"
VERSION_FILE="${PROJECT_ROOT}/_build/.build-version"
cat > "$VERSION_FILE" <<EOF
GIT_COMMIT="$GIT_COMMIT"
BUILD_DATE="$BUILD_DATE"
VERSION_TAG="$VERSION_TAG"
FRONTEND_IMAGE="stg_rd-frontend"
BACKEND_IMAGE="stg_rd-backend"
EOF

log_success "Version info saved to: $VERSION_FILE"
log_info ""
log_info "Next steps:"
log_info "  Run full CI/CD workflow: ./scripts/build-test-push.sh"
log_info "  (This will test the images and push to Docker Hub)"

