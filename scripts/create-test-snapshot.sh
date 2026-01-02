#!/bin/bash

# Create a snapshot of the current database state before testing
# This allows you to restore to a clean state after testing
# Usage: ./scripts/create-test-snapshot.sh

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check if we're in the project root
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    log_error "Must be run from project root"
    exit 1
fi

# Load environment
ENV_FILE="${PROJECT_ROOT}/config/.env.production"
if [ ! -f "$ENV_FILE" ]; then
    log_error "Production environment file not found: $ENV_FILE"
    exit 1
fi

set -a
source "$ENV_FILE"
set +a

# Check if ArangoDB is running
if ! curl -s -f -u "${ARANGO_USERNAME:-root}:${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" "${ARANGO_URL}/_api/version" > /dev/null 2>&1; then
    log_error "Cannot connect to ArangoDB at ${ARANGO_URL}"
    log_info "Make sure ArangoDB is running"
    exit 1
fi

# Create snapshot directory
SNAPSHOT_DIR="${PROJECT_ROOT}/_build/backups/test-snapshots"
mkdir -p "$SNAPSHOT_DIR"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
SNAPSHOT_PATH="${SNAPSHOT_DIR}/snapshot-${TIMESTAMP}"

log_info "Creating database snapshot..."
log_info "  Database: ${ARANGO_DB:-smacktalk}"
log_info "  Snapshot: $SNAPSHOT_PATH"

# Extract port from URL
if [[ "$ARANGO_URL" =~ :([0-9]+) ]]; then
    ARANGO_PORT="${BASH_REMATCH[1]}"
else
    ARANGO_PORT="8529"
fi

# Use Docker container for snapshot
CONTAINER_ID=$(docker ps -q -f name=arangodb)
if [ -z "$CONTAINER_ID" ]; then
    log_error "ArangoDB container not found"
    exit 1
fi

# Create snapshot
docker exec "$CONTAINER_ID" arangodump \
    --server.endpoint tcp://127.0.0.1:8529 \
    --server.username "${ARANGO_USERNAME:-root}" \
    --server.password "${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" \
    --server.database "${ARANGO_DB:-smacktalk}" \
    --output-directory "/tmp/snapshot-${TIMESTAMP}"

# Copy snapshot out of container
docker cp "${CONTAINER_ID}:/tmp/snapshot-${TIMESTAMP}" "$SNAPSHOT_PATH"
docker exec "$CONTAINER_ID" rm -rf "/tmp/snapshot-${TIMESTAMP}"

# Compress snapshot
log_info "Compressing snapshot..."
tar -czf "${SNAPSHOT_PATH}.tar.gz" -C "$SNAPSHOT_DIR" "snapshot-${TIMESTAMP}"
rm -rf "$SNAPSHOT_PATH"

# Create symlink to latest
LATEST_LINK="${SNAPSHOT_DIR}/latest-snapshot.tar.gz"
ln -sf "snapshot-${TIMESTAMP}.tar.gz" "$LATEST_LINK"

log_success "Snapshot created: ${SNAPSHOT_PATH}.tar.gz"
log_info "Snapshot size: $(du -h "${SNAPSHOT_PATH}.tar.gz" | cut -f1)"
log_info ""
log_info "To restore this snapshot:"
log_info "  ./scripts/restore-test-snapshot.sh ${SNAPSHOT_PATH}.tar.gz"

