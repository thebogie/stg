#!/bin/bash

# Start Docker containers for E2E tests
# This starts frontend, backend, ArangoDB, and Redis in a production-like environment
# Uses separate Docker network (e2e_env) to isolate from other environments
# Matches production configuration exactly for accurate test coverage

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEPLOY_DIR="${PROJECT_ROOT}/deploy"
# Explicitly set ENV_FILE to absolute path - must include config/
ENV_FILE="${PROJECT_ROOT}/config/.env.development"
PROJECT_NAME="e2e_env"
NETWORK_NAME="e2e_env"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}🚀 Starting E2E test environment (production-like, isolated network)...${NC}"

# Check if environment file exists first
if [ ! -f "$ENV_FILE" ]; then
  echo -e "${RED}❌ Error: Environment file not found: $ENV_FILE${NC}"
  echo -e "${YELLOW}💡 Run: ./config/setup-env.sh development${NC}"
  exit 1
fi

# Load ALL environment variables from config/.env.development
# This ensures all variables (database URLs, API keys, ports, credentials, etc.) are available
# NOTE: We preserve ENV_FILE before sourcing load-env.sh in case it modifies it
if [ -f "${SCRIPT_DIR}/load-env.sh" ]; then
  source "${SCRIPT_DIR}/load-env.sh" development
  # Restore ENV_FILE to the correct path (load-env.sh might have changed it)
  ENV_FILE="${PROJECT_ROOT}/config/.env.development"
else
  echo -e "${YELLOW}⚠️  Warning: load-env.sh not found, using environment file directly${NC}"
fi

# Check if docker-compose files exist
if [ ! -f "${DEPLOY_DIR}/docker-compose.yaml" ]; then
  echo -e "${RED}❌ Error: docker-compose.yaml not found${NC}"
  exit 1
fi

if [ ! -f "${DEPLOY_DIR}/docker-compose.e2e.yml" ]; then
  echo -e "${RED}❌ Error: docker-compose.e2e.yml not found${NC}"
  exit 1
fi

# Ensure ENV_FILE is an absolute path BEFORE cd'ing (important!)
# This prevents path resolution issues when docker-compose runs from deploy/
ENV_FILE="${PROJECT_ROOT}/config/.env.development"

# Verify the env file exists (check before cd'ing)
if [ ! -f "$ENV_FILE" ]; then
  echo -e "${RED}❌ Error: Environment file not found: $ENV_FILE${NC}"
  echo -e "${YELLOW}💡 Run: ./config/setup-env.sh development${NC}"
  exit 1
fi

cd "$DEPLOY_DIR"

# Create isolated network for E2E tests (like hybrid_dev_env)
echo -e "${BLUE}📡 Creating isolated network: ${NETWORK_NAME}...${NC}"
if ! docker network inspect "$NETWORK_NAME" > /dev/null 2>&1; then
  docker network create "$NETWORK_NAME" > /dev/null 2>&1
  echo -e "${GREEN}✅ Network created${NC}"
else
  echo -e "${GREEN}✅ Network already exists${NC}"
fi

# Export ENV_FILE for docker-compose (must be absolute path)
export ENV_FILE

# E2E environment uses standardized ports (defined in docker-compose.e2e.yml):
# - Frontend: 50023 (host) -> 8080 (container, nginx listens on FRONTEND_PORT env)
# - Backend: 50022 (host) -> 50022 (container)
# - ArangoDB: 50021 (host) -> 8529 (container)
# - Redis: 63790 (host) -> 6379 (container)
# All ports are hardcoded in docker-compose.e2e.yml to avoid conflicts

# Note: All other variables (ARANGO_URL, REDIS_URL, API keys, database credentials, etc.)
# come from config/.env.development via load-env.sh above

