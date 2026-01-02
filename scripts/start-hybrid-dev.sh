#!/bin/bash

# Quick start script for hybrid development
# Starts dependencies in Docker, then provides instructions for starting backend/frontend locally

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

echo -e "${BLUE}🚀 Starting Hybrid Development Environment${NC}"
echo ""

# Ensure network exists (create if it doesn't)
if ! docker network inspect hybrid_dev_env > /dev/null 2>&1; then
  echo -e "${BLUE}📡 Creating network hybrid_dev_env...${NC}"
  docker network create hybrid_dev_env
fi

# Check if containers are already running
ARANGO_RUNNING=$(docker ps -q -f name=arangodb-dev)
REDIS_RUNNING=$(docker ps -q -f name=redis-dev)

if [ -n "$ARANGO_RUNNING" ] && [ -n "$REDIS_RUNNING" ]; then
  # Containers already running - be quiet if called from setup script
  if [ "${QUIET:-false}" != "true" ]; then
    echo -e "${YELLOW}⚠️  Containers are already running${NC}"
    echo "   Using existing containers..."
  fi
else
  # Start dependencies
  # Use explicit project name to avoid "deploy" prefix
  echo -e "${BLUE}📦 Starting dependencies (ArangoDB + Redis)...${NC}"
  cd "${PROJECT_ROOT}"
  docker compose -p hybrid_dev_env -f "${PROJECT_ROOT}/deploy/docker-compose.deps.yml" --env-file "$ENV_FILE" up -d
  
  # Wait for services to be healthy
  echo -e "${BLUE}⏳ Waiting for services to be ready...${NC}"
  sleep 3
fi

# Check ArangoDB
echo -e "${BLUE}⏳ Checking ArangoDB...${NC}"
if docker exec arangodb-dev curl -sf http://localhost:8529/_admin/server/availability > /dev/null 2>&1; then
  echo -e "${GREEN}✅ ArangoDB is ready${NC}"
else
  echo -e "${YELLOW}⚠️  ArangoDB may not be ready yet (checking health...${NC}"
  # Wait a bit more and check again
  sleep 5
  if docker exec arangodb-dev curl -sf http://localhost:8529/_admin/server/availability > /dev/null 2>&1; then
    echo -e "${GREEN}✅ ArangoDB is now ready${NC}"
  else
    echo -e "${YELLOW}⚠️  ArangoDB is starting but not fully ready yet${NC}"
    echo "   You can check status with: docker ps | grep arangodb-dev"
  fi
fi

# Check Redis
echo -e "${BLUE}⏳ Checking Redis...${NC}"
if docker exec redis-dev redis-cli ping > /dev/null 2>&1; then
  echo -e "${GREEN}✅ Redis is ready${NC}"
else
  echo -e "${YELLOW}⚠️  Redis may not be ready yet${NC}"
  # Wait a bit more and check again
  sleep 2
  if docker exec redis-dev redis-cli ping > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Redis is now ready${NC}"
  else
    echo -e "${YELLOW}⚠️  Redis is starting but not fully ready yet${NC}"
    echo "   You can check status with: docker ps | grep redis-dev"
  fi
fi

echo ""
echo -e "${GREEN}✅ Dependencies started!${NC}"
echo ""
echo -e "${BLUE}📝 Next steps:${NC}"
echo ""
echo "1. Load production data (optional):"
echo "   ./scripts/load-prod-data.sh"
echo ""
echo "2. Start backend in VSCode:"
echo "   - Open Debug panel (F5)"
echo "   - Select 'Debug Backend (Hybrid Dev)'"
echo "   - Press F5 to start"
echo ""
echo "3. Start frontend:"
echo "   - Press Ctrl+Shift+P → 'Tasks: Run Task' → 'frontend: trunk serve'"
echo "   - Or in terminal: cd frontend && trunk serve"
echo ""
echo "4. Access services:"
echo "   - Frontend: http://localhost:${FRONTEND_PORT}"
echo "   - Backend API: http://localhost:${BACKEND_PORT}"
echo "   - ArangoDB: http://localhost:${ARANGODB_PORT}"
echo "   - Redis: localhost:6379"
echo ""
echo "To stop dependencies:"
echo "   docker compose -p hybrid_dev_env -f deploy/docker-compose.deps.yml --env-file config/.env.development down"



