#!/bin/bash

# Quick Development Tests
# Fast, basic tests for development workflow - runs unit tests only
# No Docker containers, no integration tests - just quick feedback during coding
#
# Usage: ./scripts/test-dev.sh

set -euo pipefail

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

cd "$PROJECT_ROOT"

# Check if we're in the project root
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    log_error "Must be run from project root"
    exit 1
fi

log_step "Quick Development Tests"
log_info "Fast unit tests only - no Docker, no integration tests"
log_info "For comprehensive tests against production containers, run:"
log_info "  ./scripts/run-tests-setup-prod.sh"
echo ""

# Run backend library unit tests (fast, no dependencies)
log_step "Backend Unit Tests (Library)"
log_info "Running backend library unit tests..."
if cargo nextest run --workspace --lib; then
    log_success "Backend unit tests passed"
else
    log_error "Backend unit tests failed!"
    exit 1
fi

log_step "✅ Development Tests Complete!"
log_success "All unit tests passed"
log_info ""
log_info "These are quick development tests. For full production testing, run:"
log_info "  ./scripts/run-tests-setup-prod.sh"
log_info ""
