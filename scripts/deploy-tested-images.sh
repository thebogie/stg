#!/bin/bash

# Deploy tested images to production
# This script loads and deploys the tested images on the production server
# Usage: ./scripts/deploy-tested-images.sh [--version TAG] [--image-dir DIR] [--skip-backup] [--skip-migrations] [--skip-load] [--docker-hub-user USER]
#
# When --skip-load is used, the script will pull images from Docker Hub to ensure
# the latest version is used (not stale local cache). Set DOCKER_HUB_USER env var
# or use --docker-hub-user to specify the Docker Hub username (default: therealbogie)

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
VERSION_TAG=""
IMAGE_DIR="/tmp"
SKIP_BACKUP=false
SKIP_MIGRATIONS=false
SKIP_LOAD=false
DOCKER_HUB_USER=""

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
        --skip-migrations)
            SKIP_MIGRATIONS=true
            shift
            ;;
        --skip-load)
            SKIP_LOAD=true
            shift
            ;;
        --docker-hub-user)
            DOCKER_HUB_USER="$2"
            shift 2
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Usage: $0 [--version TAG] [--image-dir DIR] [--skip-backup] [--skip-migrations] [--skip-load] [--docker-hub-user USER]"
            exit 1
            ;;
    esac
done

# Set Docker Hub user (default: therealbogie, can be overridden by env var or --docker-hub-user)
# If not set via --docker-hub-user, check environment variable, then use default
if [ -z "$DOCKER_HUB_USER" ]; then
    # Check if DOCKER_HUB_USER env var is set (before we assigned the script variable)
    # Use env var if set, otherwise default to therealbogie
    if [ -n "${DOCKER_HUB_USER_ENV:-}" ]; then
        DOCKER_HUB_USER="$DOCKER_HUB_USER_ENV"
    else
        DOCKER_HUB_USER="therealbogie"
    fi
fi

# Check if we're in the project root
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    log_error "Must be run from project root"
    exit 1
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

# Ensure ENV_FILE is an absolute path for docker-compose
# (docker-compose resolves paths relative to where it's run)
if [[ ! "$ENV_FILE" =~ ^/ ]]; then
    ENV_FILE="${PROJECT_ROOT}/${ENV_FILE}"
fi

export ENV_FILE

# If version not provided, try to find latest in image dir
if [ -z "$VERSION_TAG" ]; then
    log_info "Version tag not provided, looking for latest in $IMAGE_DIR..."
    LATEST_FRONTEND=$(ls -t "${IMAGE_DIR}"/frontend-v*.tar.gz 2>/dev/null | head -1)
    if [ -n "$LATEST_FRONTEND" ]; then
        VERSION_TAG=$(echo "$LATEST_FRONTEND" | sed 's/.*frontend-\(.*\)\.tar\.gz/\1/')
        log_info "Found version: $VERSION_TAG"
    else
        log_error "Could not find image files in $IMAGE_DIR"
        log_info "Please specify version: --version TAG"
        exit 1
    fi
fi

log_info "Deploying tested images to production"
log_info "  Version: $VERSION_TAG"
log_info "  Image directory: $IMAGE_DIR"

# Backup database before deployment
if [ "$SKIP_BACKUP" = false ]; then
    log_info "Creating database backup before deployment..."
    if [ -f "$PROJECT_ROOT/scripts/backup-prod-db.sh" ]; then
        # Use local backup directory to avoid permission issues
        mkdir -p "${PROJECT_ROOT}/_build/backups"
        "$PROJECT_ROOT/scripts/backup-prod-db.sh" --output-dir "${PROJECT_ROOT}/_build/backups" || {
            log_warning "Backup failed, but continuing..."
        }
    else
        log_warning "backup-prod-db.sh not found. Skipping backup."
    fi
fi

