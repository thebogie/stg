#!/bin/bash

# Backup production ArangoDB database
# This script creates a backup of the production database before deployment
# Usage: ./scripts/backup-prod-db.sh [--remote SERVER] [--output-dir DIR]

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
REMOTE_SERVER=""
OUTPUT_DIR=""
LOCAL_BACKUP=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --remote)
            REMOTE_SERVER="$2"
            shift 2
            ;;
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --local)
            LOCAL_BACKUP=true
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Usage: $0 [--remote SERVER] [--output-dir DIR] [--local]"
            exit 1
            ;;
    esac
done

# Set default output directory
if [ -z "$OUTPUT_DIR" ]; then
    if [ "$LOCAL_BACKUP" = true ]; then
        OUTPUT_DIR="${PROJECT_ROOT}/_build/backups"
    else
        OUTPUT_DIR="/backups/arangodb"
    fi
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Timestamp for backup
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BACKUP_NAME="arangodb-backup-${TIMESTAMP}"
BACKUP_PATH="${OUTPUT_DIR}/${BACKUP_NAME}"

if [ -n "$REMOTE_SERVER" ]; then
    # Remote backup - need to SSH to production server
    log_info "Creating backup on remote server: $REMOTE_SERVER"
    
    # Load production environment from remote or use defaults
    log_info "Connecting to remote server to create backup..."
    
    # You'll need to provide these via environment or arguments
    ARANGO_URL="${ARANGO_URL:-http://localhost:50011}"
    ARANGO_DB="${ARANGO_DB:-smacktalk}"
    ARANGO_USERNAME="${ARANGO_USERNAME:-root}"
    ARANGO_PASSWORD="${ARANGO_PASSWORD:-}"
    
    if [ -z "$ARANGO_PASSWORD" ]; then
        log_error "ARANGO_PASSWORD must be set for remote backup"
        log_info "Set it via: export ARANGO_PASSWORD=your_password"
        exit 1
    fi
    
    # Create backup on remote server
    ssh "$REMOTE_SERVER" "mkdir -p $OUTPUT_DIR && \
        docker exec \$(docker ps -q -f name=arangodb) arangodump \
        --server.endpoint tcp://127.0.0.1:8529 \
        --server.username $ARANGO_USERNAME \
        --server.password $ARANGO_PASSWORD \
        --server.database $ARANGO_DB \
        --output-directory /tmp/$BACKUP_NAME && \
        docker cp \$(docker ps -q -f name=arangodb):/tmp/$BACKUP_NAME $OUTPUT_DIR/ && \
        docker exec \$(docker ps -q -f name=arangodb) rm -rf /tmp/$BACKUP_NAME && \
        tar -czf $OUTPUT_DIR/${BACKUP_NAME}.tar.gz -C $OUTPUT_DIR $BACKUP_NAME && \
        rm -rf $OUTPUT_DIR/$BACKUP_NAME"
    
    log_success "Backup created on remote server: ${OUTPUT_DIR}/${BACKUP_NAME}.tar.gz"
    log_info "To download: scp ${REMOTE_SERVER}:${OUTPUT_DIR}/${BACKUP_NAME}.tar.gz ./_build/backups/"
    
