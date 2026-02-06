#!/bin/bash
# Start the production stack locally (same images as workflow) for visual testing.
# Run after ./scripts/workflow.sh or ./scripts/build.sh so _build/.build-version exists.
#
# Usage: ./scripts/start-stack-local.sh [--load-prod-data]
#   --load-prod-data  Load production data into ArangoDB (optional)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

BLUE='\033[0;34m'
GREEN='\033[0;32m'
NC='\033[0m'
log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }

cd "$PROJECT_ROOT"

VERSION_FILE="${PROJECT_ROOT}/_build/.build-version"
if [ ! -f "$VERSION_FILE" ]; then
    echo "❌ Build version file not found. Run ./scripts/build.sh or ./scripts/workflow.sh first."
    exit 1
fi

source "$VERSION_FILE"
ENV_FILE="${PROJECT_ROOT}/config/.env.production"
if [ ! -f "$ENV_FILE" ]; then
    echo "❌ Production env not found: $ENV_FILE"
    exit 1
fi

export ENV_FILE
export IMAGE_TAG="$VERSION_TAG"
export FRONTEND_IMAGE_TAG="$VERSION_TAG"

log_info "Starting stack (version: $VERSION_TAG)..."
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    up -d

log_info "Waiting for services to be healthy..."
MAX_WAIT=120
WAITED=0
while [ $WAITED -lt $MAX_WAIT ]; do
    if docker compose \
        --env-file "$ENV_FILE" \
        -f deploy/docker-compose.production.yml \
        ps 2>/dev/null | grep -q "healthy\|running"; then
        break
    fi
    sleep 2
    WAITED=$((WAITED + 2))
done

if [[ "${1:-}" == "--load-prod-data" ]]; then
    log_info "Loading production data..."
    ./scripts/load-prod-data.sh || log_info "Data load skipped or failed"
fi

source "$ENV_FILE"
FRONTEND_PORT="${FRONTEND_PORT:-50003}"
BACKEND_PORT="${BACKEND_PORT:-50002}"

log_success "Stack is up."
echo ""
echo "  Frontend:  http://localhost:${FRONTEND_PORT}"
echo "  Backend:   http://localhost:${BACKEND_PORT}"
echo ""
echo "  Stop:      docker compose --env-file $ENV_FILE -f deploy/docker-compose.production.yml down"
echo "  Logs:     docker compose --env-file $ENV_FILE -f deploy/docker-compose.production.yml logs -f"
echo ""