# Load images (skip if --skip-load is used, e.g., when images are already loaded from Docker Hub)
if [ "$SKIP_LOAD" = false ]; then
    FRONTEND_IMAGE_FILE="${IMAGE_DIR}/frontend-${VERSION_TAG}.tar.gz"
    BACKEND_IMAGE_FILE="${IMAGE_DIR}/backend-${VERSION_TAG}.tar.gz"

    if [ ! -f "$FRONTEND_IMAGE_FILE" ]; then
        log_error "Frontend image file not found: $FRONTEND_IMAGE_FILE"
        log_info "If images are already loaded from Docker Hub, use --skip-load flag"
        exit 1
    fi

    if [ ! -f "$BACKEND_IMAGE_FILE" ]; then
        log_error "Backend image file not found: $BACKEND_IMAGE_FILE"
        log_info "If images are already loaded from Docker Hub, use --skip-load flag"
        exit 1
    fi

    # Verify checksums if available
    if [ -f "${FRONTEND_IMAGE_FILE}.sha256" ]; then
        log_info "Verifying frontend image checksum..."
        # Run checksum verification from the image directory
        (cd "$IMAGE_DIR" && sha256sum -c "$(basename "${FRONTEND_IMAGE_FILE}.sha256")") || {
            log_error "Frontend image checksum verification failed!"
            exit 1
        }
    fi

    if [ -f "${BACKEND_IMAGE_FILE}.sha256" ]; then
        log_info "Verifying backend image checksum..."
        # Run checksum verification from the image directory
        (cd "$IMAGE_DIR" && sha256sum -c "$(basename "${BACKEND_IMAGE_FILE}.sha256")") || {
            log_error "Backend image checksum verification failed!"
            exit 1
        }
    fi

    # Load images
    log_info "Loading frontend image..."
    gunzip -c "$FRONTEND_IMAGE_FILE" | docker load
    log_success "Frontend image loaded"

    log_info "Loading backend image..."
    gunzip -c "$BACKEND_IMAGE_FILE" | docker load
    log_success "Backend image loaded"
else
    log_info "Skipping image load from files (will pull from Docker Hub instead)"
    
    # When --skip-load is used, we assume images are on Docker Hub
    # Pull them to ensure we have the latest version (not stale local cache)
    FRONTEND_HUB="${DOCKER_HUB_USER}/stg_rd:frontend-${VERSION_TAG}"
    BACKEND_HUB="${DOCKER_HUB_USER}/stg_rd:backend-${VERSION_TAG}"
    
    log_info "Pulling images from Docker Hub to ensure latest version..."
    log_info "  Docker Hub user: $DOCKER_HUB_USER"
    log_info "  Frontend: $FRONTEND_HUB"
    log_info "  Backend: $BACKEND_HUB"
    
    # Pull frontend image (show output to debug issues)
    log_info "Attempting to pull frontend image..."
    if docker pull "$FRONTEND_HUB"; then
        log_success "Frontend image pulled from Docker Hub"
        # Tag it for local use
        docker tag "$FRONTEND_HUB" "stg_rd-frontend:${VERSION_TAG}" 2>/dev/null || true
        # Verify the pulled image has correct WASM files (for debugging)
        log_info "Verifying pulled image..."
        WASM_DATE=$(docker run --rm --entrypoint sh "$FRONTEND_HUB" -c 'stat /usr/share/nginx/html/frontend_bg.optimized.wasm 2>/dev/null | grep Modify | cut -d" " -f2-3' 2>/dev/null || echo "unknown")
        log_info "WASM file date in pulled image: $WASM_DATE"
    else
        log_error "Failed to pull frontend image from Docker Hub: $FRONTEND_HUB"
        log_warning "Will try to use locally cached image if available (may be stale!)"
    fi
    
    # Pull backend image (show output to debug issues)
    log_info "Attempting to pull backend image..."
    if docker pull "$BACKEND_HUB"; then
        log_success "Backend image pulled from Docker Hub"
        # Tag it for local use
        docker tag "$BACKEND_HUB" "stg_rd-backend:${VERSION_TAG}" 2>/dev/null || true
    else
        log_error "Failed to pull backend image from Docker Hub: $BACKEND_HUB"
        log_warning "Will try to use locally cached image if available (may be stale!)"
    fi
fi

# Get image names - try common names first, then fall back to project name
COMPOSE_PROJECT_NAME=$(basename "$PROJECT_ROOT" | tr '[:upper:]' '[:lower:]' | tr -cd '[:alnum:]-')

# Try to find the actual image names that were loaded
# Common names: stg_rd-frontend, stg-frontend, or project-name-frontend
FRONTEND_IMAGE=""
BACKEND_IMAGE=""

# Check for stg_rd images first (most common)
if docker image inspect "stg_rd-frontend:latest" > /dev/null 2>&1; then
    FRONTEND_IMAGE="stg_rd-frontend:${VERSION_TAG}"
    # Tag with version if not already tagged
    if ! docker image inspect "$FRONTEND_IMAGE" > /dev/null 2>&1; then
        docker tag "stg_rd-frontend:latest" "$FRONTEND_IMAGE" 2>/dev/null || true
    fi
