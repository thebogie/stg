#!/bin/bash

# Production deployment with migrations
# This is the production deployment workflow:
# 1. Backup current production database
# 2. Deploy tested images
# 3. Restore production backup (if needed)
# 4. Run migrations on existing data
# Usage: ./scripts/deploy-with-migrations.sh --version TAG [--skip-backup] [--skip-restore]

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

# Parse arguments
VERSION_TAG=""
IMAGE_DIR="/tmp"
SKIP_BACKUP=false
SKIP_RESTORE=false
RESTORE_BACKUP=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            VERSION_TAG="$2"
            shift 2
            ;;
        --image-dir)
            IMAGE_DIR="$2"
            shift 2
            ;;
        --skip-backup)
            SKIP_BACKUP=true
            shift
            ;;
        --skip-restore)
            SKIP_RESTORE=true
            shift
            ;;
        --restore-backup)
            RESTORE_BACKUP="$2"
            shift 2
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Usage: $0 --version TAG [--image-dir DIR] [--skip-backup] [--skip-restore] [--restore-backup FILE]"
            exit 1
            ;;
    esac
done

if [ -z "$VERSION_TAG" ]; then
    log_error "Version tag required"
    echo "Usage: $0 --version TAG"
    exit 1
fi

# Check if we're in the project root
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    log_error "Must be run from project root"
    exit 1
fi

log_section "Production Deployment with Migrations"
log_info "Version: $VERSION_TAG"
log_warning "This will deploy to PRODUCTION!"

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

# Step 1: Backup production database
if [ "$SKIP_BACKUP" = false ]; then
    log_section "Step 1: Backing Up Production Database"
    log_info "Creating backup before deployment..."
    if [ -f "$PROJECT_ROOT/scripts/backup-prod-db.sh" ]; then
        "$PROJECT_ROOT/scripts/backup-prod-db.sh" || {
            log_error "Backup failed! Aborting deployment."
            exit 1
        }
    else
        log_error "backup-prod-db.sh not found"
        exit 1
    fi
else
    log_warning "Skipping backup (--skip-backup)"
fi

# Step 2: Deploy tested images
log_section "Step 2: Deploying Tested Images"
log_info "Deploying version: $VERSION_TAG"

if [ -f "$PROJECT_ROOT/scripts/deploy-tested-images.sh" ]; then
    # Deploy but skip migrations (we'll handle them separately)
    "$PROJECT_ROOT/scripts/deploy-tested-images.sh" \
        --version "$VERSION_TAG" \
        --image-dir "$IMAGE_DIR" \
        --skip-backup \
        --skip-migrations || {
        log_error "Deployment failed!"
        exit 1
    }
else
    log_error "deploy-tested-images.sh not found"
    exit 1
fi

# Step 3: Restore backup if needed
# Note: In normal deployment, data is already in volumes, so we don't need to restore
# This step is only if you're deploying to a fresh server or need to restore from a specific backup
if [ -n "$RESTORE_BACKUP" ]; then
    log_section "Step 3: Restoring Specific Backup"
    log_warning "Restoring from: $RESTORE_BACKUP"
    log_warning "This will REPLACE the current database!"
    echo ""
    read -p "Type 'restore' to confirm: " confirm
    if [ "$confirm" = "restore" ]; then
        # Use the restore script from test-migrations-on-existing-data.sh logic
        CONTAINER_ID=$(docker ps -q -f name=arangodb)
        if [ -n "$CONTAINER_ID" ]; then
            # Extract and restore backup
            TEMP_DIR=$(mktemp -d)
            if [[ "$RESTORE_BACKUP" == *.tar.gz ]] || [[ "$RESTORE_BACKUP" == *.tar ]]; then
                tar -xf "$RESTORE_BACKUP" -C "$TEMP_DIR"
            elif [[ "$RESTORE_BACKUP" == *.zip ]]; then
                unzip -q "$RESTORE_BACKUP" -d "$TEMP_DIR"
            fi
            
            DB_DIR=$(find "$TEMP_DIR" -type d -name "${ARANGO_DB:-smacktalk}" | head -1)
            if [ -z "$DB_DIR" ]; then
                DB_DIR=$(find "$TEMP_DIR" -mindepth 1 -maxdepth 1 -type d | head -1)
            fi
            
            if [ -n "$DB_DIR" ]; then
                docker exec "$CONTAINER_ID" arangosh --server.endpoint tcp://127.0.0.1:8529 \
                    --server.username "${ARANGO_USERNAME:-root}" \
                    --server.password "${ARANGO_PASSWORD:-${ARANGO_ROOT_PASSWORD:-}}" \
                    --javascript.execute-string "db._dropDatabase('${ARANGO_DB:-smacktalk}');" 2>/dev/null || true
                
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
            fi
        fi
    fi
elif [ "$SKIP_RESTORE" = false ]; then
    log_section "Step 3: Database State"
    log_info "Using existing production data from volumes"
    log_info "Data persists in: ${VOLUME_PATH}/arango_data"
    log_info "No restore needed - data is already there"
fi

# Step 4: Run migrations on existing production data
log_section "Step 4: Running Migrations on Production Data"
if [ ! -d "$PROJECT_ROOT/migrations/files" ] || [ -z "$(ls -A "$PROJECT_ROOT/migrations/files" 2>/dev/null)" ]; then
    log_info "No migrations found. Skipping."
else
    log_info "Running migrations on existing production data..."
    log_warning "Migrations will modify the production database!"
    echo ""
    read -p "Continue with migrations? (yes/no): " confirm
    if [ "$confirm" != "yes" ]; then
        log_warning "Migrations skipped. Run manually when ready."
    else
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
            log_error "Migrations failed on production!"
            log_error "You may need to restore from backup"
            exit 1
        }
        
        log_success "Migrations completed successfully"
    fi
fi

# Step 5: Verify deployment
log_section "Step 5: Verifying Deployment"
log_info "Checking service health..."

BACKEND_PORT="${BACKEND_PORT:-50012}"
FRONTEND_PORT="${FRONTEND_PORT:-50013}"

if curl -s -f "http://localhost:${BACKEND_PORT}/health" > /dev/null; then
    log_success "Backend is healthy"
else
    log_warning "Backend health check failed"
fi

if curl -s -f "http://localhost:${FRONTEND_PORT}" > /dev/null; then
    log_success "Frontend is accessible"
else
    log_warning "Frontend health check failed"
fi

log_section "✅ Deployment Complete"
log_success "Version: $VERSION_TAG"
log_success "Migrations: Applied to existing production data"
log_info ""
log_info "Services:"
log_info "  Backend: http://localhost:${BACKEND_PORT}"
log_info "  Frontend: http://localhost:${FRONTEND_PORT}"
log_info ""
log_info "Monitor logs:"
log_info "  docker compose --env-file $ENV_FILE -f deploy/docker-compose.yaml -f deploy/docker-compose.prod.yml logs -f"

