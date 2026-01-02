#!/bin/bash

# One-command setup for hybrid development
# Starts dependencies and optionally loads production data
# Usage: ./scripts/setup-hybrid-dev.sh [--skip-data]

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Load environment variables
source "${SCRIPT_DIR}/load-env.sh"

SKIP_DATA=false
if [ "$1" = "--skip-data" ]; then
  SKIP_DATA=true
fi

echo -e "${BLUE}🚀 Setting up Hybrid Development Environment${NC}"
echo ""

# Check environment file
ENV_FILE="${PROJECT_ROOT}/config/.env.development"
if [ ! -f "$ENV_FILE" ]; then
  echo -e "${YELLOW}⚠️  Environment file not found: $ENV_FILE${NC}"
  echo "   Run: ./config/setup-env.sh development"
  exit 1
fi

# Step 1: Start dependencies (handles existing containers)
echo -e "${BLUE}📦 Step 1: Starting dependencies (ArangoDB + Redis)...${NC}"

# Ensure network exists
if ! docker network inspect hybrid_dev_env > /dev/null 2>&1; then
  docker network create hybrid_dev_env > /dev/null 2>&1
fi

# Check if containers are already running
ARANGO_RUNNING=$(docker ps -q -f name=arangodb-dev)
REDIS_RUNNING=$(docker ps -q -f name=redis-dev)

if [ -n "$ARANGO_RUNNING" ] && [ -n "$REDIS_RUNNING" ]; then
  echo -e "${GREEN}✅ Containers already running${NC}"
else
  cd "${PROJECT_ROOT}"
  docker compose -p hybrid_dev_env -f "${PROJECT_ROOT}/deploy/docker-compose.deps.yml" --env-file "$ENV_FILE" up -d > /dev/null 2>&1
  echo -e "${GREEN}✅ Containers started${NC}"
  sleep 3
fi

# Quick health check
if docker exec redis-dev redis-cli ping > /dev/null 2>&1; then
  echo -e "${GREEN}✅ Redis is ready${NC}"
else
  echo -e "${YELLOW}⚠️  Redis starting...${NC}"
fi

if docker exec arangodb-dev nc -z 127.0.0.1 8529 > /dev/null 2>&1; then
  echo -e "${GREEN}✅ ArangoDB is ready${NC}"
else
  echo -e "${YELLOW}⚠️  ArangoDB starting...${NC}"
fi

# Step 2: Load production data (optional)
if [ "$SKIP_DATA" = false ]; then
  echo ""
  echo -e "${BLUE}📊 Step 2: Loading production data (if available)...${NC}"
  if "${PROJECT_ROOT}/scripts/load-prod-data.sh" 2>&1 | grep -q "No data dump file found"; then
    echo -e "${YELLOW}⚠️  No data dump found - skipping data load${NC}"
    echo "   To load data later: ./scripts/load-prod-data.sh"
  else
    echo -e "${GREEN}✅ Data loaded${NC}"
  fi
else
  echo -e "${YELLOW}⏭️  Skipping data load (--skip-data flag)${NC}"
fi

echo ""
echo -e "${GREEN}✅ Hybrid development environment is ready!${NC}"
echo ""
echo -e "${BLUE}📝 Next steps:${NC}"
echo ""
echo "1. Start backend in VSCode:"
echo "   - Open Debug panel (F5)"
echo "   - Select 'Debug Backend (Hybrid Dev)'"
echo "   - Press F5 to start"
echo ""
echo "2. Start frontend:"
echo "   - Run: ./scripts/start-frontend.sh"
echo "   - Or VSCode: Ctrl+Shift+P → 'Tasks: Run Task' → 'frontend: trunk serve'"
echo ""
echo "3. Access services:"
echo "   - Frontend: http://localhost:${FRONTEND_PORT}"
echo "   - Backend API: http://localhost:${BACKEND_PORT}"
echo "   - ArangoDB: http://localhost:${ARANGODB_PORT}"
echo ""
echo "To stop dependencies:"
echo "   docker compose -p hybrid_dev_env -f deploy/docker-compose.deps.yml --env-file config/.env.development down"

