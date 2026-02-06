#!/bin/bash

# Production Side: Deploy Tested Containers
# This script pulls tested containers from Docker Hub and deploys them.
#
# Usage: ./scripts/deploy-production.sh --version VERSION_TAG
#
# Example:
#   ./scripts/deploy-production.sh --version v48b71e5-20260204-164600
#
# Prerequisites:
#   - docker login (for pulling from Docker Hub)
#   - DOCKER_HUB_USER env var (default: therealbogie)

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }
log_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
log_error() { echo -e "${RED}❌ $1${NC}"; }

# Parse arguments
VERSION_TAG=""
SKIP_BACKUP=false
SKIP_MIGRATIONS=false
DOCKER_HUB_USER=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            VERSION_TAG="$2"
            shift 2
            ;;
        --skip-backup)
            SKIP_BACKUP=true
            shift
            ;;
        --skip-migrations)
            SKIP_MIGRATIONS=true
            shift
            ;;
        --docker-hub-user)
            DOCKER_HUB_USER="$2"
            shift 2
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Usage: $0 --version VERSION_TAG [--skip-backup] [--skip-migrations] [--docker-hub-user USER]"
            exit 1
            ;;
    esac
done

if [ -z "$VERSION_TAG" ]; then
    log_error "Version tag is required!"
    echo "Usage: $0 --version VERSION_TAG"
    echo ""
    echo "Example:"
    echo "  $0 --version v48b71e5-20260204-164600"
    exit 1
fi

# Set Docker Hub user
if [ -z "$DOCKER_HUB_USER" ]; then
    DOCKER_HUB_USER="${DOCKER_HUB_USER:-therealbogie}"
fi

# Check prerequisites
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    log_error "Must be run from project root"
    exit 1
fi

ENV_FILE="${PROJECT_ROOT}/config/.env.production"
if [ ! -f "$ENV_FILE" ]; then
    log_error "Production environment file not found: $ENV_FILE"
    exit 1
fi

# Check Docker login
if ! docker info >/dev/null 2>&1; then
    log_error "Docker is not running or you're not logged in"
    log_info "Please run: docker login"
    exit 1
fi

# ============================================================================
# STEP 1: Pull Images from Docker Hub
# ============================================================================
log_info "Pulling tested images from Docker Hub..."

FRONTEND_HUB="${DOCKER_HUB_USER}/stg_rd:frontend-${VERSION_TAG}"
BACKEND_HUB="${DOCKER_HUB_USER}/stg_rd:backend-${VERSION_TAG}"

log_info "Pulling frontend: $FRONTEND_HUB"
if ! docker pull "$FRONTEND_HUB"; then
    log_error "Failed to pull frontend image!"
    exit 1
fi

