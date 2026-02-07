#!/bin/bash
# Industry-Standard CI/CD Workflow
# Build → Test → Push
# Usage: ./scripts/workflow.sh [--load-prod-data]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }
log_error() { echo -e "${RED}❌ $1${NC}"; }
log_step() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}▶ $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

cd "$PROJECT_ROOT"

# Parse args
TEST_ARGS=""
FAST=false
if [[ "$*" == *"--load-prod-data"* ]]; then
    TEST_ARGS="--load-prod-data"
fi
if [[ "$*" == *"--fast"* ]]; then
    FAST=true
fi

# Step -1: Git sanity checks (ensure correct code is built)
log_step "STEP -1: Git sanity checks"
git status -sb
if [ -n "$(git status --porcelain)" ]; then
    log_error "Working directory has uncommitted changes!"
    log_error "Commit or stash before running workflow.sh to ensure correct code is built."
    exit 1
fi
log_success "Git working tree is clean"

# Step 0: Docker cleanup (build cache + dangling images) so build doesn't reuse stale layers
log_step "STEP 0: Docker cleanup (fresh build)"
if [ "$FAST" = true ]; then
    log_info "Fast mode enabled: skipping aggressive Docker cleanup"
else
    if ! ./scripts/clean-docker-for-build.sh --aggressive; then
        log_error "Docker cleanup failed!"
        exit 1
    fi
fi

# Step 1: Build
log_step "STEP 1: Building Production Images"
if ! ./scripts/build.sh; then
    log_error "Build failed!"
    exit 1
fi

# Load version
source "$PROJECT_ROOT/_build/.build-version"
VERSION_TAG="$VERSION_TAG"

# Step 1.5: Image provenance checks (correct code)
log_step "STEP 1.5: Image provenance checks"
EXPECTED_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
EXPECTED_SOURCE_HASH=$(git rev-parse HEAD:frontend/src/pages/contests.rs 2>/dev/null || echo "unknown")
log_info "Expected commit: $EXPECTED_COMMIT"
log_info "Expected source hash (contests.rs): $EXPECTED_SOURCE_HASH"

# Verify frontend version.json in image (fallback to labels if needed)
FRONTEND_IMG="stg_rd-frontend:${VERSION_TAG}"
VERSION_JSON=$(docker run --rm "$FRONTEND_IMG" cat /usr/share/nginx/html/version.json 2>/dev/null || true)
IMAGE_COMMIT=$(echo "$VERSION_JSON" | grep -o '"git_commit":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
IMAGE_SOURCE_HASH=$(echo "$VERSION_JSON" | grep -o '"source_hash":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
if [ "$IMAGE_COMMIT" = "unknown" ]; then
    LABEL_COMMIT=$(docker image inspect "$FRONTEND_IMG" --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' 2>/dev/null || echo "")
    [ -n "$LABEL_COMMIT" ] && IMAGE_COMMIT="$LABEL_COMMIT"
fi
if [ "$IMAGE_SOURCE_HASH" = "unknown" ]; then
    LABEL_HASH=$(docker image inspect "$FRONTEND_IMG" --format '{{ index .Config.Labels "org.opencontainers.image.source_hash" }}' 2>/dev/null || echo "")
    [ -n "$LABEL_HASH" ] && IMAGE_SOURCE_HASH="$LABEL_HASH"
fi
if [ -z "$VERSION_JSON" ]; then
    log_info "version.json not found in frontend image; using labels for provenance"
fi
log_info "Image git_commit: $IMAGE_COMMIT"
log_info "Image source_hash: $IMAGE_SOURCE_HASH"
if [ "$IMAGE_COMMIT" = "unknown" ]; then
    log_warning "Image git_commit is unknown; skipping strict provenance check"
else
    if [ "$IMAGE_COMMIT" != "$EXPECTED_COMMIT" ]; then
        log_error "Image git_commit mismatch! Expected $EXPECTED_COMMIT, got $IMAGE_COMMIT"
        exit 1
    fi
fi
if [ "$IMAGE_SOURCE_HASH" = "unknown" ]; then
    log_warning "Image source_hash is unknown; skipping strict provenance check"
else
    if [ "$IMAGE_SOURCE_HASH" != "$EXPECTED_SOURCE_HASH" ]; then
        log_error "Image source_hash mismatch! Expected $EXPECTED_SOURCE_HASH, got $IMAGE_SOURCE_HASH"
        exit 1
    fi
fi
log_success "Image provenance checks passed (with warnings if metadata missing)"

# Step 2: Test
log_step "STEP 2: Testing Production Containers"
if ! ./scripts/test.sh $TEST_ARGS; then
    log_error "Tests failed!"
    exit 1
fi

# Step 3: Push
log_step "STEP 3: Pushing to Docker Hub"
if ! ./scripts/push.sh; then
    log_error "Push failed!"
    exit 1
fi

# Success
log_step "✅ Workflow Complete!"
log_success ""
log_success "Version: $VERSION_TAG"
log_success ""
log_info "To deploy to production:"
log_info "  ./scripts/deploy-production.sh --version $VERSION_TAG"
