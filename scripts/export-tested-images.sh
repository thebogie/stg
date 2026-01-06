#!/bin/bash

# Export tested Docker images for deployment
# This script exports the tested images to tar.gz files for transfer to production
# Usage: ./scripts/export-tested-images.sh [--output-dir DIR]

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
OUTPUT_DIR="${PROJECT_ROOT}/_build/artifacts"

while [[ $# -gt 0 ]]; do
    case $1 in
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Usage: $0 [--output-dir DIR]"
            exit 1
            ;;
    esac
done

# Check if we're in the project root
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    log_error "Must be run from project root"
    exit 1
fi

# Check for version file
VERSION_FILE="${PROJECT_ROOT}/_build/.build-version"
if [ ! -f "$VERSION_FILE" ]; then
    log_warning "No build version file found. Attempting to detect existing images..."
    
    # Try to find existing images
    COMPOSE_PROJECT_NAME=$(basename "$PROJECT_ROOT" | tr '[:upper:]' '[:lower:]' | tr -cd '[:alnum:]-')
    
    # Check for images with common tags
    FRONTEND_FOUND=""
    BACKEND_FOUND=""
    
    # Try 'tested' tag first
    if docker image inspect "${COMPOSE_PROJECT_NAME}-frontend:tested" > /dev/null 2>&1; then
        FRONTEND_FOUND="${COMPOSE_PROJECT_NAME}-frontend:tested"
    elif docker image inspect "stg_rd-frontend:tested" > /dev/null 2>&1; then
        FRONTEND_FOUND="stg_rd-frontend:tested"
    elif docker image inspect "${COMPOSE_PROJECT_NAME}-frontend:latest" > /dev/null 2>&1; then
        FRONTEND_FOUND="${COMPOSE_PROJECT_NAME}-frontend:latest"
    elif docker image inspect "stg_rd-frontend:latest" > /dev/null 2>&1; then
        FRONTEND_FOUND="stg_rd-frontend:latest"
    fi
    
    if docker image inspect "${COMPOSE_PROJECT_NAME}-backend:tested" > /dev/null 2>&1; then
        BACKEND_FOUND="${COMPOSE_PROJECT_NAME}-backend:tested"
    elif docker image inspect "stg_rd-backend:tested" > /dev/null 2>&1; then
        BACKEND_FOUND="stg_rd-backend:tested"
    elif docker image inspect "${COMPOSE_PROJECT_NAME}-backend:latest" > /dev/null 2>&1; then
        BACKEND_FOUND="${COMPOSE_PROJECT_NAME}-backend:latest"
    elif docker image inspect "stg_rd-backend:latest" > /dev/null 2>&1; then
        BACKEND_FOUND="stg_rd-backend:latest"
    fi
    
    if [ -n "$FRONTEND_FOUND" ] && [ -n "$BACKEND_FOUND" ]; then
        log_info "Found existing images:"
        log_info "  Frontend: $FRONTEND_FOUND"
        log_info "  Backend: $BACKEND_FOUND"
        log_info "Creating version file from existing images..."
        
        # Get git commit and build date
        if command -v git > /dev/null 2>&1; then
            GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
        else
            GIT_COMMIT="unknown"
        fi
        BUILD_DATE=$(date -u +"%Y-%m-%d %H:%M:%S UTC")
        VERSION_TAG="tested-$(date +%Y%m%d-%H%M%S)"
        
        # Extract image names (without tag) for version file
        FRONTEND_IMAGE_NAME=$(echo "$FRONTEND_FOUND" | cut -d: -f1)
        BACKEND_IMAGE_NAME=$(echo "$BACKEND_FOUND" | cut -d: -f1)
        
        # Create version file
        mkdir -p "${PROJECT_ROOT}/_build"
        cat > "$VERSION_FILE" <<EOF
GIT_COMMIT=$GIT_COMMIT
BUILD_DATE=$BUILD_DATE
VERSION_TAG=$VERSION_TAG
FRONTEND_IMAGE=$FRONTEND_IMAGE_NAME
BACKEND_IMAGE=$BACKEND_IMAGE_NAME
EOF
        log_success "Created version file from existing images"
        
        # Store full image names with tags for later use
        export FRONTEND_FOUND
        export BACKEND_FOUND
    else
        log_error "No build version file found and could not detect existing images."
        log_info ""
        log_info "You have two options:"
        log_info "  1. Build production images (recommended):"
        log_info "     ./scripts/build-prod-images.sh"
        log_info "     (This will be fast if images already exist due to Docker cache)"
        log_info ""
        log_info "  2. If you ran E2E tests, the images may exist but need version tracking."
        log_info "     Run build-prod-images.sh to create the version file."
        log_info ""
        log_info "Available images:"
        docker images | grep -E "frontend|backend|stg_rd" | head -10 || log_info "  (none found)"
        exit 1
    fi
fi

source "$VERSION_FILE"

log_info "Exporting tested images"
log_info "  Version: $VERSION_TAG"
log_info "  Git Commit: $GIT_COMMIT"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Export images
log_info "Exporting images to: $OUTPUT_DIR"

FRONTEND_EXPORT="${OUTPUT_DIR}/frontend-${VERSION_TAG}.tar.gz"
BACKEND_EXPORT="${OUTPUT_DIR}/backend-${VERSION_TAG}.tar.gz"

# Determine which images to export
# If we auto-detected images above (FRONTEND_FOUND/BACKEND_FOUND set), use those
# Otherwise, try to find images using the version file info
if [ -n "${FRONTEND_FOUND:-}" ]; then
    # Use the image we found during auto-detection
    FRONTEND_TO_EXPORT="$FRONTEND_FOUND"
else
    # Try to find images with 'tested' tag first, then version tag, then latest
    FRONTEND_TO_EXPORT="${FRONTEND_IMAGE}:tested"
    if ! docker image inspect "$FRONTEND_TO_EXPORT" > /dev/null 2>&1; then
        FRONTEND_TO_EXPORT="${FRONTEND_IMAGE}:${VERSION_TAG}"
        if ! docker image inspect "$FRONTEND_TO_EXPORT" > /dev/null 2>&1; then
            FRONTEND_TO_EXPORT="${FRONTEND_IMAGE}:latest"
        fi
    fi
fi

if [ -n "${BACKEND_FOUND:-}" ]; then
    BACKEND_TO_EXPORT="$BACKEND_FOUND"
else
    BACKEND_TO_EXPORT="${BACKEND_IMAGE}:tested"
    if ! docker image inspect "$BACKEND_TO_EXPORT" > /dev/null 2>&1; then
        BACKEND_TO_EXPORT="${BACKEND_IMAGE}:${VERSION_TAG}"
        if ! docker image inspect "$BACKEND_TO_EXPORT" > /dev/null 2>&1; then
            BACKEND_TO_EXPORT="${BACKEND_IMAGE}:latest"
        fi
    fi
fi

# Verify images exist
if ! docker image inspect "$FRONTEND_TO_EXPORT" > /dev/null 2>&1; then
    log_error "Frontend image not found: $FRONTEND_TO_EXPORT"
    log_info "Available images:"
    docker images | grep -E "frontend|stg_rd" || true
    exit 1
fi

if ! docker image inspect "$BACKEND_TO_EXPORT" > /dev/null 2>&1; then
    log_error "Backend image not found: $BACKEND_TO_EXPORT"
    log_info "Available images:"
    docker images | grep -E "backend|stg_rd" || true
    exit 1
fi

log_info "Exporting frontend image: $FRONTEND_TO_EXPORT"
docker save "$FRONTEND_TO_EXPORT" | gzip > "$FRONTEND_EXPORT"
log_success "Frontend exported: $FRONTEND_EXPORT ($(du -h "$FRONTEND_EXPORT" | cut -f1))"

log_info "Exporting backend image: $BACKEND_TO_EXPORT"
docker save "$BACKEND_TO_EXPORT" | gzip > "$BACKEND_EXPORT"
log_success "Backend exported: $BACKEND_EXPORT ($(du -h "$BACKEND_EXPORT" | cut -f1))"

# Create deployment info file
DEPLOY_INFO="${OUTPUT_DIR}/deploy-info-${VERSION_TAG}.txt"
cat > "$DEPLOY_INFO" <<EOF
Deployment Information
=====================
Version Tag: $VERSION_TAG
Git Commit: $GIT_COMMIT
Build Date: $BUILD_DATE
Export Date: $(date -u +"%Y-%m-%d %H:%M:%S UTC")

Images:
  Frontend: $FRONTEND_TO_EXPORT
  Backend: $BACKEND_TO_EXPORT

Files:
  Frontend: frontend-${VERSION_TAG}.tar.gz
  Backend: backend-${VERSION_TAG}.tar.gz

Deployment Instructions:
  1. Transfer files to production server:
     scp ${OUTPUT_DIR}/*.tar.gz user@production-server:/tmp/
  
  2. On production server, load images:
     gunzip -c /tmp/frontend-${VERSION_TAG}.tar.gz | docker load
     gunzip -c /tmp/backend-${VERSION_TAG}.tar.gz | docker load
  
  3. Deploy using:
     ./scripts/deploy-tested-images.sh --version ${VERSION_TAG}
EOF

log_success "Deployment info saved: $DEPLOY_INFO"

# Create checksums
log_info "Creating checksums..."
cd "$OUTPUT_DIR"
sha256sum frontend-${VERSION_TAG}.tar.gz > frontend-${VERSION_TAG}.tar.gz.sha256
sha256sum backend-${VERSION_TAG}.tar.gz > backend-${VERSION_TAG}.tar.gz.sha256

log_success "Checksums created"

log_success "Images exported successfully!"
log_info ""
log_info "Files in $OUTPUT_DIR:"
ls -lh "$OUTPUT_DIR"/*${VERSION_TAG}* | awk '{print "  " $9 " (" $5 ")"}'
log_info ""
log_info "To transfer to production:"
log_info "  scp ${OUTPUT_DIR}/*.tar.gz* user@production-server:/tmp/"
log_info ""
log_info "Or use a Docker registry (recommended):"
log_info "  docker tag $FRONTEND_TO_EXPORT your-registry/stg_rd-frontend:${VERSION_TAG}"
log_info "  docker tag $BACKEND_TO_EXPORT your-registry/stg_rd-backend:${VERSION_TAG}"
log_info "  docker push your-registry/stg_rd-frontend:${VERSION_TAG}"
log_info "  docker push your-registry/stg_rd-backend:${VERSION_TAG}"

