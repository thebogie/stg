#!/bin/bash
# Push Tested Images to Docker Hub
# Usage: ./scripts/push.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }
log_error() { echo -e "${RED}❌ $1${NC}"; }

cd "$PROJECT_ROOT"

# Load version
VERSION_FILE="${PROJECT_ROOT}/_build/.build-version"
if [ ! -f "$VERSION_FILE" ]; then
    log_error "Build version file not found!"
    log_info "Run ./scripts/build.sh first"
    exit 1
fi

source "$VERSION_FILE"
DOCKER_HUB_USER="${DOCKER_HUB_USER:-therealbogie}"

# Verify images exist
FRONTEND_LOCAL="stg_rd-frontend:${VERSION_TAG}"
BACKEND_LOCAL="stg_rd-backend:${VERSION_TAG}"

if ! docker image inspect "$FRONTEND_LOCAL" > /dev/null 2>&1; then
    log_error "Frontend image not found: $FRONTEND_LOCAL"
    exit 1
fi

if ! docker image inspect "$BACKEND_LOCAL" > /dev/null 2>&1; then
    log_error "Backend image not found: $BACKEND_LOCAL"
    exit 1
fi

# Tag for Docker Hub
FRONTEND_HUB="${DOCKER_HUB_USER}/stg_rd:frontend-${VERSION_TAG}"
BACKEND_HUB="${DOCKER_HUB_USER}/stg_rd:backend-${VERSION_TAG}"

log_info "Tagging images..."
docker tag "$FRONTEND_LOCAL" "$FRONTEND_HUB"
docker tag "$BACKEND_LOCAL" "$BACKEND_HUB"

# Verify WASM content before pushing
log_info "Verifying frontend image..."
WASM_FILES=$(docker run --rm "$FRONTEND_LOCAL" find /usr/share/nginx/html -name '*.wasm' -type f 2>/dev/null || echo "")
if [ -n "$WASM_FILES" ]; then
    for WASM_PATH in $WASM_FILES; do
        if docker run --rm "$FRONTEND_LOCAL" strings "$WASM_PATH" 2>/dev/null | grep -qi "Search People\|Search people"; then
            log_error "❌ CRITICAL: WASM contains 'Search People' - NOT pushing!"
            exit 1
        fi
    done
fi

# Push
log_info "Pushing frontend..."
if ! docker push "$FRONTEND_HUB"; then
    log_error "Failed to push frontend!"
    exit 1
fi

log_info "Pushing backend..."
if ! docker push "$BACKEND_HUB"; then
    log_error "Failed to push backend!"
    exit 1
fi

log_success ""
log_success "✅ Images pushed to Docker Hub"
log_success ""
log_info "Frontend: $FRONTEND_HUB"
log_info "Backend: $BACKEND_HUB"
log_info ""
log_info "To deploy:"
log_info "  ./scripts/deploy.sh --version $VERSION_TAG"
