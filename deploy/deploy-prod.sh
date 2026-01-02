#!/bin/bash

# Production deployment script (industry-standard)
# This script handles production deployments with proper error handling, rollback, and verification

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Configuration
ENV_FILE="${PROJECT_ROOT}/config/.env.production"
COMPOSE_BASE="${SCRIPT_DIR}/docker-compose.yaml"
COMPOSE_PROD="${SCRIPT_DIR}/docker-compose.prod.yml"
BACKUP_DIR="${PROJECT_ROOT}/.deploy-backups"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

# Functions
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

check_prerequisites() {
  log_info "Checking prerequisites..."
  
  # Check Docker
  if ! command -v docker &> /dev/null; then
    log_error "Docker not found. Please install Docker."
    exit 1
  fi
  
  # Check Docker Compose
  if ! docker compose version &> /dev/null; then
    log_error "Docker Compose not found. Please install Docker Compose."
    exit 1
  fi
  
  # Check environment file
  if [ ! -f "$ENV_FILE" ]; then
    log_error "Environment file not found: $ENV_FILE"
    log_info "Create it with: ./config/setup-env.sh production"
    exit 1
  fi
  
  # Check compose files
  if [ ! -f "$COMPOSE_BASE" ] || [ ! -f "$COMPOSE_PROD" ]; then
    log_error "Docker Compose files not found"
    exit 1
  fi
  
  log_success "Prerequisites check passed"
}

backup_current_deployment() {
  log_info "Creating backup of current deployment state..."
  mkdir -p "$BACKUP_DIR"
  
  # Save current git commit
  if [ -d "$PROJECT_ROOT/.git" ]; then
    git rev-parse HEAD > "$BACKUP_DIR/${TIMESTAMP}_git_commit.txt" 2>/dev/null || true
  fi
  
  # Save docker compose ps output
  export ENV_FILE
  docker compose \
    --env-file "$ENV_FILE" \
    -f "$COMPOSE_BASE" \
    -f "$COMPOSE_PROD" \
    ps > "$BACKUP_DIR/${TIMESTAMP}_containers.txt" 2>/dev/null || true
  
  log_success "Backup created: $BACKUP_DIR/${TIMESTAMP}_*"
}

get_build_info() {
  log_info "Getting build information..."
  if [ -f "$PROJECT_ROOT/scripts/build-info.sh" ]; then
    source "$PROJECT_ROOT/scripts/build-info.sh"
    export GIT_COMMIT
    export BUILD_DATE
    log_info "  Git Commit: ${GIT_COMMIT:-unknown}"
    log_info "  Build Date: ${BUILD_DATE:-unknown}"
  else
    export GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
    export BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
  fi
}

compose_cmd() {
  local cmd="$1"
  shift
  docker compose \
    --env-file "$ENV_FILE" \
    -f "$COMPOSE_BASE" \
    -f "$COMPOSE_PROD" \
    "$cmd" "$@"
}

build_images() {
  log_info "Building Docker images..."
  compose_cmd build --progress=plain
  log_success "Images built successfully"
}

deploy_services() {
  log_info "Deploying services..."
  
  # Pull latest images if needed (for base images)
  compose_cmd pull --ignore-pull-failures || true
  
  # Start services with health checks
  compose_cmd up -d
  
  log_success "Services deployed"
}

verify_deployment() {
  log_info "Verifying deployment..."
  
  local max_attempts=30
  local attempt=0
  local all_running=false
  
  while [ $attempt -lt $max_attempts ]; do
    # Check if all containers are running
    local status_output=$(compose_cmd ps 2>/dev/null)
    local exited=$(echo "$status_output" | grep -c "Exit" || echo "0")
    local total=$(echo "$status_output" | grep -c "backend\|frontend\|redis\|arangodb" || echo "0")
    
    # If no exited containers and we have services, consider it good
    if [ "$exited" = "0" ] && [ "$total" -gt "0" ]; then
      all_running=true
      break
    fi
    
    attempt=$((attempt + 1))
    log_info "Waiting for services to start... (attempt $attempt/$max_attempts)"
    sleep 2
  done
  
  if [ "$all_running" = true ]; then
    log_success "All services are running"
    compose_cmd ps
    return 0
  else
    log_warning "Some services may not be running properly"
    compose_cmd ps
    return 1
  fi
}

rollback() {
  log_warning "Rolling back deployment..."
  
  # Find latest backup
  local latest_backup=$(ls -t "$BACKUP_DIR"/*_git_commit.txt 2>/dev/null | head -1)
  
  if [ -n "$latest_backup" ] && [ -d "$PROJECT_ROOT/.git" ]; then
    local previous_commit=$(cat "$latest_backup")
    log_info "Rolling back to commit: $previous_commit"
    git checkout "$previous_commit"
    build_images
    deploy_services
    log_success "Rollback complete"
  else
    log_error "No backup found for rollback"
    log_info "Stopping services..."
    compose_cmd down
    exit 1
  fi
}

main() {
  local skip_backup=false
  local skip_verify=false
  local force_rollback=false
  
  # Parse arguments
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --skip-backup)
        skip_backup=true
        shift
        ;;
      --skip-verify)
        skip_verify=true
        shift
        ;;
      --rollback)
        force_rollback=true
        shift
        ;;
      *)
        log_error "Unknown argument: $1"
        exit 1
        ;;
    esac
  done
  
  if [ "$force_rollback" = true ]; then
    rollback
    exit 0
  fi
  
  log_info "Starting production deployment..."
  
  check_prerequisites
  
  if [ "$skip_backup" != true ]; then
    backup_current_deployment
  fi
  
  get_build_info
  build_images
  deploy_services
  
  if [ "$skip_verify" != true ]; then
    if ! verify_deployment; then
      log_error "Deployment verification failed"
      read -p "Rollback? (y/N): " -n 1 -r
      echo
      if [[ $REPLY =~ ^[Yy]$ ]]; then
        rollback
        exit 1
      fi
    fi
  fi
  
  log_success "Production deployment complete!"
  log_info "View logs: ./deploy/deploy.sh --env production --logs"
  log_info "Check status: ./deploy/deploy.sh --env production --status"
}

# Run main function
main "$@"

