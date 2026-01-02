#!/bin/bash

# Restore database from a test snapshot
# Usage: ./scripts/restore-test-snapshot.sh [snapshot-file]

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

# Get snapshot file
if [ -n "$1" ]; then
    SNAPSHOT_FILE="$1"
else
    # Use latest snapshot
    SNAPSHOT_DIR="${PROJECT_ROOT}/_build/backups/test-snapshots"
    LATEST_LINK="${SNAPSHOT_DIR}/latest-snapshot.tar.gz"
    if [ -f "$LATEST_LINK" ]; then
        SNAPSHOT_FILE=$(readlink -f "$LATEST_LINK")
    else
        log_error "No snapshot file provided and no latest snapshot found"
        log_info "Usage: $0 [snapshot-file]"
        exit 1
    fi
fi

if [ ! -f "$SNAPSHOT_FILE" ]; then
    log_error "Snapshot file not found: $SNAPSHOT_FILE"
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

log_warning "This will REPLACE the current database with the snapshot!"
log_info "  Database: ${ARANGO_DB:-smacktalk}"
log_info "  Snapshot: $SNAPSHOT_FILE"
echo ""
read -p "Are you sure? Type 'yes' to continue: " confirm
if [ "$confirm" != "yes" ]; then
    log_info "Restore cancelled"
    exit 0
fi

# Check if ArangoDB is running
if ! curl -s -f -u "${ARANGO_USERNAME:-root}:${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" "${ARANGO_URL}/_api/version" > /dev/null 2>&1; then
    log_error "Cannot connect to ArangoDB at ${ARANGO_URL}"
    log_info "Make sure ArangoDB is running"
    exit 1
fi

# Extract snapshot
TEMP_DIR=$(mktemp -d)
log_info "Extracting snapshot..."
tar -xzf "$SNAPSHOT_FILE" -C "$TEMP_DIR"

# Find database directory
DB_DIR=$(find "$TEMP_DIR" -type d -name "${ARANGO_DB:-smacktalk}" | head -1)
if [ -z "$DB_DIR" ]; then
    # Try to find any database directory
    DB_DIR=$(find "$TEMP_DIR" -mindepth 1 -maxdepth 1 -type d | head -1)
fi

if [ -z "$DB_DIR" ]; then
    log_error "Could not find database directory in snapshot"
    rm -rf "$TEMP_DIR"
    exit 1
fi

log_info "Restoring from: $DB_DIR"

# Use Docker container for restore
CONTAINER_ID=$(docker ps -q -f name=arangodb)
if [ -z "$CONTAINER_ID" ]; then
    log_error "ArangoDB container not found"
    rm -rf "$TEMP_DIR"
    exit 1
fi

# Copy to container
docker cp "$DB_DIR" "${CONTAINER_ID}:/tmp/restore-db"

# Restore
log_info "Restoring database..."
docker exec "$CONTAINER_ID" arangorestore \
    --server.endpoint tcp://127.0.0.1:8529 \
    --server.username "${ARANGO_USERNAME:-root}" \
    --server.password "${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" \
    --server.database "${ARANGO_DB:-smacktalk}" \
    --create-database true \
    --input-directory "/tmp/restore-db"

# Cleanup
docker exec "$CONTAINER_ID" rm -rf "/tmp/restore-db"
rm -rf "$TEMP_DIR"

log_success "Database restored from snapshot!"

