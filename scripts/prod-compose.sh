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

# Change to deploy directory so relative paths in compose files work
cd "$PROJECT_ROOT/deploy"

# Run docker compose with the compose files and pass all arguments
docker compose \
    --env-file "$ENV_FILE" \
    -f docker-compose.yaml \
    -f docker-compose.prod.yml \
    "$@"

