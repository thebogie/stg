#!/bin/bash

# Test migrations workflow: Load prod data → Wipe → Run migrations → Test
# This ensures migrations work correctly from a clean state
# Usage: ./scripts/test-migrations-workflow.sh

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

log_section "Migration Testing Workflow"
log_info "This workflow:"
log_info "  1. Loads production data"
log_info "  2. Wipes the database (to test migrations from scratch)"
log_info "  3. Runs migrations"
log_info "  4. Tests the migrated database"
log_info ""
log_warning "This will DESTROY the current database and recreate it with migrations!"
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
    log_info "Run: ./config/setup-env.sh production"
    exit 1
fi

# Load environment
set -a
source "$ENV_FILE"
set +a

export ENV_FILE

# Step 1: Build production containers
log_section "Step 1: Building Production Containers"
if [ ! -f "$PROJECT_ROOT/_build/.build-version" ]; then
    log_info "Building production images..."
    "$PROJECT_ROOT/scripts/build-prod-images.sh"
else
    log_info "Images already built (found _build/.build-version)"
fi

# Step 2: Start containers
log_section "Step 2: Starting Production Containers"
log_info "Starting containers..."
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.yaml \
    -f deploy/docker-compose.prod.yml \
    up -d

# Wait for services
log_info "Waiting for services to be ready..."
sleep 5

# Extract ArangoDB port
if [[ "$ARANGO_URL" =~ :([0-9]+) ]]; then
    ARANGO_PORT="${BASH_REMATCH[1]}"
else
    ARANGO_PORT="8529"
fi

# Check connection
if ! curl -s -f -u "${ARANGO_USERNAME:-root}:${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" "${ARANGO_URL}/_api/version" > /dev/null 2>&1; then
    log_error "Cannot connect to ArangoDB"
    exit 1
fi

log_success "Containers are running"

# Step 3: Load production data (if available)
log_section "Step 3: Loading Production Data"
if [ -f "$PROJECT_ROOT/scripts/load-prod-data.sh" ]; then
    log_info "Loading production data snapshot..."
    "$PROJECT_ROOT/scripts/load-prod-data.sh" || {
        log_warning "Data load failed or skipped. Continuing with empty database."
    }
else
    log_warning "load-prod-data.sh not found. Starting with empty database."
fi

# Step 4: Create backup of loaded data (for reference)
log_section "Step 4: Creating Backup of Loaded Data"
BACKUP_DIR="${PROJECT_ROOT}/_build/backups/migration-test"
mkdir -p "$BACKUP_DIR"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BACKUP_FILE="${BACKUP_DIR}/pre-migration-${TIMESTAMP}"

CONTAINER_ID=$(docker ps -q -f name=arangodb)
if [ -n "$CONTAINER_ID" ]; then
    log_info "Creating backup before wiping..."
    docker exec "$CONTAINER_ID" arangodump \
        --server.endpoint tcp://127.0.0.1:8529 \
        --server.username "${ARANGO_USERNAME:-root}" \
        --server.password "${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" \
        --server.database "${ARANGO_DB:-smacktalk}" \
        --output-directory "/tmp/pre-migration-${TIMESTAMP}" 2>/dev/null || true
    
    docker cp "${CONTAINER_ID}:/tmp/pre-migration-${TIMESTAMP}" "$BACKUP_FILE" 2>/dev/null || true
    docker exec "$CONTAINER_ID" rm -rf "/tmp/pre-migration-${TIMESTAMP}" 2>/dev/null || true
    
    if [ -d "$BACKUP_FILE" ]; then
        log_success "Backup created: $BACKUP_FILE"
    fi
fi

# Step 5: Wipe database (drop and recreate)
log_section "Step 5: Wiping Database"
log_warning "Dropping database: ${ARANGO_DB:-smacktalk}"

CONTAINER_ID=$(docker ps -q -f name=arangodb)
if [ -n "$CONTAINER_ID" ]; then
    # Drop database
    docker exec "$CONTAINER_ID" arangosh --server.endpoint tcp://127.0.0.1:8529 \
        --server.username "${ARANGO_USERNAME:-root}" \
        --server.password "${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" \
        --javascript.execute-string "db._dropDatabase('${ARANGO_DB:-smacktalk}');" 2>/dev/null || {
        log_info "Database doesn't exist or couldn't be dropped (may not exist yet)"
    }
    
    log_success "Database wiped"
else
    log_error "ArangoDB container not found"
    exit 1
fi

# Step 6: Run migrations
log_section "Step 6: Running Migrations"
if [ ! -d "$PROJECT_ROOT/migrations/files" ] || [ -z "$(ls -A "$PROJECT_ROOT/migrations/files" 2>/dev/null)" ]; then
    log_warning "No migrations found. Skipping."
else
    log_info "Running migrations from scratch..."
    cargo run --package stg-rd-migrations --release -- \
        --endpoint "http://localhost:${ARANGO_PORT}" \
        --database "${ARANGO_DB:-smacktalk}" \
        --username "${ARANGO_USERNAME:-root}" \
        --password "${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" \
        --migrations-dir "$PROJECT_ROOT/migrations/files" || {
        log_error "Migrations failed!"
        exit 1
    }
    
    log_success "Migrations completed"
fi

# Step 7: Verify database structure
log_section "Step 7: Verifying Database Structure"
log_info "Checking database collections..."

CONTAINER_ID=$(docker ps -q -f name=arangodb)
if [ -n "$CONTAINER_ID" ]; then
    COLLECTIONS=$(docker exec "$CONTAINER_ID" arangosh --server.endpoint tcp://127.0.0.1:8529 \
        --server.username "${ARANGO_USERNAME:-root}" \
        --server.password "${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" \
        --javascript.execute-string "db._useDatabase('${ARANGO_DB:-smacktalk}'); db._collections().map(c => c.name())" 2>/dev/null | grep -v "^$" | grep -v "^---" || echo "")
    
    if [ -n "$COLLECTIONS" ]; then
        log_success "Collections created:"
        echo "$COLLECTIONS" | while read -r col; do
            if [ -n "$col" ]; then
                log_info "  - $col"
            fi
        done
    else
        log_warning "No collections found (database may be empty)"
    fi
fi

# Step 8: Test the application
log_section "Step 8: Testing Application"
log_info "Application should now be running with migrated database"
log_info ""
log_info "Services:"
log_info "  Backend: http://localhost:${BACKEND_PORT:-50012}"
log_info "  Frontend: http://localhost:${FRONTEND_PORT:-50013}"
log_info "  ArangoDB: ${ARANGO_URL}"
log_info ""
log_info "Test the application now. When done, you can:"
log_info "  1. Keep this migrated database for further testing"
log_info "  2. Restore the backup: ./scripts/restore-migration-test-backup.sh $BACKUP_FILE"
log_info "  3. Run migrations on the restored backup to test migration on existing data"

log_success "Migration testing workflow complete!"

