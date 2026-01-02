#!/bin/bash

# Test production containers locally
# This script starts production containers with production-like data for testing
# Usage: ./scripts/test-prod-containers.sh [--skip-data-load] [--run-tests]

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

# Parse arguments
SKIP_DATA_LOAD=false
RUN_TESTS=false
USE_TEST_DB=false
RESTORE_AFTER=false
BACKUP_BEFORE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-data-load)
            SKIP_DATA_LOAD=true
            shift
            ;;
        --run-tests)
            RUN_TESTS=true
            shift
            ;;
        --use-test-db)
            USE_TEST_DB=true
            shift
            ;;
        --backup-before)
            BACKUP_BEFORE=true
            shift
            ;;
        --restore-after)
            RESTORE_AFTER=true
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Usage: $0 [--skip-data-load] [--run-tests] [--use-test-db] [--backup-before] [--restore-after]"
            exit 1
            ;;
    esac
done

# Check if we're in the project root
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    log_error "Must be run from project root"
    exit 1
fi

# Check if images are built
VERSION_FILE="${PROJECT_ROOT}/_build/.build-version"
if [ ! -f "$VERSION_FILE" ]; then
    log_warning "No build version file found. Building images first..."
    "$PROJECT_ROOT/scripts/build-prod-images.sh"
fi

source "$VERSION_FILE"

log_info "Testing production containers"
log_info "  Version: $VERSION_TAG"
log_info "  Git Commit: $GIT_COMMIT"

# Check for production environment file
ENV_FILE="${PROJECT_ROOT}/config/.env.production"
if [ ! -f "$ENV_FILE" ]; then
    log_error "Production environment file not found: $ENV_FILE"
    log_info "Run: ./config/setup-env.sh production"
    exit 1
fi

export ENV_FILE

# Load environment variables
set -a
source "$ENV_FILE"
set +a

# Use test database if requested (prevents contaminating production data)
if [ "$USE_TEST_DB" = true ]; then
    TEST_DB="${ARANGO_DB:-smacktalk}_test"
    log_info "Using test database: $TEST_DB (to prevent contaminating production data)"
    export ARANGO_DB="$TEST_DB"
    # Update ENV_FILE temporarily
    export ARANGO_DB="$TEST_DB"
fi

# Create backup before testing if requested
BACKUP_FILE=""
if [ "$BACKUP_BEFORE" = true ] && [ "$SKIP_DATA_LOAD" = false ]; then
    log_info "Creating backup before testing..."
    BACKUP_DIR="${PROJECT_ROOT}/_build/backups/test-backups"
    mkdir -p "$BACKUP_DIR"
    TIMESTAMP=$(date +%Y%m%d-%H%M%S)
    BACKUP_FILE="${BACKUP_DIR}/pre-test-${TIMESTAMP}"
    
    # Extract ArangoDB port
    if [[ "$ARANGO_URL" =~ :([0-9]+) ]]; then
        ARANGO_PORT="${BASH_REMATCH[1]}"
    else
        ARANGO_PORT="8529"
    fi
    
    # Check if ArangoDB is already running
    if curl -s -f -u "${ARANGO_USERNAME:-root}:${ARANGO_PASSWORD:-}" "${ARANGO_URL}/_api/version" > /dev/null 2>&1; then
        CONTAINER_ID=$(docker ps -q -f name=arangodb)
        if [ -n "$CONTAINER_ID" ]; then
            docker exec "$CONTAINER_ID" arangodump \
                --server.endpoint tcp://127.0.0.1:8529 \
                --server.username "${ARANGO_USERNAME:-root}" \
                --server.password "${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" \
                --server.database "${ARANGO_DB:-smacktalk}" \
                --output-directory "/tmp/pre-test-${TIMESTAMP}" 2>/dev/null || true
            
            docker cp "${CONTAINER_ID}:/tmp/pre-test-${TIMESTAMP}" "$BACKUP_DIR/" 2>/dev/null || true
            docker exec "$CONTAINER_ID" rm -rf "/tmp/pre-test-${TIMESTAMP}" 2>/dev/null || true
        fi
    fi
    log_info "Backup saved (if database was running)"
fi

# Stop any existing containers
log_info "Stopping any existing containers..."
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.yaml \
    -f deploy/docker-compose.prod.yml \
    down 2>/dev/null || true

# Start production containers
log_info "Starting production containers..."
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.yaml \
    -f deploy/docker-compose.prod.yml \
    up -d

# Wait for services to be healthy
log_info "Waiting for services to be healthy..."
sleep 5

# Check health
MAX_WAIT=60
WAITED=0
while [ $WAITED -lt $MAX_WAIT ]; do
    if docker compose \
        --env-file "$ENV_FILE" \
        -f deploy/docker-compose.yaml \
        -f deploy/docker-compose.prod.yml \
        ps | grep -q "healthy\|running"; then
        break
    fi
    sleep 2
    WAITED=$((WAITED + 2))
done

