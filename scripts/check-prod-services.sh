#!/bin/bash

# Check if production services are running correctly
# Usage: ./scripts/check-prod-services.sh

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

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

cd "$PROJECT_ROOT"

log_info "Checking production services..."

# Check if containers are running
log_info "Container status:"
cd deploy
export ENV_FILE="../config/.env.production"
if [ -f "docker-compose.stg_prod.yml" ]; then
    docker compose --env-file "$ENV_FILE" -f docker-compose.yaml -f docker-compose.prod.yml -f docker-compose.stg_prod.yml ps
else
    docker compose --env-file "$ENV_FILE" -f docker-compose.yaml -f docker-compose.prod.yml ps
fi

echo ""
log_info "Checking network connectivity..."

# Check if network exists
if docker network ls | grep -q "stg_prod"; then
    log_success "Network 'stg_prod' exists"
else
    log_warning "Network 'stg_prod' not found. Creating it..."
    docker network create stg_prod
fi

# Check if ArangoDB is accessible
if docker ps --format "{{.Names}}" | grep -q "^arangodb$"; then
    log_success "ArangoDB container is running"
    
    # Check if backend can reach ArangoDB (use ArangoDB container's curl or wget)
    if docker exec arangodb curl -f http://localhost:8529/_api/version >/dev/null 2>&1; then
        log_success "ArangoDB is responding on port 8529"
        
        # Test network connectivity from backend's perspective
        if docker exec backend sh -c "nc -z arangodb 8529" >/dev/null 2>&1 || \
           docker exec backend sh -c "timeout 2 bash -c 'cat < /dev/null > /dev/tcp/arangodb/8529'" >/dev/null 2>&1; then
            log_success "Backend can reach ArangoDB on network"
        else
            log_warning "Backend cannot reach ArangoDB on network (might be network issue)"
        fi
    else
        log_error "ArangoDB is not responding"
    fi
else
    log_error "ArangoDB container is not running!"
    log_info "Start all services with: ./scripts/prod-compose.sh up -d"
fi

# Check if Redis is accessible
if docker ps --format "{{.Names}}" | grep -q "^redis$"; then
    log_success "Redis container is running"
else
    log_warning "Redis container is not running"
fi

# Check if backend is running
if docker ps --format "{{.Names}}" | grep -q "^backend$"; then
    log_success "Backend container is running"
else
    log_warning "Backend container is not running"
fi

# Check if frontend is running
if docker ps --format "{{.Names}}" | grep -q "^frontend$"; then
    log_success "Frontend container is running"
else
    log_warning "Frontend container is not running"
fi

echo ""
log_info "Recent logs from backend (last 20 lines):"
docker logs --tail 20 backend 2>&1 | tail -20

