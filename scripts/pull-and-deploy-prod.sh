#!/bin/bash

# Pull tested Docker images from Docker Hub and deploy to production
# This script automates the full production deployment workflow:
# 1. Pulls images from Docker Hub
# 2. Tags them for deployment
# 3. Deploys using deploy-tested-images.sh
# 4. Optionally restarts a systemd service
#
# Usage: ./scripts/pull-and-deploy-prod.sh [--version TAG] [--docker-hub-user USERNAME] [--skip-backup] [--skip-migrations] [--restart-service SERVICE_NAME]
#
# Example:
#   ./scripts/pull-and-deploy-prod.sh --version va8b5487-20260106-073414 --docker-hub-user therealbogie --restart-service stg_rd

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
DOCKER_HUB_USER=""
SKIP_BACKUP=false
SKIP_MIGRATIONS=false
RESTART_SERVICE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            VERSION_TAG="$2"
            shift 2
            ;;
        --docker-hub-user)
            DOCKER_HUB_USER="$2"
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
        --restart-service)
            RESTART_SERVICE="$2"
            shift 2
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Usage: $0 [--version TAG] [--docker-hub-user USERNAME] [--skip-backup] [--skip-migrations] [--restart-service SERVICE_NAME]"
            exit 1
            ;;
    esac
done

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

# Get Docker Hub username
if [ -z "$DOCKER_HUB_USER" ]; then
    # Try to detect from existing images
    EXISTING_IMAGE=$(docker images --format "{{.Repository}}" | grep -E "^[^/]+/stg_rd" | head -1 | cut -d'/' -f1 || true)
    if [ -n "$EXISTING_IMAGE" ]; then
        DOCKER_HUB_USER="$EXISTING_IMAGE"
        log_info "Detected Docker Hub username from existing images: $DOCKER_HUB_USER"
    else
        # Try environment variable
        if [ -n "${DOCKER_HUB_USER:-}" ]; then
            DOCKER_HUB_USER="$DOCKER_HUB_USER"
        else
            log_error "Docker Hub username not provided and could not be detected"
            log_info "Usage: $0 --version TAG --docker-hub-user USERNAME"
            log_info "Or set: export DOCKER_HUB_USER=your-username"
            exit 1
        fi
    fi
fi

log_info "Using Docker Hub username: $DOCKER_HUB_USER"

# Check if version is provided
if [ -z "$VERSION_TAG" ]; then
    log_error "Version tag is required"
    log_info "Usage: $0 --version TAG --docker-hub-user USERNAME"
    log_info "Example: $0 --version va8b5487-20260106-073414 --docker-hub-user therealbogie"
    exit 1
fi

# Check Docker login
log_info "Checking Docker Hub access..."
if ! docker info >/dev/null 2>&1; then
    log_error "Docker is not running or you're not logged in"
    log_info "Please run: docker login"
    exit 1
fi

# Try to verify Docker Hub access
if ! docker pull "${DOCKER_HUB_USER}/stg_rd:frontend-${VERSION_TAG}" >/dev/null 2>&1; then
    log_warning "Could not pull test image. Make sure you're logged in:"
    log_info "  docker login"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# ============================================================================
# STEP 1: Pull Images from Docker Hub
# ============================================================================
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "STEP 1: Pulling Images from Docker Hub"
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

FRONTEND_HUB="${DOCKER_HUB_USER}/stg_rd:frontend-${VERSION_TAG}"
BACKEND_HUB="${DOCKER_HUB_USER}/stg_rd:backend-${VERSION_TAG}"

log_info "Pulling frontend image: $FRONTEND_HUB"
if ! docker pull "$FRONTEND_HUB"; then
    log_error "Failed to pull frontend image!"
    exit 1
fi
log_success "Frontend image pulled"

log_info "Pulling backend image: $BACKEND_HUB"
if ! docker pull "$BACKEND_HUB"; then
    log_error "Failed to pull backend image!"
    exit 1
fi
log_success "Backend image pulled"

# ============================================================================
# STEP 2: Tag Images for Deployment
# ============================================================================
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "STEP 2: Tagging Images for Deployment"
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "Tagging frontend: $FRONTEND_HUB -> stg_rd-frontend:latest"
docker tag "$FRONTEND_HUB" "stg_rd-frontend:latest"
log_success "Frontend tagged"

log_info "Tagging backend: $BACKEND_HUB -> stg_rd-backend:latest"
docker tag "$BACKEND_HUB" "stg_rd-backend:latest"
log_success "Backend tagged"

# ============================================================================
# STEP 3: Deploy Using deploy-tested-images.sh
# ============================================================================
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "STEP 3: Deploying to Production"
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Build deploy command
DEPLOY_CMD="./scripts/deploy-tested-images.sh --version $VERSION_TAG --skip-load"

if [ "$SKIP_BACKUP" = true ]; then
    DEPLOY_CMD="$DEPLOY_CMD --skip-backup"
fi

if [ "$SKIP_MIGRATIONS" = true ]; then
    DEPLOY_CMD="$DEPLOY_CMD --skip-migrations"
fi

log_info "Running: $DEPLOY_CMD"
eval "$DEPLOY_CMD"

log_success "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_success "Production deployment complete! 🚀"
log_success "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Reminder about service restart
if [ -n "$RESTART_SERVICE" ]; then
    log_info ""
    log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    log_info "⚠️  NEXT STEP: Restart your systemd service"
    log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    log_info ""
    log_info "Run this command to restart your service:"
    log_info "  sudo systemctl restart $RESTART_SERVICE"
    log_info ""
fi
