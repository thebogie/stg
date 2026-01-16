#!/bin/bash

# Check if Redis cache is working
# This script verifies Redis connectivity and shows cache statistics

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Load environment - prefer production, fall back to development
ENV_FILE=""
if [ -f "$PROJECT_ROOT/config/.env.production" ]; then
    ENV_FILE="$PROJECT_ROOT/config/.env.production"
    source "$ENV_FILE"
    log_info "Using production environment"
elif [ -f "$PROJECT_ROOT/config/.env.development" ]; then
    ENV_FILE="$PROJECT_ROOT/config/.env.development"
    source "$ENV_FILE"
    log_info "Using development environment"
fi

REDIS_PORT="${REDIS_PORT:-63791}"
REDIS_URL="${REDIS_URL:-redis://localhost:${REDIS_PORT}/}"

log_info "Checking Redis cache status..."
log_info "Redis URL: $REDIS_URL"
log_info ""

# Method 1: Check Redis container health
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "Method 1: Check Redis Container Health"
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

REDIS_CONTAINER=""
if docker ps --format "{{.Names}}" | grep -q "^redis$"; then
    REDIS_CONTAINER="redis"
elif docker ps --format "{{.Names}}" | grep -q "redis"; then
    REDIS_CONTAINER=$(docker ps --format "{{.Names}}" | grep redis | head -1)
    log_info "Found Redis container: $REDIS_CONTAINER"
fi

if [ -n "$REDIS_CONTAINER" ]; then
    log_success "Redis container is running: $REDIS_CONTAINER"
    
    # Check if Redis is responding
    if docker exec "$REDIS_CONTAINER" redis-cli ping > /dev/null 2>&1; then
        log_success "Redis is responding to PING"
    else
        log_error "Redis is not responding to PING"
        log_info "Checking if Redis requires password..."
        # Try with password if REDIS_PASSWORD is set
        if [ -n "${REDIS_PASSWORD:-}" ]; then
            if docker exec "$REDIS_CONTAINER" redis-cli -a "$REDIS_PASSWORD" ping > /dev/null 2>&1; then
                log_success "Redis is responding with password"
            else
                log_error "Redis is not responding even with password"
                exit 1
            fi
        else
            exit 1
        fi
    fi
else
    log_warning "Redis container not found (may be using external Redis)"
    log_info "Checking if Redis is accessible at $REDIS_URL"
fi

