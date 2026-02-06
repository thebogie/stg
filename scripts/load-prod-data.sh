#!/bin/bash

# Script to load production data into local ArangoDB for hybrid development
# This allows you to develop with real production data while running backend/frontend locally

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Load environment variables
# Allow caller to override ENV_FILE (e.g., production workflow)
ENV_FILE="${ENV_FILE:-${PROJECT_ROOT}/config/.env.development}"
if [ ! -f "$ENV_FILE" ]; then
  echo -e "${RED}❌ Error: Environment file not found: $ENV_FILE${NC}"
  echo -e "${YELLOW}💡 Run: ./config/setup-env.sh development${NC}"
  exit 1
fi

# Source environment variables
set -a
source "$ENV_FILE"
set +a

# Load environment variables
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/load-env.sh"

# Default values (use env if set, otherwise defaults)
ARANGO_DB="${ARANGO_DB:-stg_rd}"
ARANGO_USERNAME="${ARANGO_USERNAME:-root}"
ARANGO_PASSWORD="${ARANGO_PASSWORD:-}"
ARANGO_ROOT_PASSWORD="${ARANGO_ROOT_PASSWORD:-}"

# ARANGO_URL and ARANGODB_PORT are set by load-env.sh

# Use root password if username is root
if [ "$ARANGO_USERNAME" = "root" ]; then
  ARANGO_PASSWORD="${ARANGO_ROOT_PASSWORD}"
fi

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

# Restore using the running ArangoDB container (no local tools required)
restore_via_container() {
  local db_dir="$1"
  local container_name=""

  container_name=$(docker ps --format "{{.Names}}" | grep -E "^arangodb(-dev)?$" | head -1 || true)
  if [ -z "$container_name" ]; then
    log_warning "ArangoDB container not found (expected 'arangodb' or 'arangodb-dev')"
    return 1
  fi

  local db_basename
  db_basename=$(basename "$db_dir")

  log_info "Using ArangoDB container: $container_name"
  docker exec "$container_name" sh -c "rm -rf /tmp/prod-restore && mkdir -p /tmp/prod-restore" || return 1
  docker cp "$db_dir" "${container_name}:/tmp/prod-restore/" || return 1

  if docker exec "$container_name" arangorestore \
    --server.endpoint "http://localhost:8529" \
    --server.username "${ARANGO_USERNAME}" \
    --server.password "${ARANGO_PASSWORD}" \
    --server.database "${ARANGO_DB}" \
    --create-database true \
    --input-directory "/tmp/prod-restore/${db_basename}"; then
    docker exec "$container_name" rm -rf /tmp/prod-restore || true
    return 0
  fi

  return 1
}

# Check if ArangoDB is running
log_info "Checking ArangoDB connection..."
if ! curl -s -f -u "${ARANGO_USERNAME}:${ARANGO_PASSWORD}" "${ARANGO_URL}/_api/version" > /dev/null 2>&1; then
  log_error "Cannot connect to ArangoDB at ${ARANGO_URL}"
  log_info "Make sure ArangoDB is running:"
  log_info "  ./deploy/deploy.sh --env development (for full stack)"
  log_info "  Or: docker compose -f deploy/docker-compose.deps.yml up -d (for dependencies only)"
  exit 1
fi

log_success "Connected to ArangoDB"

# Check for data dump files
DUMP_PATHS=(
  "${PROJECT_ROOT}/_build/backups/smacktalk.zip"
  "${PROJECT_ROOT}/_build/backups/smacktalk.tar"
  "${PROJECT_ROOT}/../_backups/smacktalk.zip"
  "${PROJECT_ROOT}/../_backups/smacktalk.tar"
  "${PROJECT_ROOT}/_build/dumps/dump.sanitized.json.gz"
  "${PROJECT_ROOT}/_build/dumps/dump.json"
)

DUMP_FILE=""
for path in "${DUMP_PATHS[@]}"; do
  if [ -f "$path" ]; then
    DUMP_FILE="$path"
    log_info "Found data dump: $DUMP_FILE"
    break
  fi
done

if [ -z "$DUMP_FILE" ]; then
  log_warning "No data dump file found in common locations:"
  for path in "${DUMP_PATHS[@]}"; do
    echo "  - $path"
  done
  echo ""
  log_info "Options:"
  log_info "1. Export from production:"
  log_info "   cargo run --package scripts --bin export_prod_data \\"
  log_info "     -- --arango-url <prod-url> --arango-password <password>"
  log_info ""
  log_info "2. Use arangodump/arangorestore manually:"
  log_info "   arangodump --server.endpoint tcp://<prod-host>:8529 \\"
  log_info "     --server.username root --server.password <password> \\"
  log_info "     --server.database ${ARANGO_DB} --output-directory ./dump"
  log_info ""
  read -p "Continue without loading data? (y/N): " -n 1 -r
  echo
  if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 0
  fi
  log_info "Skipping data load. You can load data manually later."
  exit 0
fi

# Determine dump type and restore
log_info "Loading data from: $DUMP_FILE"

