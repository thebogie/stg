#!/bin/bash
# Deploy to Production - Simple One-Command Deploy
# Usage: ./scripts/deploy.sh --version v<commit>-<timestamp>

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

# Parse version
VERSION_TAG=""
while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            VERSION_TAG="$2"
            shift 2
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Usage: $0 --version v<commit>-<timestamp>"
            exit 1
            ;;
    esac
done

if [ -z "$VERSION_TAG" ]; then
    log_error "Version required!"
    echo "Usage: $0 --version v<commit>-<timestamp>"
    exit 1
fi

cd "$PROJECT_ROOT"

# Use existing deploy-production.sh (it's already good)
log_info "Deploying version: $VERSION_TAG"
./scripts/deploy-production.sh --version "$VERSION_TAG"

log_success "✅ Deployment complete!"
