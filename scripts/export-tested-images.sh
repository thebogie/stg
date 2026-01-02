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
    log_error "No build version file found. Build images first:"
    log_info "  ./scripts/build-prod-images.sh"
    exit 1
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

# Try to find images with 'tested' tag first, then version tag
FRONTEND_IMAGE="${FRONTEND_IMAGE}:tested"
BACKEND_IMAGE="${BACKEND_IMAGE}:tested"

if ! docker image inspect "$FRONTEND_IMAGE" > /dev/null 2>&1; then
    FRONTEND_IMAGE="${FRONTEND_IMAGE}:${VERSION_TAG}"
fi

if ! docker image inspect "$BACKEND_IMAGE" > /dev/null 2>&1; then
    BACKEND_IMAGE="${BACKEND_IMAGE}:${VERSION_TAG}"
fi

# Verify images exist
if ! docker image inspect "$FRONTEND_IMAGE" > /dev/null 2>&1; then
    log_error "Frontend image not found: $FRONTEND_IMAGE"
    log_info "Available images:"
    docker images | grep -E "frontend|stg_rd" || true
    exit 1
fi

if ! docker image inspect "$BACKEND_IMAGE" > /dev/null 2>&1; then
    log_error "Backend image not found: $BACKEND_IMAGE"
    log_info "Available images:"
    docker images | grep -E "backend|stg_rd" || true
    exit 1
fi

log_info "Exporting frontend image: $FRONTEND_IMAGE"
docker save "$FRONTEND_IMAGE" | gzip > "$FRONTEND_EXPORT"
log_success "Frontend exported: $FRONTEND_EXPORT ($(du -h "$FRONTEND_EXPORT" | cut -f1))"

log_info "Exporting backend image: $BACKEND_IMAGE"
docker save "$BACKEND_IMAGE" | gzip > "$BACKEND_EXPORT"
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
  Frontend: $FRONTEND_IMAGE
  Backend: $BACKEND_IMAGE

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
log_info "  docker tag $FRONTEND_IMAGE your-registry/stg_rd-frontend:${VERSION_TAG}"
log_info "  docker tag $BACKEND_IMAGE your-registry/stg_rd-backend:${VERSION_TAG}"
log_info "  docker push your-registry/stg_rd-frontend:${VERSION_TAG}"
log_info "  docker push your-registry/stg_rd-backend:${VERSION_TAG}"

