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
if [[ "$*" == *"--load-prod-data"* ]]; then
    TEST_ARGS="--load-prod-data"
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
log_info "  ./scripts/deploy.sh --version $VERSION_TAG"