if [[ "$DUMP_FILE" == *.zip ]]; then
  log_info "Detected ZIP archive, extracting..."
  TEMP_DIR=$(mktemp -d)
  unzip -q "$DUMP_FILE" -d "$TEMP_DIR" || {
    log_error "Failed to extract ZIP file"
    rm -rf "$TEMP_DIR"
    exit 1
  }
  
  # Find the database directory in the extracted files
  DB_DIR=$(find "$TEMP_DIR" -type d -name "${ARANGO_DB}" -o -name "smacktalk" | head -1)
  if [ -z "$DB_DIR" ]; then
    log_error "Could not find database directory in dump"
    rm -rf "$TEMP_DIR"
    exit 1
  fi
  
  log_info "Restoring from: $DB_DIR"

  RESTORE_SUCCESS=false

  log_info "Attempting restore inside ArangoDB container..."
  if restore_via_container "$DB_DIR"; then
    log_success "Container restore completed successfully"
    RESTORE_SUCCESS=true
  else
    log_warning "Container restore failed, trying Docker network restore..."
  fi

  if [ "$RESTORE_SUCCESS" = false ]; then
    NETWORK_NAME=""
    for net in "hybrid_dev_env" "deploy_hybrid_dev_env" "deploy_hybridDevEnv" "hybridDevEnv" "deploy_stg_rd_net" "stg_rd_net" "stg_rd_stg_rd_net"; do
      if docker network inspect "$net" > /dev/null 2>&1; then
        NETWORK_NAME="$net"
        log_info "Using Docker network: $NETWORK_NAME"
        break
      fi
    done

    if [ -n "$NETWORK_NAME" ]; then
      if docker run --rm \
        --network "$NETWORK_NAME" \
        -v "${DB_DIR}:/dump" \
        arangodb:3.12.5 \
        arangorestore \
        --server.endpoint "http://arangodb-dev:8529" \
        --server.username "${ARANGO_USERNAME}" \
        --server.password "${ARANGO_PASSWORD}" \
        --server.database "${ARANGO_DB}" \
        --create-database true \
        --input-directory /dump; then
        log_success "Docker restore completed successfully"
        RESTORE_SUCCESS=true
      else
        log_error "Docker restore failed"
      fi
    else
      log_error "Docker network not found for restore"
    fi
  fi
  
  rm -rf "$TEMP_DIR"
  
elif [[ "$DUMP_FILE" == *.tar ]]; then
  log_info "Detected TAR archive, extracting..."
  TEMP_DIR=$(mktemp -d)
  tar -xf "$DUMP_FILE" -C "$TEMP_DIR" || {
    log_error "Failed to extract TAR file"
    rm -rf "$TEMP_DIR"
    exit 1
  }
  
  DB_DIR=$(find "$TEMP_DIR" -type d -name "${ARANGO_DB}" -o -name "smacktalk" | head -1)
  if [ -z "$DB_DIR" ]; then
    log_error "Could not find database directory in dump"
    rm -rf "$TEMP_DIR"
    exit 1
  fi
  
  log_info "Restoring from: $DB_DIR"

  RESTORE_SUCCESS=false

  log_info "Attempting restore inside ArangoDB container..."
  if restore_via_container "$DB_DIR"; then
    log_success "Container restore completed successfully"
    RESTORE_SUCCESS=true
  else
    log_warning "Container restore failed, trying Docker network restore..."
  fi

  if [ "$RESTORE_SUCCESS" = false ]; then
    NETWORK_NAME=""
    for net in "hybrid_dev_env" "deploy_hybrid_dev_env" "deploy_hybridDevEnv" "hybridDevEnv" "deploy_stg_rd_net" "stg_rd_net" "stg_rd_stg_rd_net"; do
      if docker network inspect "$net" > /dev/null 2>&1; then
        NETWORK_NAME="$net"
        log_info "Using Docker network: $NETWORK_NAME"
        break
      fi
    done

    if [ -n "$NETWORK_NAME" ]; then
      if docker run --rm \
        --network "$NETWORK_NAME" \
        -v "${DB_DIR}:/dump" \
        arangodb:3.12.5 \
        arangorestore \
        --server.endpoint "http://arangodb-dev:8529" \
        --server.username "${ARANGO_USERNAME}" \
        --server.password "${ARANGO_PASSWORD}" \
        --server.database "${ARANGO_DB}" \
        --create-database true \
        --input-directory /dump; then
        log_success "Docker restore completed successfully"
        RESTORE_SUCCESS=true
      else
        log_error "Docker restore failed"
      fi
    else
      log_error "Docker network not found for restore"
    fi
  fi
  
  rm -rf "$TEMP_DIR"
  
else
  log_warning "Unknown dump file format. Expected .zip or .tar"
  log_info "You may need to restore manually using arangorestore"
fi

log_success "Data load complete!"
log_info "Database: ${ARANGO_DB}"
log_info "URL: ${ARANGO_URL}"
log_info ""
log_info "You can now start your backend/frontend locally with debuggers."



