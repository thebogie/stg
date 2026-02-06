#!/bin/bash
# Clean Docker build cache and old project images so the next build doesn't reuse stale layers.
# Run before ./scripts/build.sh when you want a guaranteed fresh build (e.g. after Dockerfile changes).
#
# Usage: ./scripts/clean-docker-for-build.sh [--aggressive]
#   --aggressive  Also remove all local stg_rd-frontend and stg_rd-backend images (not just cache).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'
log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }
log_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }

cd "$PROJECT_ROOT"

AGGRESSIVE=""
for arg in "$@"; do
    [[ "$arg" == "--aggressive" ]] && AGGRESSIVE="1"
done

log_info "Stopping local stack so images are not in use..."
docker compose --env-file "${PROJECT_ROOT}/config/.env.production" -f deploy/docker-compose.production.yml down 2>/dev/null || true

log_info "Pruning build cache (forces next build to rebuild all layers)..."
docker builder prune -f
log_success "Build cache pruned"

log_info "Pruning dangling images..."
docker image prune -f
log_success "Dangling images pruned"

if [[ -n "$AGGRESSIVE" ]]; then
    log_warning "Removing all local stg_rd-frontend and stg_rd-backend images..."
    docker images --format '{{.Repository}}:{{.Tag}}' | grep -E '^stg_rd-(frontend|backend):' || true | while read -r img; do
        [[ -n "$img" ]] && docker rmi "$img" 2>/dev/null || true
    done
    docker images --format '{{.Repository}}:{{.Tag}}' | grep -E '^stg-(frontend|backend):' || true | while read -r img; do
        [[ -n "$img" ]] && docker rmi "$img" 2>/dev/null || true
    done
    log_success "Project images removed"
fi

log_success "Docker cleanup done. Next build will be fresh."
log_info "Run: ./scripts/build.sh"
