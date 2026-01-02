#!/bin/bash

# Unified deployment script for stg_rd
# Supports both development and production deployments

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get script directory (deploy/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Defaults
ENVIRONMENT="development"
ENV_FILE=""
ACTION="up"
SERVICE=""
BUILD=false
NO_CACHE=false
FORCE=false

# Usage function
usage() {
  echo -e "${YELLOW}Usage: $0 [options]${NC}"
  echo -e ""
  echo -e "Options:"
  echo -e "  --env ENV              Environment: development (default) or production"
  echo -e "  --env-file FILE       Custom .env file path (relative to config/)"
  echo -e "  --build               Build images before starting"
  echo -e "  --no-cache            Build without cache"
  echo -e "  --down                Stop and remove containers"
  echo -e "  --restart             Restart containers"
  echo -e "  --logs [SERVICE]      Show logs (all or specific service)"
  echo -e "  --status              Show container status"
  echo -e "  --clean               Remove containers, volumes, and images"
  echo -e "  --force               Use with --clean to also prune system"
  echo -e "  --help                Show this help message"
  echo -e ""
  echo -e "Examples:"
  echo -e "  $0                                    # Start development"
  echo -e "  $0 --env production --build          # Build and start production"
  echo -e "  $0 --env development --logs backend   # Show backend logs"
  echo -e "  $0 --down                             # Stop all containers"
  exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --env)
      ENVIRONMENT="$2"
      shift 2
      ;;
    --env-file)
      ENV_FILE="$2"
      shift 2
      ;;
    --build)
      BUILD=true
      shift
      ;;
    --no-cache)
      NO_CACHE=true
      BUILD=true
      shift
      ;;
    --down)
      ACTION="down"
      shift
      ;;
    --restart)
      ACTION="restart"
      shift
      ;;
    --logs)
      ACTION="logs"
      if [[ -n "$2" && ! "$2" =~ ^-- ]]; then
        SERVICE="$2"
        shift 2
      else
        shift
      fi
      ;;
    --status)
      ACTION="status"
      shift
      ;;
    --clean)
      ACTION="clean"
      shift
      ;;
    --force)
      FORCE=true
      shift
      ;;
    --help)
      usage
      ;;
    *)
      echo -e "${RED}❌ Unknown argument: $1${NC}"
      usage
      ;;
  esac
done

# Determine environment file
if [ -z "$ENV_FILE" ]; then
  if [ "$ENVIRONMENT" = "production" ]; then
    ENV_FILE="$PROJECT_ROOT/config/.env.production"
  else
    ENV_FILE="$PROJECT_ROOT/config/.env.development"
  fi
else
  # If relative path, assume it's in config/
  if [[ ! "$ENV_FILE" =~ ^/ ]]; then
    ENV_FILE="$PROJECT_ROOT/config/$ENV_FILE"
  fi
fi

# Check if environment file exists
if [ ! -f "$ENV_FILE" ]; then
  echo -e "${RED}❌ Error: Environment file not found: $ENV_FILE${NC}"
  echo -e "${YELLOW}💡 Tip: Copy config/env.${ENVIRONMENT}.template to config/.env.${ENVIRONMENT}${NC}"
  exit 1
fi

# Docker compose files
COMPOSE_BASE="$SCRIPT_DIR/docker-compose.yaml"
if [ "$ENVIRONMENT" = "production" ]; then
  COMPOSE_OVERRIDE="$SCRIPT_DIR/docker-compose.prod.yml"
else
  COMPOSE_OVERRIDE="$SCRIPT_DIR/docker-compose.dev.yml"
fi

# Check compose files exist
if [ ! -f "$COMPOSE_BASE" ]; then
  echo -e "${RED}❌ Error: docker-compose.yaml not found in $SCRIPT_DIR${NC}"
  exit 1
fi

if [ ! -f "$COMPOSE_OVERRIDE" ]; then
  echo -e "${RED}❌ Error: docker-compose override not found: $COMPOSE_OVERRIDE${NC}"
  exit 1
fi

# Export ENV_FILE for docker-compose
export ENV_FILE

# Build info for production
if [ "$ENVIRONMENT" = "production" ] && [ "$BUILD" = true ]; then
  echo -e "${BLUE}📋 Getting build information...${NC}"
  if [ -f "$PROJECT_ROOT/scripts/build-info.sh" ]; then
    source "$PROJECT_ROOT/scripts/build-info.sh"
    export GIT_COMMIT
    export BUILD_DATE
    echo -e "  Git Commit: ${GIT_COMMIT:-unknown}"
    echo -e "  Build Date: ${BUILD_DATE:-unknown}"
  fi
fi

# Docker compose command builder
compose_cmd() {
  local cmd="$1"
  shift
  docker compose \
    --env-file "$ENV_FILE" \
    -f "$COMPOSE_BASE" \
    -f "$COMPOSE_OVERRIDE" \
    "$cmd" "$@"
}

# Handle different actions
case "$ACTION" in
  up)
    echo -e "${GREEN}🚀 Starting $ENVIRONMENT environment...${NC}"
    if [ "$BUILD" = true ]; then
      if [ "$NO_CACHE" = true ]; then
        compose_cmd build --no-cache
      else
        compose_cmd build
      fi
    fi
    compose_cmd up -d
    echo -e "${GREEN}✅ Services started!${NC}"
    echo -e "${BLUE}📊 Status:${NC}"
    compose_cmd ps
    ;;
    
  down)
    echo -e "${YELLOW}🛑 Stopping containers...${NC}"
    compose_cmd down
    echo -e "${GREEN}✅ Containers stopped${NC}"
    ;;
    
  restart)
    echo -e "${YELLOW}🔄 Restarting containers...${NC}"
    compose_cmd restart
    echo -e "${GREEN}✅ Containers restarted${NC}"
    ;;
    
  logs)
    echo -e "${BLUE}📜 Showing logs...${NC}"
    if [ -n "$SERVICE" ]; then
      compose_cmd logs -f "$SERVICE"
    else
      compose_cmd logs -f
    fi
    ;;
    
  status)
    echo -e "${BLUE}📊 Container Status:${NC}"
    compose_cmd ps
    ;;
    
  clean)
    echo -e "${YELLOW}🧹 Cleaning containers, volumes, and images...${NC}"
    compose_cmd down -v --remove-orphans
    
    if [ "$FORCE" = true ]; then
      echo -e "${RED}⚠️  Running: docker system prune -af --volumes${NC}"
      docker system prune -af --volumes
    else
      echo -e "${YELLOW}💡 Use --force to also prune system resources${NC}"
    fi
    
    echo -e "${GREEN}✅ Cleanup complete${NC}"
    ;;
    
  *)
    echo -e "${RED}❌ Unknown action: $ACTION${NC}"
    exit 1
    ;;
esac



