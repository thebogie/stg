#!/bin/bash

# Build Docker images for E2E testing
# This script builds images separately, so E2E tests can just start them
# Usage: ./scripts/build-e2e-images.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEPLOY_DIR="${PROJECT_ROOT}/deploy"
ENV_FILE="${PROJECT_ROOT}/config/.env.development"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}🔨 Building Docker images for E2E testing...${NC}"

# Check if environment file exists
if [ ! -f "$ENV_FILE" ]; then
  echo -e "${RED}❌ Error: Environment file not found: $ENV_FILE${NC}"
  echo -e "${YELLOW}💡 Run: ./config/setup-env.sh development${NC}"
  exit 1
fi

# Load environment variables
if [ -f "${SCRIPT_DIR}/load-env.sh" ]; then
  source "${SCRIPT_DIR}/load-env.sh" development
  ENV_FILE="${PROJECT_ROOT}/config/.env.development"
fi

cd "$DEPLOY_DIR"
export ENV_FILE

# Build images without starting containers
# Uses EXACT same compose files as production to ensure identical builds
echo -e "${BLUE}📦 Building images (this may take 5-10 minutes)...${NC}"
echo -e "${BLUE}   Using production compose files for identical builds${NC}"
docker compose \
  -p e2e_env \
  --env-file "$ENV_FILE" \
  -f docker-compose.yaml \
  -f docker-compose.prod.yml \
  -f docker-compose.stg_prod.yml \
  -f docker-compose.e2e.yml \
  build

echo -e "${GREEN}✅ Images built successfully!${NC}"
echo -e "${BLUE}💡 Now you can run E2E tests with: just test-frontend-e2e${NC}"
echo -e "${BLUE}💡 Or start containers manually: docker compose -p e2e_env -f docker-compose.yaml -f docker-compose.prod.yml -f docker-compose.e2e.yml up -d${NC}"

