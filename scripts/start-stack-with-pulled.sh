#!/bin/bash
# Start the production stack locally using **pulled** images from Docker Hub.
# Use this to test the exact images you will deploy (catches WASM/nginx issues before production).
#
# Usage: ./scripts/start-stack-with-pulled.sh [VERSION_TAG] [--load-prod-data]
#   VERSION_TAG       e.g. v60059ed-20260205-163636 (default: from _build/.build-version)
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

# Parse args
LOAD_PROD_DATA=""
VERSION_TAG_ARG=""
for arg in "$@"; do
    if [[ "$arg" == "--load-prod-data" ]]; then
        LOAD_PROD_DATA="1"
    elif [[ -n "$arg" && "$arg" != "--"* ]]; then
        VERSION_TAG_ARG="$arg"
    fi
done

VERSION_FILE="${PROJECT_ROOT}/_build/.build-version"
if [[ -n "$VERSION_TAG_ARG" ]]; then
    VERSION_TAG="$VERSION_TAG_ARG"
elif [ -f "$VERSION_FILE" ]; then
    source "$VERSION_FILE"
else
    echo "❌ No VERSION_TAG given and _build/.build-version not found."
    echo "   Usage: $0 <VERSION_TAG> [--load-prod-data]"
    echo "   Example: $0 v60059ed-20260205-163636"
    exit 1
fi

ENV_FILE="${PROJECT_ROOT}/config/.env.production"
if [ ! -f "$ENV_FILE" ]; then
    echo "❌ Production env not found: $ENV_FILE"
    exit 1
fi

DOCKER_HUB_USER="${DOCKER_HUB_USER:-therealbogie}"
FRONTEND_HUB="${DOCKER_HUB_USER}/stg_rd:frontend-${VERSION_TAG}"
BACKEND_HUB="${DOCKER_HUB_USER}/stg_rd:backend-${VERSION_TAG}"

export ENV_FILE
export IMAGE_TAG="$VERSION_TAG"
export FRONTEND_IMAGE_TAG="$VERSION_TAG"
export FRONTEND_IMAGE="$FRONTEND_HUB"
export BACKEND_IMAGE="$BACKEND_HUB"

log_info "Pulling images (version: $VERSION_TAG)..."
docker pull "$FRONTEND_HUB" || { echo "❌ Failed to pull frontend. Is the image pushed?"; exit 1; }
docker pull "$BACKEND_HUB" || { echo "❌ Failed to pull backend. Is the image pushed?"; exit 1; }

log_info "Starting stack with pulled images (version: $VERSION_TAG)..."
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

if [[ -n "$LOAD_PROD_DATA" ]]; then
    log_info "Loading production data..."
    ./scripts/load-prod-data.sh || log_info "Data load skipped or failed"
fi

source "$ENV_FILE"
FRONTEND_PORT="${FRONTEND_PORT:-50003}"
BACKEND_PORT="${BACKEND_PORT:-50002}"

log_success "Stack is up (using pulled images)."
echo ""
echo "  Frontend:  http://localhost:${FRONTEND_PORT}"
echo "  Backend:   http://localhost:${BACKEND_PORT}"
echo ""
echo "  Stop:      docker compose --env-file $ENV_FILE -f deploy/docker-compose.production.yml down"
echo "  Logs:     docker compose --env-file $ENV_FILE -f deploy/docker-compose.production.yml logs -f"
echo ""
