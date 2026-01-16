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

# CRITICAL: Expand variable substitutions in .env.production before passing to docker-compose
# Docker Compose's env_file directive does NOT expand ${VAR} syntax - it passes them literally to containers
# The environment: section in docker-compose.yaml overrides ARANGO_URL correctly, but we need
# all other variables (ARANGO_DB, ARANGO_USERNAME, etc.) to be expanded for docker-compose parsing
TEMP_ENV_FILE="${PROJECT_ROOT}/.env.production.expanded.$$"
log_info "Expanding variable substitutions in environment file..."

# First pass: Load variables from env file to expand references
# This allows us to expand ${ARANGODB_PORT} etc. before writing the expanded file
set -a
source "$ENV_FILE" 2>/dev/null || {
    log_error "Failed to load environment file: $ENV_FILE"
    exit 1
}
set +a

# Load IMAGE_TAG from deployment version file if it exists
# This ensures systemctl restarts use the correct version
VERSION_FILE="${PROJECT_ROOT}/_build/.deployed-version"
if [ -f "$VERSION_FILE" ]; then
    # Source the version file to get all variables
    set -a
    source "$VERSION_FILE" 2>/dev/null || true
    set +a
    
    if [ -n "${VERSION_TAG:-}" ]; then
        export IMAGE_TAG="${IMAGE_TAG:-$VERSION_TAG}"
        export FRONTEND_IMAGE_TAG="${FRONTEND_IMAGE_TAG:-$VERSION_TAG}"
        export FRONTEND_IMAGE="${FRONTEND_IMAGE:-stg_rd-frontend:${VERSION_TAG}}"
        export BACKEND_IMAGE="${BACKEND_IMAGE:-stg_rd-backend:${VERSION_TAG}}"
        log_info "Using deployed version: $VERSION_TAG"
    fi
fi

# Second pass: Write expanded values (replacing ${VAR} with actual values)
rm -f "$TEMP_ENV_FILE"
while IFS= read -r line || [ -n "$line" ]; do
    # Skip empty lines and comments
    if [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]]; then
        echo "$line" >> "$TEMP_ENV_FILE"
        continue
    fi
    
    # If line contains variable assignment, expand it
    if [[ "$line" =~ ^[[:space:]]*([^#=]+)=(.*)$ ]]; then
        var_name="${BASH_REMATCH[1]// /}"
        var_value="${BASH_REMATCH[2]}"
        
        # Expand variables in the value (handle ${VAR} syntax)
        # Use eval carefully - only expand known variable patterns
        if [[ "$var_value" == *\$\{* ]]; then
            # Expand using bash parameter expansion
            expanded_value=$(eval "echo \"$var_value\"")
            echo "${var_name}=${expanded_value}" >> "$TEMP_ENV_FILE"
        else
            # No expansion needed
            echo "$line" >> "$TEMP_ENV_FILE"
        fi
    else
        # Keep non-assignment lines as-is
        echo "$line" >> "$TEMP_ENV_FILE"
    fi
done < "$ENV_FILE"

# Append IMAGE_TAG and image names to temp env file if they were loaded from version file
# This ensures docker-compose uses the correct version
if [ -n "${IMAGE_TAG:-}" ]; then
    # Remove any existing IMAGE_TAG lines
    sed -i '/^IMAGE_TAG=/d' "$TEMP_ENV_FILE" 2>/dev/null || sed -i.bak '/^IMAGE_TAG=/d' "$TEMP_ENV_FILE"
    sed -i '/^FRONTEND_IMAGE_TAG=/d' "$TEMP_ENV_FILE" 2>/dev/null || sed -i.bak '/^FRONTEND_IMAGE_TAG=/d' "$TEMP_ENV_FILE"
    sed -i '/^FRONTEND_IMAGE=/d' "$TEMP_ENV_FILE" 2>/dev/null || sed -i.bak '/^FRONTEND_IMAGE=/d' "$TEMP_ENV_FILE"
    sed -i '/^BACKEND_IMAGE=/d' "$TEMP_ENV_FILE" 2>/dev/null || sed -i.bak '/^BACKEND_IMAGE=/d' "$TEMP_ENV_FILE"
    rm -f "${TEMP_ENV_FILE}.bak" 2>/dev/null || true
    # Append the version info
    echo "IMAGE_TAG=$IMAGE_TAG" >> "$TEMP_ENV_FILE"
    echo "FRONTEND_IMAGE_TAG=$FRONTEND_IMAGE_TAG" >> "$TEMP_ENV_FILE"
    if [ -n "${FRONTEND_IMAGE:-}" ]; then
        echo "FRONTEND_IMAGE=$FRONTEND_IMAGE" >> "$TEMP_ENV_FILE"
    fi
    if [ -n "${BACKEND_IMAGE:-}" ]; then
        echo "BACKEND_IMAGE=$BACKEND_IMAGE" >> "$TEMP_ENV_FILE"
    fi
fi

# Use the expanded file for docker-compose (both --env-file and env_file: will use expanded values)
export ENV_FILE="$TEMP_ENV_FILE"

# Cleanup function to remove temp file on exit
cleanup_temp_env() {
    if [ -f "$TEMP_ENV_FILE" ]; then
        rm -f "$TEMP_ENV_FILE"
    fi
}
trap cleanup_temp_env EXIT INT TERM

# Ensure the production network exists (needed for docker-compose.production.yml)
# The compose file will create it if external: false, but we ensure it exists for safety
if ! docker network inspect stg_prod >/dev/null 2>&1; then
    log_info "Creating production network 'stg_prod'..."
    docker network create stg_prod || log_warning "Network may already exist or creation failed"
else
    log_info "Production network 'stg_prod' already exists"
fi

# Change to deploy directory so relative paths in compose files work
cd "$PROJECT_ROOT/deploy"

# Use single consolidated production docker-compose file
COMPOSE_FILE="docker-compose.production.yml"

if [ ! -f "$COMPOSE_FILE" ]; then
    log_error "Production compose file not found: $COMPOSE_FILE"
    exit 1
fi

# Run docker compose with the single production compose file
# All environment variables come from config/.env.production via --env-file
docker compose \
    -f "$COMPOSE_FILE" \
    --env-file "$ENV_FILE" \
    "$@"

