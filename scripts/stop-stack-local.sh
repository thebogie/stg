#!/bin/bash
# Stop the production stack started with start-stack-local.sh.
#
# Usage: ./scripts/stop-stack-local.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

BLUE='\033[0;34m'
GREEN='\033[0;32m'
NC='\033[0m'
log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }

cd "$PROJECT_ROOT"

ENV_FILE="${PROJECT_ROOT}/config/.env.production"
if [ ! -f "$ENV_FILE" ]; then
    echo "❌ Production env not found: $ENV_FILE"
    exit 1
fi

VERSION_FILE="${PROJECT_ROOT}/_build/.build-version"
if [ -f "$VERSION_FILE" ]; then
    source "$VERSION_FILE"
    log_info "Stopping stack (version: ${VERSION_TAG:-unknown})..."
else
    log_info "Stopping production stack..."
fi

docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    down

log_success "Stack stopped."
