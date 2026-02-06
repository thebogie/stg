#!/bin/bash
# Build Production Images - Industry Standard
# Usage: ./scripts/build.sh

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

cd "$PROJECT_ROOT"

# Load build info
source "$PROJECT_ROOT/scripts/build-info.sh"
VERSION_TAG="v${GIT_COMMIT}-$(date +%Y%m%d-%H%M%S)"

log_info "Building version: $VERSION_TAG"

# Call existing build script (reuse proven logic)
./scripts/build-prod-images.sh

log_success "✅ Build complete: $VERSION_TAG"