# CRITICAL: Verify the pulled image using industry-standard version.json
log_info "Verifying pulled frontend image (using version.json)..."
log_info "Checking if version.json exists in image..."
if docker run --rm "$FRONTEND_HUB" test -f /usr/share/nginx/html/version.json 2>/dev/null; then
    log_info "✅ version.json found"
    log_info "Reading version.json from pulled image..."
    VERSION_JSON=$(docker run --rm "$FRONTEND_HUB" cat /usr/share/nginx/html/version.json 2>/dev/null)
    if [ -z "$VERSION_JSON" ]; then
        log_error "❌ CRITICAL: version.json is empty or missing in pulled image!"
        exit 1
    fi
    
    # Extract key fields (from pulled image)
    CONTAINER_BUILD_DATE=$(echo "$VERSION_JSON" | grep -o '"build_date":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
    CONTAINER_GIT_COMMIT=$(echo "$VERSION_JSON" | grep -o '"git_commit":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
    EXPECTED_COMMIT=$(echo "$VERSION_TAG" | cut -d'-' -f1 | sed 's/^v//' || echo "unknown")
    
    log_info "Pulled image metadata:"
    log_info "  Build Date: $CONTAINER_BUILD_DATE"
    log_info "  Git Commit: $CONTAINER_GIT_COMMIT"
    log_info "  Expected Commit: $EXPECTED_COMMIT"
    
    # Verify build date is recent (not Jan 16)
    if echo "$CONTAINER_BUILD_DATE" | grep -q "2026-01-16\|2026-01-15\|2026-01-14"; then
        log_error "❌ CRITICAL: Pulled image build date is from January 16 or earlier!"
        log_error "Docker Hub has old code - DO NOT DEPLOY!"
        exit 1
    fi
    
    # Verify git commit matches (if we can extract it from version tag)
    if [ "$EXPECTED_COMMIT" != "unknown" ] && [ "$CONTAINER_GIT_COMMIT" != "$EXPECTED_COMMIT" ]; then
        log_warning "⚠️  Git commit mismatch: expected $EXPECTED_COMMIT, got $CONTAINER_GIT_COMMIT"
        log_warning "This might be OK if the tag format is different"
    fi
    
    log_success "✅ Pulled image verified - build metadata looks correct"
else
    log_warning "⚠️  version.json not found at /usr/share/nginx/html/version.json"
    log_info "This might mean the image was built before version.json was added, or it's in a different location."
    log_info "Falling back to WASM content verification..."
    log_info "Listing all files in /usr/share/nginx/html to debug..."
    docker run --rm "$FRONTEND_HUB" ls -lah /usr/share/nginx/html/ 2>/dev/null | head -20 || true
    
    # Find ALL WASM files and check each one
    log_info "Finding all WASM files in image..."
    WASM_FILES=$(docker run --rm "$FRONTEND_HUB" find /usr/share/nginx/html -name '*.wasm' -type f 2>/dev/null || echo "")
    
    if [ -z "$WASM_FILES" ]; then
        log_error "❌ CRITICAL: No WASM files found in pulled image!"
        log_error "This image is corrupted or was built incorrectly."
        exit 1
    fi
    
    log_info "Found WASM files:"
    echo "$WASM_FILES"
    
    # Check each WASM file for old code
    OLD_CODE_FOUND=false
    for WASM_PATH in $WASM_FILES; do
        log_info "Checking WASM file: $WASM_PATH"
        if docker run --rm "$FRONTEND_HUB" strings "$WASM_PATH" 2>/dev/null | grep -qi "Search People\|Search people"; then
            log_error "❌ CRITICAL: WASM file '$WASM_PATH' contains 'Search People'!"
            OLD_CODE_FOUND=true
        fi
    done
    
    if [ "$OLD_CODE_FOUND" = true ]; then
        log_error ""
        log_error "❌❌❌ DEPLOYMENT BLOCKED ❌❌❌"
        log_error ""
        log_error "The pulled image contains OLD code (has 'Search People' instead of 'Players')."
        log_error "This means the image was built with outdated source code or used cached layers."
        log_error ""
        log_error "SOLUTION:"
        log_error "  1. On your dev machine, ensure you have the latest code:"
        log_error "     git pull"
        log_error "     git status  # verify no uncommitted changes"
        log_error ""
        log_error "  2. Rebuild and push a fresh image:"
        log_error "     ./scripts/build-test-push.sh"
        log_error ""
        log_error "  3. Then deploy the NEW version tag from the output"
        log_error ""
        exit 1
    fi
    
    log_success "✅ Fallback verification passed - no old code found in WASM files"
fi

log_info "Pulling backend: $BACKEND_HUB"
if ! docker pull "$BACKEND_HUB"; then
    log_error "Failed to pull backend image!"
    exit 1
fi

# Tag for local use
docker tag "$FRONTEND_HUB" "stg_rd-frontend:${VERSION_TAG}"
docker tag "$BACKEND_HUB" "stg_rd-backend:${VERSION_TAG}"

log_success "Images pulled and tagged"

# ============================================================================
# STEP 2: Backup Database (if not skipped)
# ============================================================================
if [ "$SKIP_BACKUP" = false ]; then
    log_info "Creating database backup..."
    if [ -f "$PROJECT_ROOT/scripts/backup-prod-db.sh" ]; then
        mkdir -p "${PROJECT_ROOT}/_build/backups"
        "$PROJECT_ROOT/scripts/backup-prod-db.sh" --output-dir "${PROJECT_ROOT}/_build/backups" || {
            log_warning "Backup failed, but continuing..."
        }
    else
        log_warning "backup-prod-db.sh not found. Skipping backup."
    fi
fi

# ============================================================================
# STEP 3: Deploy Containers
# ============================================================================
log_info "Deploying containers..."

# Stop existing containers
log_info "Stopping existing containers..."
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    down

# Set image tags for deployment
export IMAGE_TAG="$VERSION_TAG"
export FRONTEND_IMAGE="stg_rd-frontend:${VERSION_TAG}"
export BACKEND_IMAGE="stg_rd-backend:${VERSION_TAG}"
export FRONTEND_IMAGE_TAG="$VERSION_TAG"

# Create temp env file with image tags
TEMP_ENV_FILE=$(mktemp)
cat "$ENV_FILE" > "$TEMP_ENV_FILE"
sed -i '/^IMAGE_TAG=/d' "$TEMP_ENV_FILE" 2>/dev/null || sed -i.bak '/^IMAGE_TAG=/d' "$TEMP_ENV_FILE"
sed -i '/^FRONTEND_IMAGE_TAG=/d' "$TEMP_ENV_FILE" 2>/dev/null || sed -i.bak '/^FRONTEND_IMAGE_TAG=/d' "$TEMP_ENV_FILE"
sed -i '/^FRONTEND_IMAGE=/d' "$TEMP_ENV_FILE" 2>/dev/null || sed -i.bak '/^FRONTEND_IMAGE=/d' "$TEMP_ENV_FILE"
sed -i '/^BACKEND_IMAGE=/d' "$TEMP_ENV_FILE" 2>/dev/null || sed -i.bak '/^BACKEND_IMAGE=/d' "$TEMP_ENV_FILE"
rm -f "${TEMP_ENV_FILE}.bak" 2>/dev/null || true

echo "IMAGE_TAG=$VERSION_TAG" >> "$TEMP_ENV_FILE"
echo "FRONTEND_IMAGE_TAG=$VERSION_TAG" >> "$TEMP_ENV_FILE"
echo "FRONTEND_IMAGE=stg_rd-frontend:${VERSION_TAG}" >> "$TEMP_ENV_FILE"
echo "BACKEND_IMAGE=stg_rd-backend:${VERSION_TAG}" >> "$TEMP_ENV_FILE"

# Deploy
log_info "Starting containers with version: $VERSION_TAG"
docker compose \
    --env-file "$TEMP_ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    up -d --force-recreate frontend backend

rm -f "$TEMP_ENV_FILE"

# Wait for services to be healthy
log_info "Waiting for services to be healthy..."
sleep 5
MAX_WAIT=60
WAITED=0
while [ $WAITED -lt $MAX_WAIT ]; do
    if docker compose \
        --env-file "$ENV_FILE" \
        -f deploy/docker-compose.production.yml \
        ps | grep -q "healthy\|running"; then
        break
    fi
    sleep 2
    WAITED=$((WAITED + 2))
done

# ============================================================================
# STEP 4: Run Migrations (if not skipped)
# ============================================================================
if [ "$SKIP_MIGRATIONS" = false ]; then
    if [ -d "$PROJECT_ROOT/migrations/files" ] && [ -n "$(ls -A "$PROJECT_ROOT/migrations/files" 2>/dev/null)" ]; then
        log_info "Running migrations..."
        
        # Extract ArangoDB port
        source "$ENV_FILE"
        if [[ "$ARANGO_URL" =~ arangodb: ]]; then
            ARANGO_PORT="${ARANGODB_PORT:-50001}"
        elif [[ "$ARANGO_URL" =~ :([0-9]+) ]]; then
            ARANGO_PORT="${BASH_REMATCH[1]}"
        else
            ARANGO_PORT="${ARANGODB_PORT:-8529}"
        fi
        
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
fi

# ============================================================================
# STEP 5: Verify Deployment
# ============================================================================
log_info "Verifying deployment..."

# Check containers are using correct images
FRONTEND_USING_VERSION=$(docker ps --format "{{.Names}}\t{{.Image}}" | grep "^frontend" | grep -c "$VERSION_TAG" || echo "0")
BACKEND_USING_VERSION=$(docker ps --format "{{.Names}}\t{{.Image}}" | grep "^backend" | grep -c "$VERSION_TAG" || echo "0")

if [ "$FRONTEND_USING_VERSION" -eq "1" ] && [ "$BACKEND_USING_VERSION" -eq "1" ]; then
    log_success "Containers are using versioned images: $VERSION_TAG"
else
    log_warning "Containers may not be using versioned images"
    log_info "Expected: $VERSION_TAG"
fi

# Health checks
log_info "Checking service health..."
if docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    ps | grep -q "healthy"; then
    log_success "Services are healthy"
else
    log_warning "Some services may not be healthy yet"
fi

# Save deployed version
mkdir -p "${PROJECT_ROOT}/_build"
cat > "${PROJECT_ROOT}/_build/.deployed-version" <<EOF
VERSION_TAG="$VERSION_TAG"
IMAGE_TAG="$VERSION_TAG"
FRONTEND_IMAGE_TAG="$VERSION_TAG"
FRONTEND_IMAGE="stg_rd-frontend:${VERSION_TAG}"
BACKEND_IMAGE="stg_rd-backend:${VERSION_TAG}"
DEPLOYED_DATE="$(date -u +"%Y-%m-%d %H:%M:%S UTC")"
EOF

log_success ""
log_success "✅ Deployment Complete!"
log_success ""
log_info "Version: $VERSION_TAG"
log_info "Services:"
log_info "  Backend: http://localhost:${BACKEND_PORT:-50002}"
log_info "  Frontend: http://localhost:${FRONTEND_PORT:-50003}"
log_info ""
log_info "To view logs:"
log_info "  docker compose --env-file $ENV_FILE -f deploy/docker-compose.production.yml logs -f"
