#!/bin/bash

# Stop Docker containers for E2E tests
# Stops the isolated e2e_env environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEPLOY_DIR="${PROJECT_ROOT}/deploy"
ENV_FILE="${PROJECT_ROOT}/config/.env.development"
PROJECT_NAME="e2e_env"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${YELLOW}🛑 Stopping E2E test containers (${PROJECT_NAME})...${NC}"

# Load environment variables from config/.env.development
if [ -f "${SCRIPT_DIR}/load-env.sh" ]; then
  source "${SCRIPT_DIR}/load-env.sh" development
fi

cd "$DEPLOY_DIR"

# Export ENV_FILE for docker-compose
export ENV_FILE

docker compose \
  -p "$PROJECT_NAME" \
  --env-file "$ENV_FILE" \
  -f docker-compose.production.yml \
  -f docker-compose.e2e.yml \
  down

echo -e "${GREEN}✅ E2E test containers stopped${NC}"
echo -e "${BLUE}💡 Network ${PROJECT_NAME} is preserved (can be reused)${NC}"
echo -e "${BLUE}💡 To remove network: docker network rm ${PROJECT_NAME}${NC}"