# Method 2: Check via redis-cli (if available)
log_info ""
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "Method 2: Check Redis via redis-cli"
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if command -v redis-cli &> /dev/null; then
    # Extract host and port from REDIS_URL
    if [[ "$REDIS_URL" =~ redis://([^:]+):([0-9]+) ]]; then
        REDIS_HOST="${BASH_REMATCH[1]}"
        REDIS_PORT_CLI="${BASH_REMATCH[2]}"
        
        if redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT_CLI" ping > /dev/null 2>&1; then
            log_success "Redis is accessible via redis-cli"
            
            # Show Redis info
            log_info "Redis Info:"
            redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT_CLI" INFO server | head -5 || true
            
            # Count cache keys
            log_info ""
            log_info "Cache Keys (by prefix):"
            redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT_CLI" --scan --pattern "stg:cache:*" | wc -l | xargs -I {} echo "  stg:cache:*: {} keys" || echo "  stg:cache:*: 0 keys"
            redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT_CLI" --scan --pattern "game:*" | wc -l | xargs -I {} echo "  game:*: {} keys" || echo "  game:*: 0 keys"
            redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT_CLI" --scan --pattern "venue:*" | wc -l | xargs -I {} echo "  venue:*: {} keys" || echo "  venue:*: 0 keys"
            redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT_CLI" --scan --pattern "player:*" | wc -l | xargs -I {} echo "  player:*: {} keys" || echo "  player:*: 0 keys"
            redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT_CLI" --scan --pattern "analytics:*" | wc -l | xargs -I {} echo "  analytics:*: {} keys" || echo "  analytics:*: 0 keys"
        else
            log_error "Cannot connect to Redis via redis-cli"
        fi
    else
        log_warning "Could not parse REDIS_URL: $REDIS_URL"
    fi
elif [ -n "$REDIS_CONTAINER" ]; then
    # Use docker exec if redis-cli not available
    log_info "Using docker exec to check Redis..."
    
    # Build redis-cli command with password if needed
    REDIS_CLI_CMD="redis-cli"
    if [ -n "${REDIS_PASSWORD:-}" ]; then
        REDIS_CLI_CMD="redis-cli -a $REDIS_PASSWORD"
    fi
    
    if docker exec "$REDIS_CONTAINER" $REDIS_CLI_CMD ping > /dev/null 2>&1; then
        log_success "Redis is accessible via docker exec"
        
        log_info "Redis Info:"
        docker exec "$REDIS_CONTAINER" $REDIS_CLI_CMD INFO server 2>/dev/null | head -5 || true
        
        log_info ""
        log_info "Cache Keys (by prefix):"
        docker exec "$REDIS_CONTAINER" $REDIS_CLI_CMD --scan --pattern "stg:cache:*" 2>/dev/null | wc -l | xargs -I {} echo "  stg:cache:*: {} keys" || echo "  stg:cache:*: 0 keys"
        docker exec "$REDIS_CONTAINER" $REDIS_CLI_CMD --scan --pattern "game:*" 2>/dev/null | wc -l | xargs -I {} echo "  game:*: {} keys" || echo "  game:*: 0 keys"
        docker exec "$REDIS_CONTAINER" $REDIS_CLI_CMD --scan --pattern "venue:*" 2>/dev/null | wc -l | xargs -I {} echo "  venue:*: {} keys" || echo "  venue:*: 0 keys"
        docker exec "$REDIS_CONTAINER" $REDIS_CLI_CMD --scan --pattern "player:*" 2>/dev/null | wc -l | xargs -I {} echo "  player:*: {} keys" || echo "  player:*: 0 keys"
        docker exec "$REDIS_CONTAINER" $REDIS_CLI_CMD --scan --pattern "analytics:*" 2>/dev/null | wc -l | xargs -I {} echo "  analytics:*: {} keys" || echo "  analytics:*: 0 keys"
        
        log_info ""
        log_info "Redis Statistics:"
        docker exec "$REDIS_CONTAINER" $REDIS_CLI_CMD INFO stats 2>/dev/null | grep -E "keyspace_hits|keyspace_misses" || true
    fi
else
    log_warning "redis-cli not available and Redis container not found"
fi

# Method 3: Check backend health endpoint
log_info ""
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "Method 3: Check Backend Health Endpoint"
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

BACKEND_PORT="${BACKEND_PORT:-50002}"
if curl -s -f "http://localhost:${BACKEND_PORT}/health/detailed" > /dev/null 2>&1; then
    log_success "Backend health endpoint is accessible"
    log_info "Detailed health check response:"
    curl -s "http://localhost:${BACKEND_PORT}/health/detailed" | python3 -m json.tool 2>/dev/null || curl -s "http://localhost:${BACKEND_PORT}/health/detailed"
else
    log_warning "Backend health endpoint not accessible at http://localhost:${BACKEND_PORT}/health/detailed"
fi

# Method 4: Check backend logs for cache activity
log_info ""
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "Method 4: Check Backend Logs for Cache Activity"
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

BACKEND_CONTAINER=""
if docker ps --format "{{.Names}}" | grep -q "^backend$"; then
    BACKEND_CONTAINER="backend"
elif docker ps --format "{{.Names}}" | grep -q "backend"; then
    BACKEND_CONTAINER=$(docker ps --format "{{.Names}}" | grep backend | head -1)
fi

if [ -n "$BACKEND_CONTAINER" ]; then
    log_info "Recent cache-related log entries from $BACKEND_CONTAINER:"
    docker logs "$BACKEND_CONTAINER" 2>&1 | grep -iE "cache|redis" | tail -10 || log_warning "No cache-related log entries found (may need to enable debug logging with RUST_LOG=debug)"
    log_info ""
    log_info "To see cache hits/misses in real-time, run:"
    log_info "  docker logs -f $BACKEND_CONTAINER | grep -iE 'cache hit|cache miss'"
else
    log_warning "Backend container not found"
fi

# Method 5: Test cache by making API calls
log_info ""
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "Method 5: Test Cache with API Calls"
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

log_info "To test if cache is working:"
log_info "1. Make an API call (e.g., GET /api/games)"
log_info "2. Make the same call again immediately"
log_info "3. The second call should be faster if cache is working"
log_info ""
log_info "Example test:"
log_info "  time curl -s http://localhost:${BACKEND_PORT}/api/games > /dev/null"
log_info "  time curl -s http://localhost:${BACKEND_PORT}/api/games > /dev/null"
log_info ""
log_info "The second call should be significantly faster if cache is working."

log_info ""
log_success "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_success "Redis Cache Check Complete"
log_success "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
