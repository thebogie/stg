#!/bin/bash

# Master workflow script: Test-Then-Deploy
# This script orchestrates the complete workflow from build to deployment
# Usage: ./scripts/workflow-test-then-deploy.sh [--skip-tests] [--skip-export] [--deploy]

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
SKIP_TESTS=false
SKIP_EXPORT=false
DEPLOY=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-tests)
            SKIP_TESTS=true
            shift
            ;;
        --skip-export)
            SKIP_EXPORT=true
            shift
            ;;
        --deploy)
            DEPLOY=true
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Usage: $0 [--skip-tests] [--skip-export] [--deploy]"
            exit 1
            ;;
    esac
done

# Check if we're in the project root
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    log_error "Must be run from project root"
    exit 1
fi

cd "$PROJECT_ROOT"

log_section "🚀 Test-Then-Deploy Workflow"

# Phase 1: Build
log_section "Phase 1: Building Production Images"
log_info "Building production Docker images..."
"$PROJECT_ROOT/scripts/build-prod-images.sh"

if [ ! -f "$PROJECT_ROOT/_build/.build-version" ]; then
    log_error "Build failed - no version file created"
    exit 1
fi

source "$PROJECT_ROOT/_build/.build-version"
log_success "Images built: $VERSION_TAG"

# Phase 2: Test
if [ "$SKIP_TESTS" = false ]; then
    log_section "Phase 2: Testing Production Containers"
    log_info "Starting production containers for testing..."
    log_warning "Make sure you have production data loaded (./scripts/load-prod-data.sh)"
    echo ""
    read -p "Press Enter to continue with testing, or Ctrl+C to skip..."
    
    "$PROJECT_ROOT/scripts/test-prod-containers.sh" || {
        log_error "Testing failed!"
        log_info "Fix issues and run again, or use --skip-tests to continue"
        exit 1
    }
    
    log_success "Containers are running and ready for testing"
    log_info ""
    log_info "Test your application at:"
    log_info "  Backend: http://localhost:${BACKEND_PORT:-50012}"
    log_info "  Frontend: http://localhost:${FRONTEND_PORT:-50013}"
    log_info ""
    read -p "Press Enter after you've completed testing, or Ctrl+C to abort..."
    
    # Stop test containers
    log_info "Stopping test containers..."
    ENV_FILE="${PROJECT_ROOT}/config/.env.production"
    if [ -f "$ENV_FILE" ]; then
        export ENV_FILE
        docker compose \
            --env-file "$ENV_FILE" \
            -f deploy/docker-compose.yaml \
            -f deploy/docker-compose.prod.yml \
            down 2>/dev/null || true
    fi
else
    log_warning "Skipping tests (--skip-tests)"
fi

# Phase 3: Export
if [ "$SKIP_EXPORT" = false ]; then
    log_section "Phase 3: Exporting Tested Images"
    log_info "Exporting tested images..."
    "$PROJECT_ROOT/scripts/export-tested-images.sh"
    
    log_success "Images exported to _build/artifacts/ directory"
    log_info ""
    log_info "Next steps:"
    log_info "  1. Transfer images to production:"
    log_info "     scp _build/artifacts/*.tar.gz* user@production-server:/tmp/"
    log_info ""
    log_info "  2. On production server:"
    log_info "     ./scripts/backup-prod-db.sh"
    log_info "     ./scripts/deploy-tested-images.sh --version $VERSION_TAG --image-dir /tmp"
else
    log_warning "Skipping export (--skip-export)"
fi

# Phase 4: Deploy (optional)
if [ "$DEPLOY" = true ]; then
    log_section "Phase 4: Deploying to Production"
    log_warning "This will deploy to production!"
    log_info "Make sure you've:"
    log_info "  1. Transferred images to production server"
    log_info "  2. Created database backup"
    log_info "  3. Verified production environment"
    echo ""
    read -p "Type 'deploy' to confirm: " confirm
    if [ "$confirm" != "deploy" ]; then
        log_info "Deployment cancelled"
        exit 0
    fi
    
    log_info "Deploying to production..."
    "$PROJECT_ROOT/scripts/deploy-tested-images.sh" --version "$VERSION_TAG" || {
        log_error "Deployment failed!"
        exit 1
    }
    
    log_success "Deployment completed!"
else
    log_info ""
    log_info "To deploy, run on production server:"
    log_info "  ./scripts/deploy-tested-images.sh --version $VERSION_TAG --image-dir /tmp"
fi

log_section "✅ Workflow Complete"
log_success "Version: $VERSION_TAG"
log_success "Git Commit: $GIT_COMMIT"
log_success "Build Date: $BUILD_DATE"