elif docker image inspect "stg-frontend:latest" > /dev/null 2>&1; then
    FRONTEND_IMAGE="stg-frontend:${VERSION_TAG}"
    if ! docker image inspect "$FRONTEND_IMAGE" > /dev/null 2>&1; then
        docker tag "stg-frontend:latest" "$FRONTEND_IMAGE" 2>/dev/null || true
    fi
else
    # Fall back to project name
    FRONTEND_IMAGE="${COMPOSE_PROJECT_NAME}-frontend:${VERSION_TAG}"
fi

if docker image inspect "stg_rd-backend:latest" > /dev/null 2>&1; then
    BACKEND_IMAGE="stg_rd-backend:${VERSION_TAG}"
    if ! docker image inspect "$BACKEND_IMAGE" > /dev/null 2>&1; then
        docker tag "stg_rd-backend:latest" "$BACKEND_IMAGE" 2>/dev/null || true
    fi
elif docker image inspect "stg-backend:latest" > /dev/null 2>&1; then
    BACKEND_IMAGE="stg-backend:${VERSION_TAG}"
    if ! docker image inspect "$BACKEND_IMAGE" > /dev/null 2>&1; then
        docker tag "stg-backend:latest" "$BACKEND_IMAGE" 2>/dev/null || true
    fi
else
    # Fall back to project name
    BACKEND_IMAGE="${COMPOSE_PROJECT_NAME}-backend:${VERSION_TAG}"
fi

# Verify images are loaded
if ! docker image inspect "$FRONTEND_IMAGE" > /dev/null 2>&1; then
    log_warning "Could not find $FRONTEND_IMAGE, checking for 'latest' tag..."
    # Try latest tag
    FRONTEND_BASE=$(echo "$FRONTEND_IMAGE" | cut -d: -f1)
    if docker image inspect "${FRONTEND_BASE}:latest" > /dev/null 2>&1; then
        FRONTEND_IMAGE="${FRONTEND_BASE}:latest"
        log_info "Using ${FRONTEND_IMAGE} (will tag as ${FRONTEND_BASE}:${VERSION_TAG})"
        docker tag "$FRONTEND_IMAGE" "${FRONTEND_BASE}:${VERSION_TAG}" 2>/dev/null || true
        FRONTEND_IMAGE="${FRONTEND_BASE}:${VERSION_TAG}"
    else
        log_error "Frontend image not found after loading"
        log_info "Available images:"
        docker images | grep -E "frontend|stg_rd" || true
        exit 1
    fi
fi

if ! docker image inspect "$BACKEND_IMAGE" > /dev/null 2>&1; then
    log_warning "Could not find $BACKEND_IMAGE, checking for 'latest' tag..."
    # Try latest tag
    BACKEND_BASE=$(echo "$BACKEND_IMAGE" | cut -d: -f1)
    if docker image inspect "${BACKEND_BASE}:latest" > /dev/null 2>&1; then
        BACKEND_IMAGE="${BACKEND_BASE}:latest"
        log_info "Using ${BACKEND_IMAGE} (will tag as ${BACKEND_BASE}:${VERSION_TAG})"
        docker tag "$BACKEND_IMAGE" "${BACKEND_BASE}:${VERSION_TAG}" 2>/dev/null || true
        BACKEND_IMAGE="${BACKEND_BASE}:${VERSION_TAG}"
    else
        log_error "Backend image not found after loading"
        log_info "Available images:"
        docker images | grep -E "backend|stg_rd" || true
        exit 1
    fi
fi

log_success "Images loaded:"
log_info "  Frontend: $FRONTEND_IMAGE"
log_info "  Backend: $BACKEND_IMAGE"

# Save deployed version to file so prod-compose.sh (used by systemctl) can use it
mkdir -p "${PROJECT_ROOT}/_build"
DEPLOYED_VERSION_FILE="${PROJECT_ROOT}/_build/.deployed-version"
cat > "$DEPLOYED_VERSION_FILE" <<EOF
# Deployed version - used by prod-compose.sh for systemctl restarts
# This file is created by deploy-tested-images.sh and read by prod-compose.sh
VERSION_TAG="$VERSION_TAG"
IMAGE_TAG="$VERSION_TAG"
FRONTEND_IMAGE_TAG="$VERSION_TAG"
FRONTEND_IMAGE="$FRONTEND_IMAGE"
BACKEND_IMAGE="$BACKEND_IMAGE"
DEPLOYED_DATE="$(date -u +"%Y-%m-%d %H:%M:%S UTC")"
EOF
log_info "Saved deployed version to: $DEPLOYED_VERSION_FILE"
log_info "  This ensures systemctl restarts use the correct version"