# Start containers in production-like environment
# Uses: docker-compose.yaml (base) + docker-compose.prod.yml (production config) + 
#       docker-compose.stg_prod.yml (production network) + docker-compose.e2e.yml (port overrides only)
# 
# IMPORTANT: This ensures E2E uses EXACT same configuration as production!
# Only differences: port mappings (for Playwright) and network name (for isolation)
#
# Note: --env-file loads ALL variables from config/.env.development
# The docker-compose files will use these variables for:
# - Database URLs (ARANGO_URL, REDIS_URL)
# - API keys (BGG_API_TOKEN, GOOGLE_LOCATION_API, etc.)
# - Ports (ARANGODB_PORT, REDIS_PORT, BACKEND_PORT, etc.)
# - Database credentials (ARANGO_USERNAME, ARANGO_PASSWORD, etc.)
# - All other configuration from config/.env.development
echo -e "${BLUE}📦 Starting containers in production-like environment...${NC}"
echo -e "${BLUE}   Project: ${PROJECT_NAME} (isolated network: ${NETWORK_NAME})${NC}"
echo -e "${BLUE}   Variables loaded: ARANGO_URL, REDIS_URL, API keys, ports, credentials, etc.${NC}"
echo -e "${BLUE}   Configuration: Matches production EXACTLY (same compose files, only ports differ)${NC}"

# Industry standard: Pre-build images, don't build during test runs
# This is faster, more reliable, and matches CI/CD practices
# Images should be built separately with: ./scripts/build-e2e-images.sh
# Or set BUILD_IMAGES=1 to build now (slower, but works)
if [ "${BUILD_IMAGES:-0}" = "1" ]; then
  echo -e "${BLUE}🔨 Building images (this may take 5-10 minutes)...${NC}"
  docker compose \
    -p "$PROJECT_NAME" \
    --env-file "$ENV_FILE" \
    -f docker-compose.yaml \
    -f docker-compose.prod.yml \
    -f docker-compose.stg_prod.yml \
    -f docker-compose.e2e.yml \
    build
fi

# Start containers (images should already exist)
echo -e "${BLUE}🚀 Starting containers...${NC}"
docker compose \
  -p "$PROJECT_NAME" \
  --env-file "$ENV_FILE" \
  -f docker-compose.yaml \
  -f docker-compose.prod.yml \
  -f docker-compose.stg_prod.yml \
  -f docker-compose.e2e.yml \
  up -d

# Wait for services to be healthy
echo -e "${BLUE}⏳ Waiting for services to be ready...${NC}"

# Wait for frontend (port 50023 for E2E)
timeout=60
elapsed=0
while ! curl -f http://localhost:50023 > /dev/null 2>&1; do
  if [ $elapsed -ge $timeout ]; then
    echo -e "${RED}❌ Timeout waiting for frontend on port 50023${NC}"
    docker compose -p "$PROJECT_NAME" -f docker-compose.yaml -f docker-compose.prod.yml -f docker-compose.stg_prod.yml -f docker-compose.e2e.yml logs frontend
    exit 1
  fi
  sleep 2
  elapsed=$((elapsed + 2))
  echo -e "${YELLOW}  Waiting for frontend... (${elapsed}s/${timeout}s)${NC}"
done

echo -e "${GREEN}✅ Frontend is ready on http://localhost:50023${NC}"

# Wait for backend health check (E2E uses port 50022)
BACKEND_PORT_E2E=50022
timeout=60
elapsed=0
while ! curl -f http://localhost:${BACKEND_PORT_E2E}/health > /dev/null 2>&1; do
  if [ $elapsed -ge $timeout ]; then
    echo -e "${YELLOW}⚠️  Backend health check timeout (may still be starting)${NC}"
    break
  fi
  sleep 2
  elapsed=$((elapsed + 2))
done

echo -e "${GREEN}✅ E2E test environment is ready!${NC}"
echo -e "${BLUE}📊 Container status:${NC}"
docker compose -p "$PROJECT_NAME" -f docker-compose.yaml -f docker-compose.prod.yml -f docker-compose.e2e.yml ps
echo ""
echo -e "${BLUE}💡 To stop E2E containers:${NC}"
echo -e "   docker compose -p ${PROJECT_NAME} -f docker-compose.yaml -f docker-compose.prod.yml -f docker-compose.e2e.yml down"
echo -e "${BLUE}💡 To view logs:${NC}"
echo -e "   docker compose -p ${PROJECT_NAME} -f docker-compose.yaml -f docker-compose.prod.yml -f docker-compose.e2e.yml logs -f"