# Load production data if not skipped
if [ "$SKIP_DATA_LOAD" = false ]; then
    log_info "Loading production data..."
    if [ -f "$PROJECT_ROOT/scripts/load-prod-data.sh" ]; then
        "$PROJECT_ROOT/scripts/load-prod-data.sh" || {
            log_warning "Data load failed or skipped. Continuing with empty database."
        }
    else
        log_warning "load-prod-data.sh not found. Skipping data load."
    fi
fi

# Run migrations if any
log_info "Checking for migrations..."
if [ -d "$PROJECT_ROOT/migrations/files" ] && [ -n "$(ls -A "$PROJECT_ROOT/migrations/files" 2>/dev/null)" ]; then
    log_info "Running migrations..."
    
    # Load environment variables
    set -a
    source "$ENV_FILE"
    set +a
    
    # Extract ArangoDB port
    if [[ "$ARANGO_URL" =~ :([0-9]+) ]]; then
        ARANGO_PORT="${BASH_REMATCH[1]}"
    else
        ARANGO_PORT="8529"
    fi
    
    # Run migrations
    cargo run --package stg-rd-migrations --release -- \
        --endpoint "http://localhost:${ARANGO_PORT}" \
        --database "${ARANGO_DB:-smacktalk}" \
        --username "${ARANGO_USERNAME:-root}" \
        --password "${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" \
        --migrations-dir "$PROJECT_ROOT/migrations/files" || {
        log_warning "Migrations failed. Check the output above."
    }
else
    log_info "No migrations found. Skipping."
fi

# Show container status
log_info "Container status:"
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.yaml \
    -f deploy/docker-compose.prod.yml \
    ps

# Test endpoints
log_info "Testing endpoints..."
BACKEND_PORT="${BACKEND_PORT:-50012}"
FRONTEND_PORT="${FRONTEND_PORT:-50013}"

if curl -s -f "http://localhost:${BACKEND_PORT}/health" > /dev/null; then
    log_success "Backend health check passed"
else
    log_warning "Backend health check failed"
fi

if curl -s -f "http://localhost:${FRONTEND_PORT}" > /dev/null; then
    log_success "Frontend is accessible"
else
    log_warning "Frontend health check failed"
fi

# Run tests if requested
if [ "$RUN_TESTS" = true ]; then
    log_info "Running test suite..."
    if [ -f "$PROJECT_ROOT/scripts/run_tests.sh" ]; then
        "$PROJECT_ROOT/scripts/run_tests.sh"
    else
        log_warning "run_tests.sh not found. Skipping automated tests."
    fi
fi

log_success "Production containers are running and ready for testing!"
log_info ""
log_info "Services:"
log_info "  Backend: http://localhost:${BACKEND_PORT}"
log_info "  Frontend: http://localhost:${FRONTEND_PORT}"
log_info "  ArangoDB: ${ARANGO_URL:-http://localhost:50011}"
if [ "$USE_TEST_DB" = true ]; then
    log_warning "  Using TEST database: ${ARANGO_DB} (data will be modified by tests)"
fi
log_info ""
log_info "To stop containers:"
log_info "  docker compose --env-file $ENV_FILE -f deploy/docker-compose.yaml -f deploy/docker-compose.prod.yml down"
log_info ""
log_info "To view logs:"
log_info "  docker compose --env-file $ENV_FILE -f deploy/docker-compose.yaml -f deploy/docker-compose.prod.yml logs -f"

# Restore from backup after testing if requested
if [ "$RESTORE_AFTER" = true ] && [ -n "$BACKUP_FILE" ] && [ -d "$BACKUP_FILE" ]; then
    log_info ""
    read -p "Press Enter when done testing to restore from backup, or Ctrl+C to keep changes..."
    log_info "Restoring from backup: $BACKUP_FILE"
    
    CONTAINER_ID=$(docker ps -q -f name=arangodb)
    if [ -n "$CONTAINER_ID" ]; then
        # Extract database name from backup
        DB_NAME=$(basename "$(find "$BACKUP_FILE" -type d -name "${ARANGO_DB:-smacktalk}" | head -1)" || echo "${ARANGO_DB:-smacktalk}")
        
        docker cp "$BACKUP_FILE" "${CONTAINER_ID}:/tmp/restore-backup" 2>/dev/null || true
        docker exec "$CONTAINER_ID" arangorestore \
            --server.endpoint tcp://127.0.0.1:8529 \
            --server.username "${ARANGO_USERNAME:-root}" \
            --server.password "${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" \
            --server.database "${ARANGO_DB:-smacktalk}" \
            --create-database true \
            --input-directory "/tmp/restore-backup/${DB_NAME}" 2>/dev/null || {
            log_warning "Restore failed. Data may have been modified."
        }
        docker exec "$CONTAINER_ID" rm -rf "/tmp/restore-backup" 2>/dev/null || true
        log_success "Database restored from backup"
    else
        log_warning "ArangoDB container not found. Cannot restore."
    fi
fi