# Set image tags for docker compose (docker-compose.yaml will use these)
# Always use versioned tags - don't fall back to "latest"
export IMAGE_TAG="$VERSION_TAG"
export FRONTEND_IMAGE="stg_rd-frontend:${VERSION_TAG}"
export BACKEND_IMAGE="stg_rd-backend:${VERSION_TAG}"
export FRONTEND_IMAGE_TAG="$VERSION_TAG"

log_info "Using images:"
log_info "  FRONTEND_IMAGE: $FRONTEND_IMAGE"
log_info "  BACKEND_IMAGE: $BACKEND_IMAGE"
log_info "  IMAGE_TAG: $IMAGE_TAG"
log_info "  FRONTEND_IMAGE_TAG: $FRONTEND_IMAGE_TAG"

# Verify the images exist locally
log_info "Verifying images exist locally..."
if ! docker image inspect "$FRONTEND_IMAGE" >/dev/null 2>&1; then
    log_error "Frontend image not found: $FRONTEND_IMAGE"
    log_info "Available frontend images:"
    docker images | grep frontend | head -5 || true
    exit 1
fi

if ! docker image inspect "$BACKEND_IMAGE" >/dev/null 2>&1; then
    log_error "Backend image not found: $BACKEND_IMAGE"
    log_info "Available backend images:"
    docker images | grep backend | head -5 || true
    exit 1
fi

log_success "Both images exist locally"

# Ensure Docker network exists (for external networks)
log_info "Ensuring Docker network exists..."
if ! docker network inspect stg_prod >/dev/null 2>&1; then
    log_info "Creating Docker network: stg_prod"
    docker network create stg_prod || {
        log_warning "Failed to create network stg_prod (may already exist)"
    }
fi

# Stop existing containers
log_info "Stopping existing containers..."
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    down

# Deploy new containers (docker-compose.production.yml will use IMAGE_TAG or FRONTEND_IMAGE/BACKEND_IMAGE)
log_info "Deploying new containers..."
log_info "Using IMAGE_TAG: $IMAGE_TAG"
log_info "Using FRONTEND_IMAGE: $FRONTEND_IMAGE"
log_info "Using BACKEND_IMAGE: $BACKEND_IMAGE"

# Ensure IMAGE_TAG is exported so docker-compose can use it
export IMAGE_TAG="$VERSION_TAG"
export FRONTEND_IMAGE_TAG="${FRONTEND_IMAGE_TAG:-$VERSION_TAG}"

# Create a temporary env file with IMAGE_TAG to ensure it's available to docker-compose
# This ensures IMAGE_TAG is definitely available even if not in the original env file
TEMP_ENV_FILE=$(mktemp)
cat "$ENV_FILE" > "$TEMP_ENV_FILE"
# Remove any existing IMAGE_TAG lines to avoid duplicates
sed -i '/^IMAGE_TAG=/d' "$TEMP_ENV_FILE" 2>/dev/null || sed -i.bak '/^IMAGE_TAG=/d' "$TEMP_ENV_FILE"
sed -i '/^FRONTEND_IMAGE_TAG=/d' "$TEMP_ENV_FILE" 2>/dev/null || sed -i.bak '/^FRONTEND_IMAGE_TAG=/d' "$TEMP_ENV_FILE"
sed -i '/^FRONTEND_IMAGE=/d' "$TEMP_ENV_FILE" 2>/dev/null || sed -i.bak '/^FRONTEND_IMAGE=/d' "$TEMP_ENV_FILE"
sed -i '/^BACKEND_IMAGE=/d' "$TEMP_ENV_FILE" 2>/dev/null || sed -i.bak '/^BACKEND_IMAGE=/d' "$TEMP_ENV_FILE"
rm -f "${TEMP_ENV_FILE}.bak" 2>/dev/null || true

# Append IMAGE_TAG variables (will override if already in file)
echo "IMAGE_TAG=$VERSION_TAG" >> "$TEMP_ENV_FILE"
echo "FRONTEND_IMAGE_TAG=$FRONTEND_IMAGE_TAG" >> "$TEMP_ENV_FILE"
echo "FRONTEND_IMAGE=$FRONTEND_IMAGE" >> "$TEMP_ENV_FILE"
echo "BACKEND_IMAGE=$BACKEND_IMAGE" >> "$TEMP_ENV_FILE"

