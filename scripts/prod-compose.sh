#!/bin/bash

# Helper script for running docker-compose commands on production
# This script ensures paths are resolved correctly
# Usage: ./scripts/prod-compose.sh [docker-compose-command] [args...]
#
# Examples:
#   ./scripts/prod-compose.sh logs -f
#   ./scripts/prod-compose.sh ps
#   ./scripts/prod-compose.sh up -d
#   ./scripts/prod-compose.sh down

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

# Check if we're in the project root
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    log_error "Must be run from project root"
    exit 1
fi

# Set environment file - use absolute path
ENV_FILE="${PROJECT_ROOT}/config/.env.production"

# Check if env file exists
if [ ! -f "$ENV_FILE" ]; then
    log_error "Environment file not found: $ENV_FILE"
    log_info "Please create it from template:"
    log_info "  cp config/env.production.template $ENV_FILE"
    log_info "  nano $ENV_FILE"
    exit 1
fi

# Export ENV_FILE as absolute path for docker-compose
export ENV_FILE

# Ensure the production network exists (needed for docker-compose.stg_prod.yml)
if docker network ls | grep -q "stg_prod"; then
    log_info "Network 'stg_prod' already exists"
else
    log_info "Creating network 'stg_prod'..."
    docker network create stg_prod || log_warning "Failed to create network (may already exist)"
fi

# Change to deploy directory so relative paths in compose files work
cd "$PROJECT_ROOT/deploy"

# Check if stg_prod.yml should be included (for production environment)
# Include docker-compose.stg_prod.yml if it exists and we're in production mode
COMPOSE_FILES="-f docker-compose.yaml -f docker-compose.prod.yml"
if [ -f "docker-compose.stg_prod.yml" ]; then
    COMPOSE_FILES="$COMPOSE_FILES -f docker-compose.stg_prod.yml"
fi

# Run docker compose with all necessary compose files and pass all arguments
# Note: All environment variables from $ENV_FILE are automatically passed to containers
# via the env_file directive in docker-compose.yaml. To pick up env var changes,
# use: ./scripts/prod-compose.sh up -d --force-recreate [service]
docker compose \
    --env-file "$ENV_FILE" \
    $COMPOSE_FILES \
    "$@"

