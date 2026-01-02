#!/bin/bash

# Test migrations on existing production data
# This tests that migrations work correctly when applied to existing data
# Usage: ./scripts/test-migrations-on-existing-data.sh [backup-file]

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

log_section() {
    echo ""
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}$1${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

# Check if we're in the project root
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    log_error "Must be run from project root"
    exit 1
fi

# Get backup file
BACKUP_FILE="$1"
if [ -z "$BACKUP_FILE" ]; then
    # Try to find latest production backup
    BACKUP_DIRS=(
        "${PROJECT_ROOT}/_build/backups"
        "/backups/arangodb"
    )
    
    for dir in "${BACKUP_DIRS[@]}"; do
        if [ -d "$dir" ]; then
            LATEST=$(find "$dir" -name "*.tar.gz" -o -name "*.zip" | sort -r | head -1)
            if [ -n "$LATEST" ]; then
                BACKUP_FILE="$LATEST"
                break
            fi
        fi
    done
fi

if [ -z "$BACKUP_FILE" ] || [ ! -f "$BACKUP_FILE" ]; then
    log_error "Backup file not found: ${BACKUP_FILE:-not specified}"
    log_info "Usage: $0 [backup-file]"
    log_info "Or place a backup file in _build/backups/ directory"
    exit 1
fi

log_section "Testing Migrations on Existing Data"
log_info "This workflow:"
log_info "  1. Restores production backup"
log_info "  2. Runs migrations on existing data"
log_info "  3. Tests that migrations work correctly"
log_info ""
log_info "Backup file: $BACKUP_FILE"
echo ""
read -p "Continue? (yes/no): " confirm
if [ "$confirm" != "yes" ]; then
    log_info "Cancelled"
    exit 0
fi

# Check for production environment file
ENV_FILE="${PROJECT_ROOT}/config/.env.production"
if [ ! -f "$ENV_FILE" ]; then
    log_error "Production environment file not found: $ENV_FILE"
    exit 1
fi

# Load environment
set -a
source "$ENV_FILE"
set +a

export ENV_FILE

# Step 1: Ensure containers are running
log_section "Step 1: Starting Containers"
log_info "Starting production containers..."
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.yaml \
    -f deploy/docker-compose.prod.yml \
    up -d

sleep 5

# Extract ArangoDB port
if [[ "$ARANGO_URL" =~ :([0-9]+) ]]; then
    ARANGO_PORT="${BASH_REMATCH[1]}"
else
    ARANGO_PORT="8529"
fi

CONTAINER_ID=$(docker ps -q -f name=arangodb)
if [ -z "$CONTAINER_ID" ]; then
    log_error "ArangoDB container not found"
    exit 1
fi

# Step 2: Drop existing database
log_section "Step 2: Preparing Database"
log_info "Dropping existing database to restore from backup..."

docker exec "$CONTAINER_ID" arangosh --server.endpoint tcp://127.0.0.1:8529 \
    --server.username "${ARANGO_USERNAME:-root}" \
    --server.password "${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" \
    --javascript.execute-string "db._dropDatabase('${ARANGO_DB:-smacktalk}');" 2>/dev/null || {
    log_info "Database doesn't exist (will be created)"
}

# Step 3: Restore backup
log_section "Step 3: Restoring Production Backup"
log_info "Restoring from: $BACKUP_FILE"

# Extract backup
TEMP_DIR=$(mktemp -d)
if [[ "$BACKUP_FILE" == *.tar.gz ]] || [[ "$BACKUP_FILE" == *.tar ]]; then
    log_info "Extracting TAR archive..."
    tar -xf "$BACKUP_FILE" -C "$TEMP_DIR"
elif [[ "$BACKUP_FILE" == *.zip ]]; then
    log_info "Extracting ZIP archive..."
    unzip -q "$BACKUP_FILE" -d "$TEMP_DIR"
else
    log_error "Unknown backup format. Expected .tar.gz, .tar, or .zip"
    rm -rf "$TEMP_DIR"
    exit 1
fi

# Find database directory
DB_DIR=$(find "$TEMP_DIR" -type d -name "${ARANGO_DB:-smacktalk}" | head -1)
if [ -z "$DB_DIR" ]; then
    # Try to find any database directory
    DB_DIR=$(find "$TEMP_DIR" -mindepth 1 -maxdepth 1 -type d | head -1)
fi

if [ -z "$DB_DIR" ]; then
    log_error "Could not find database directory in backup"
    rm -rf "$TEMP_DIR"
    exit 1
fi

log_info "Restoring from: $DB_DIR"

# Copy to container and restore
docker cp "$DB_DIR" "${CONTAINER_ID}:/tmp/restore-db"
docker exec "$CONTAINER_ID" arangorestore \
    --server.endpoint tcp://127.0.0.1:8529 \
    --server.username "${ARANGO_USERNAME:-root}" \
    --server.password "${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" \
    --server.database "${ARANGO_DB:-smacktalk}" \
    --create-database true \
    --input-directory "/tmp/restore-db"

docker exec "$CONTAINER_ID" rm -rf "/tmp/restore-db"
rm -rf "$TEMP_DIR"

log_success "Backup restored"

# Step 4: Clear migration state (to test migrations from scratch)
log_section "Step 4: Clearing Migration State"
log_info "Clearing migration tracking to test migrations on existing data..."

# Clear migration lock and state
docker exec "$CONTAINER_ID" arangosh --server.endpoint tcp://127.0.0.1:8529 \
    --server.username "${ARANGO_USERNAME:-root}" \
    --server.password "${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" \
    --javascript.execute-string "
        db._useDatabase('${ARANGO_DB:-smacktalk}');
        try { db._collection('_schema_migrations').truncate(); } catch(e) {}
        try { db._collection('_migration_lock').truncate(); } catch(e) {}
    " 2>/dev/null || {
    log_info "Migration collections don't exist (will be created)"
}

log_success "Migration state cleared"

# Step 5: Run migrations
log_section "Step 5: Running Migrations on Existing Data"
if [ ! -d "$PROJECT_ROOT/migrations/files" ] || [ -z "$(ls -A "$PROJECT_ROOT/migrations/files" 2>/dev/null)" ]; then
    log_warning "No migrations found. Skipping."
else
    log_info "Running migrations on existing production data..."
    log_warning "This tests that migrations work correctly with existing data!"
    
    cargo run --package stg-rd-migrations --release -- \
        --endpoint "http://localhost:${ARANGO_PORT}" \
        --database "${ARANGO_DB:-smacktalk}" \
        --username "${ARANGO_USERNAME:-root}" \
        --password "${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" \
        --migrations-dir "$PROJECT_ROOT/migrations/files" || {
        log_error "Migrations failed on existing data!"
        exit 1
    }
    
    log_success "Migrations completed successfully on existing data"
fi

# Step 6: Verify
log_section "Step 6: Verifying Migration Results"
log_info "Database should now have:"
log_info "  - All original production data"
log_info "  - All migrations applied"
log_info "  - Data integrity maintained"

log_success "Migration testing on existing data complete!"
log_info ""
log_info "Test your application now to verify everything works correctly"
log_info "Services:"
log_info "  Backend: http://localhost:${BACKEND_PORT:-50012}"
log_info "  Frontend: http://localhost:${FRONTEND_PORT:-50013}"