log_info "Deploying with docker-compose..."
log_info "Temp env file contains:"
grep -E "^(IMAGE_TAG|FRONTEND_IMAGE|BACKEND_IMAGE)=" "$TEMP_ENV_FILE" || true

log_info "Forcing recreation to ensure versioned images are used..."
docker compose \
    --env-file "$TEMP_ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    up -d --force-recreate frontend backend

# Clean up temp file
rm -f "$TEMP_ENV_FILE"

# Verify containers are using the correct image tags
log_info "Verifying deployed containers..."
sleep 3

log_info "Container images:"
docker ps --format "{{.Names}}\t{{.Image}}" | grep -E "^(frontend|backend)" || true

# Check if containers are using versioned images
FRONTEND_USING_VERSION=$(docker ps --format "{{.Names}}\t{{.Image}}" | grep "^frontend" | grep -c "$VERSION_TAG" || echo "0")
BACKEND_USING_VERSION=$(docker ps --format "{{.Names}}\t{{.Image}}" | grep "^backend" | grep -c "$VERSION_TAG" || echo "0")

if [ "$FRONTEND_USING_VERSION" -eq "1" ] && [ "$BACKEND_USING_VERSION" -eq "1" ]; then
    log_success "Containers are using versioned images: $VERSION_TAG"
else
    log_warning "Containers may not be using versioned images"
    log_info "Expected: $VERSION_TAG"
    log_info "If containers show 'latest', they need to be restarted with the versioned images"
    log_info "You may need to manually restart:"
    log_info "  docker compose --env-file $ENV_FILE -f deploy/docker-compose.production.yml up -d --force-recreate"
fi

# Verify environment variables in running containers
log_info ""
log_info "Checking IMAGE_TAG in running containers..."
if docker exec backend sh -c 'echo $IMAGE_TAG' 2>/dev/null | grep -q "$VERSION_TAG"; then
    log_success "Backend container has IMAGE_TAG=$VERSION_TAG"
else
    BACKEND_TAG=$(docker exec backend sh -c 'echo $IMAGE_TAG' 2>/dev/null || echo "not set")
    log_warning "Backend IMAGE_TAG is: ${BACKEND_TAG:-not set} (expected: $VERSION_TAG)"
fi

if docker exec frontend sh -c 'echo $FRONTEND_IMAGE_TAG' 2>/dev/null | grep -q "$VERSION_TAG"; then
    log_success "Frontend container has FRONTEND_IMAGE_TAG=$VERSION_TAG"
else
    FRONTEND_TAG=$(docker exec frontend sh -c 'echo $FRONTEND_IMAGE_TAG' 2>/dev/null || echo "not set")
    log_warning "Frontend FRONTEND_IMAGE_TAG is: ${FRONTEND_TAG:-not set} (expected: $VERSION_TAG)"
fi

# Wait for services to be healthy
log_info "Waiting for services to be healthy..."
sleep 5

# Check health
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

# Run migrations if not skipped
if [ "$SKIP_MIGRATIONS" = false ]; then
    if [ -d "$PROJECT_ROOT/migrations/files" ] && [ -n "$(ls -A "$PROJECT_ROOT/migrations/files" 2>/dev/null)" ]; then
        log_info "Running migrations..."
        
        # Extract ArangoDB port for migrations (run from host, so use external port)
        # If ARANGO_URL uses container name (arangodb), use ARANGODB_PORT (external port)
        # Otherwise, extract port from URL
        if [[ "$ARANGO_URL" =~ arangodb: ]]; then
            # Using container name, so use external port
            ARANGO_PORT="${ARANGODB_PORT:-50003}"
        elif [[ "$ARANGO_URL" =~ :([0-9]+) ]]; then
            ARANGO_PORT="${BASH_REMATCH[1]}"
        else
            ARANGO_PORT="${ARANGODB_PORT:-8529}"
        fi
        
        # Run migrations (from host, so use localhost with external port)
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

# Verify deployment
log_info "Verifying deployment..."
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

# Show container status
log_info "Container status:"
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    ps

log_success "Deployment completed!"
log_info ""
log_info "Services:"
log_info "  Backend: http://localhost:${BACKEND_PORT}"
log_info "  Frontend: http://localhost:${FRONTEND_PORT}"
log_info ""
log_info "To view logs:"
log_info "  docker compose --env-file $ENV_FILE -f deploy/docker-compose.production.yml logs -f"

