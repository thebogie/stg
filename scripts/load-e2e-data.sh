#!/bin/bash

# Load production backup into E2E test environment
# This restores smacktalk.zip into the E2E ArangoDB container
# Usage: ./scripts/load-e2e-data.sh [backup-file]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEPLOY_DIR="${PROJECT_ROOT}/deploy"
PROJECT_NAME="e2e_env"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Load environment variables
if [ -f "${SCRIPT_DIR}/load-env.sh" ]; then
  source "${SCRIPT_DIR}/load-env.sh" development
fi

ARANGO_DB="${ARANGO_DB:-smacktalk}"
ARANGO_USERNAME="${ARANGO_USERNAME:-root}"
ARANGO_PASSWORD="${ARANGO_PASSWORD:-letmein}"

# Get backup file
BACKUP_FILE="$1"
if [ -z "$BACKUP_FILE" ]; then
  # Try to find latest production backup
  BACKUP_DIRS=(
    "${PROJECT_ROOT}/_build/backups"
    "${PROJECT_ROOT}/../_backups"
  )
  
  for dir in "${BACKUP_DIRS[@]}"; do
    if [ -d "$dir" ]; then
      # Look for smacktalk.zip or latest backup
      if [ -f "${dir}/smacktalk.zip" ]; then
        BACKUP_FILE="${dir}/smacktalk.zip"
        break
      fi
      # Try to find latest backup file
      LATEST=$(find "$dir" -name "*.zip" -o -name "*.tar.gz" | sort -r | head -1)
      if [ -n "$LATEST" ]; then
        BACKUP_FILE="$LATEST"
        break
      fi
    fi
  done
fi

if [ -z "$BACKUP_FILE" ] || [ ! -f "$BACKUP_FILE" ]; then
  echo -e "${YELLOW}⚠️  Backup file not found: ${BACKUP_FILE:-not specified}${NC}"
  echo -e "${BLUE}💡 Usage: $0 [backup-file]${NC}"
  echo -e "${BLUE}💡 Or place smacktalk.zip in _build/backups/${NC}"
  echo -e "${YELLOW}   Continuing without loading data...${NC}"
  exit 0
fi

echo -e "${BLUE}📦 Loading production backup into E2E environment...${NC}"
echo -e "${BLUE}   Backup file: $BACKUP_FILE${NC}"

# Find ArangoDB container
CONTAINER_ID=$(docker ps -q -f "name=arangodb" --filter "label=com.docker.compose.project=${PROJECT_NAME}" | head -1)

if [ -z "$CONTAINER_ID" ]; then
  echo -e "${RED}❌ ArangoDB container not found for project: ${PROJECT_NAME}${NC}"
  echo -e "${YELLOW}💡 Make sure E2E containers are running: ./scripts/start-e2e-docker.sh${NC}"
  exit 1
fi

echo -e "${BLUE}📋 Found ArangoDB container: ${CONTAINER_ID:0:12}...${NC}"

# Wait for ArangoDB to be ready
echo -e "${BLUE}⏳ Waiting for ArangoDB to be ready...${NC}"
timeout=30
elapsed=0
while ! docker exec "$CONTAINER_ID" arangosh --server.endpoint tcp://127.0.0.1:8529 \
  --server.username "$ARANGO_USERNAME" --server.password "$ARANGO_PASSWORD" \
  --server.database "_system" --javascript.execute "db._version()" > /dev/null 2>&1; do
  if [ $elapsed -ge $timeout ]; then
    echo -e "${RED}❌ Timeout waiting for ArangoDB${NC}"
    exit 1
  fi
  sleep 1
  elapsed=$((elapsed + 1))
done

echo -e "${GREEN}✅ ArangoDB is ready${NC}"

# Extract backup if needed
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

if [[ "$BACKUP_FILE" == *.zip ]]; then
  echo -e "${BLUE}📂 Extracting ZIP archive...${NC}"
  unzip -q "$BACKUP_FILE" -d "$TEMP_DIR" || {
    echo -e "${RED}❌ Failed to extract ZIP file${NC}"
    exit 1
  }
elif [[ "$BACKUP_FILE" == *.tar.gz ]] || [[ "$BACKUP_FILE" == *.tar ]]; then
  echo -e "${BLUE}📂 Extracting TAR archive...${NC}"
  tar -xf "$BACKUP_FILE" -C "$TEMP_DIR" || {
    echo -e "${RED}❌ Failed to extract TAR file${NC}"
    exit 1
  }
else
  echo -e "${RED}❌ Unknown backup format. Expected .zip, .tar.gz, or .tar${NC}"
  exit 1
fi

# Find database directory
DB_DIR=$(find "$TEMP_DIR" -type d -name "${ARANGO_DB}" -o -name "smacktalk" | head -1)
if [ -z "$DB_DIR" ]; then
  # Check if dump directory contains database files directly
  if [ -f "$TEMP_DIR/_collections" ] || [ -d "$TEMP_DIR/collections" ]; then
    DB_DIR="$TEMP_DIR"
  else
    echo -e "${RED}❌ Could not find database directory in backup${NC}"
    echo -e "${YELLOW}   Searched for: ${ARANGO_DB}, smacktalk${NC}"
    exit 1
  fi
fi

echo -e "${BLUE}📂 Found database directory: $DB_DIR${NC}"

# Copy dump into container
echo -e "${BLUE}📤 Copying backup into container...${NC}"
docker cp "$DB_DIR" "${CONTAINER_ID}:/tmp/restore-dump" || {
  echo -e "${RED}❌ Failed to copy backup into container${NC}"
  exit 1
}

# Restore backup
echo -e "${BLUE}🔄 Restoring backup into database '${ARANGO_DB}'...${NC}"
docker exec "$CONTAINER_ID" arangorestore \
  --server.endpoint "tcp://127.0.0.1:8529" \
  --server.username "$ARANGO_USERNAME" \
  --server.password "$ARANGO_PASSWORD" \
  --server.database "$ARANGO_DB" \
  --create-database true \
  --input-directory "/tmp/restore-dump" \
  --overwrite true || {
  echo -e "${YELLOW}⚠️  Restore had warnings (this may be normal)${NC}"
}

# Cleanup
docker exec "$CONTAINER_ID" rm -rf "/tmp/restore-dump" 2>/dev/null || true

echo -e "${GREEN}✅ Production backup loaded successfully!${NC}"
echo -e "${BLUE}💡 Database '${ARANGO_DB}' is ready for E2E tests${NC}"
