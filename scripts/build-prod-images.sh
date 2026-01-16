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

# Build images using docker compose
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    build --progress=plain

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

# Tag with version
docker tag "${FRONTEND_FULL}:latest" "${FRONTEND_FULL}:${VERSION_TAG}" 2>/dev/null || \
docker tag "${FRONTEND_IMAGE}:latest" "${FRONTEND_IMAGE}:${VERSION_TAG}" 2>/dev/null || \
log_warning "Could not tag frontend image (may already be tagged)"

docker tag "${BACKEND_FULL}:latest" "${BACKEND_FULL}:${VERSION_TAG}" 2>/dev/null || \
docker tag "${BACKEND_IMAGE}:latest" "${BACKEND_IMAGE}:${VERSION_TAG}" 2>/dev/null || \
log_warning "Could not tag backend image (may already be tagged)"

# Also tag as 'tested' for easy reference
docker tag "${FRONTEND_FULL}:${VERSION_TAG}" "${FRONTEND_FULL}:tested" 2>/dev/null || \
docker tag "${FRONTEND_IMAGE}:${VERSION_TAG}" "${FRONTEND_IMAGE}:tested" 2>/dev/null || true

docker tag "${BACKEND_FULL}:${VERSION_TAG}" "${BACKEND_FULL}:tested" 2>/dev/null || \
docker tag "${BACKEND_IMAGE}:${VERSION_TAG}" "${BACKEND_IMAGE}:tested" 2>/dev/null || true

log_success "Images tagged:"
log_info "  Frontend: ${FRONTEND_FULL}:${VERSION_TAG} (also tagged as 'tested')"
log_info "  Backend: ${BACKEND_FULL}:${VERSION_TAG} (also tagged as 'tested')"

# Save version info to file for other scripts
mkdir -p "${PROJECT_ROOT}/_build"
VERSION_FILE="${PROJECT_ROOT}/_build/.build-version"
cat > "$VERSION_FILE" <<EOF
GIT_COMMIT="$GIT_COMMIT"
BUILD_DATE="$BUILD_DATE"
VERSION_TAG="$VERSION_TAG"
FRONTEND_IMAGE="${FRONTEND_FULL}"
BACKEND_IMAGE="${BACKEND_FULL}"
EOF

log_success "Version info saved to: $VERSION_FILE"
log_info ""
log_info "Next steps:"
log_info "  1. Load production data: ./scripts/load-prod-data.sh"
log_info "  2. Test containers: ./scripts/test-prod-containers.sh"
log_info "  3. Export tested images: ./scripts/export-tested-images.sh"