elif [ "$LOCAL_BACKUP" = true ]; then
    # Local backup - backup local ArangoDB
    log_info "Creating local backup..."
    
    # Load environment
    ENV_FILE="${PROJECT_ROOT}/config/.env.development"
    # Load environment variables
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    source "${SCRIPT_DIR}/load-env.sh"
    
    # ARANGO_URL is set by load-env.sh
    ARANGO_DB="${ARANGO_DB:-smacktalk}"
    ARANGO_USERNAME="${ARANGO_USERNAME:-root}"
    ARANGO_PASSWORD="${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}"
    
    # Extract port from URL
    if [[ "$ARANGO_URL" =~ :([0-9]+) ]]; then
        ARANGO_PORT="${BASH_REMATCH[1]}"
    else
        ARANGO_PORT="8529"
    fi
    
    # Check if ArangoDB is running locally
    if ! curl -s -f -u "${ARANGO_USERNAME}:${ARANGO_PASSWORD}" "${ARANGO_URL}/_api/version" > /dev/null 2>&1; then
        log_error "Cannot connect to ArangoDB at ${ARANGO_URL}"
        log_info "Make sure ArangoDB is running"
        exit 1
    fi
    
    # Try to use arangodump directly
    if command -v arangodump > /dev/null 2>&1; then
        log_info "Using local arangodump..."
        arangodump \
            --server.endpoint "tcp://localhost:${ARANGO_PORT}" \
            --server.username "$ARANGO_USERNAME" \
            --server.password "$ARANGO_PASSWORD" \
            --server.database "$ARANGO_DB" \
            --output-directory "$BACKUP_PATH"
    else
        # Use Docker container
        log_info "Using Docker container for backup..."
        CONTAINER_ID=$(docker ps -q -f name=arangodb)
        if [ -z "$CONTAINER_ID" ]; then
            log_error "ArangoDB container not found"
            exit 1
        fi
        
        docker exec "$CONTAINER_ID" arangodump \
            --server.endpoint tcp://127.0.0.1:8529 \
            --server.username "$ARANGO_USERNAME" \
            --server.password "$ARANGO_PASSWORD" \
            --server.database "$ARANGO_DB" \
            --output-directory "/tmp/$BACKUP_NAME"
        
        docker cp "${CONTAINER_ID}:/tmp/$BACKUP_NAME" "$OUTPUT_DIR/"
        docker exec "$CONTAINER_ID" rm -rf "/tmp/$BACKUP_NAME"
    fi
    
    # Compress backup
    log_info "Compressing backup..."
    tar -czf "${BACKUP_PATH}.tar.gz" -C "$OUTPUT_DIR" "$BACKUP_NAME"
    rm -rf "$BACKUP_PATH"
    
    log_success "Local backup created: ${BACKUP_PATH}.tar.gz"
    
else
    # Production backup - run on production server
    log_info "Creating production backup..."
    
    # Load production environment
    ENV_FILE="${PROJECT_ROOT}/config/.env.production"
    if [ ! -f "$ENV_FILE" ]; then
        log_error "Production environment file not found: $ENV_FILE"
        exit 1
    fi
    
    set -a
    source "$ENV_FILE"
    set +a
    
    ARANGO_URL="${ARANGO_URL:-http://localhost:50011}"
    ARANGO_DB="${ARANGO_DB:-smacktalk}"
    ARANGO_USERNAME="${ARANGO_USERNAME:-root}"
    ARANGO_PASSWORD="${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}"
    
    # Extract port from URL
    if [[ "$ARANGO_URL" =~ :([0-9]+) ]]; then
        ARANGO_PORT="${BASH_REMATCH[1]}"
    else
        ARANGO_PORT="8529"
    fi
    
    # Use Docker container for backup
    CONTAINER_ID=$(docker ps -q -f name=arangodb)
    if [ -z "$CONTAINER_ID" ]; then
        log_error "ArangoDB container not found"
        log_info "Make sure you're running this on the production server"
        exit 1
    fi
    
    log_info "Creating backup from container: $CONTAINER_ID"
    
    docker exec "$CONTAINER_ID" arangodump \
        --server.endpoint tcp://127.0.0.1:8529 \
        --server.username "$ARANGO_USERNAME" \
        --server.password "$ARANGO_PASSWORD" \
        --server.database "$ARANGO_DB" \
        --output-directory "/tmp/$BACKUP_NAME"
    
    docker cp "${CONTAINER_ID}:/tmp/$BACKUP_NAME" "$OUTPUT_DIR/"
    docker exec "$CONTAINER_ID" rm -rf "/tmp/$BACKUP_NAME"
    
    # Compress backup
    log_info "Compressing backup..."
    tar -czf "${BACKUP_PATH}.tar.gz" -C "$OUTPUT_DIR" "$BACKUP_NAME"
    rm -rf "$BACKUP_PATH"
    
    log_success "Production backup created: ${BACKUP_PATH}.tar.gz"
    log_info "Backup size: $(du -h "${BACKUP_PATH}.tar.gz" | cut -f1)"
fi

# Create symlink to latest backup
LATEST_LINK="${OUTPUT_DIR}/latest-backup.tar.gz"
if [ -f "${BACKUP_PATH}.tar.gz" ]; then
    ln -sf "${BACKUP_NAME}.tar.gz" "$LATEST_LINK"
    log_info "Latest backup link: $LATEST_LINK"
fi

log_success "Backup completed successfully!"

