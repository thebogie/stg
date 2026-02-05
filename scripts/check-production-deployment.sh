#!/bin/bash

# Check what's actually deployed in production
# This helps diagnose if the correct version is running

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }
log_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
log_error() { echo -e "${RED}❌ $1${NC}"; }

cd "$PROJECT_ROOT"

# Find frontend container
FRONTEND_CONTAINER=""
if docker ps --format '{{.Names}}' | grep -qE "^frontend$|^stg.*frontend"; then
    FRONTEND_CONTAINER=$(docker ps --format '{{.Names}}' | grep -E "^frontend$|^stg.*frontend" | head -1)
else
    log_error "No frontend container found!"
    docker ps -a --format '{{.Names}}' | head -10
    exit 1
fi

log_info "Checking production deployment..."
log_info "Container: $FRONTEND_CONTAINER"

# Check what image is running
log_info ""
log_info "=== Container Image Info ==="
docker inspect "$FRONTEND_CONTAINER" --format '{{.Config.Image}}' || log_warning "Could not get image name"

# Check WASM file modification time
log_info ""
log_info "=== WASM File Info ==="
WASM_FILE=""
if docker exec "$FRONTEND_CONTAINER" test -f /usr/share/nginx/html/frontend_bg.optimized.wasm 2>/dev/null; then
    WASM_FILE="/usr/share/nginx/html/frontend_bg.optimized.wasm"
elif docker exec "$FRONTEND_CONTAINER" test -f /usr/share/caddy/frontend_bg.optimized.wasm 2>/dev/null; then
    WASM_FILE="/usr/share/caddy/frontend_bg.optimized.wasm"
else
    log_error "WASM file not found!"
    exit 1
fi

log_info "WASM file: $WASM_FILE"
docker exec "$FRONTEND_CONTAINER" stat "$WASM_FILE" 2>/dev/null | grep -E "Modify|Change" || true

# Extract strings from WASM to check for "Players" vs "People"
log_info ""
log_info "=== Checking WASM Content ==="
if docker exec "$FRONTEND_CONTAINER" which strings >/dev/null 2>&1; then
    log_info "Extracting strings from WASM..."
    WASM_STRINGS=$(docker exec "$FRONTEND_CONTAINER" strings "$WASM_FILE" 2>/dev/null || true)
    
    if echo "$WASM_STRINGS" | grep -qi "Search People\|Search people"; then
        log_error "❌ Found 'Search People' in WASM!"
        echo "$WASM_STRINGS" | grep -i "Search People\|Search people" | head -5
    else
        log_success "✅ No 'Search People' found in WASM"
    fi
    
    if echo "$WASM_STRINGS" | grep -qi "Players"; then
        log_success "✅ Found 'Players' in WASM"
        echo "$WASM_STRINGS" | grep -i "Players" | head -5
    else
        log_warning "⚠️  'Players' not found in WASM"
    fi
else
    log_warning "strings command not available in container"
    log_info "Copying WASM file to host for analysis..."
    docker cp "${FRONTEND_CONTAINER}:${WASM_FILE}" /tmp/check_wasm.wasm 2>/dev/null || {
        log_error "Could not copy WASM file"
        exit 1
    }
    
    if which strings >/dev/null 2>&1; then
        log_info "Extracting strings from WASM..."
        WASM_STRINGS=$(strings /tmp/check_wasm.wasm 2>/dev/null || true)
        
        if echo "$WASM_STRINGS" | grep -qi "Search People\|Search people"; then
            log_error "❌ Found 'Search People' in WASM!"
            echo "$WASM_STRINGS" | grep -i "Search People\|Search people" | head -5
        else
            log_success "✅ No 'Search People' found in WASM"
        fi
        
        if echo "$WASM_STRINGS" | grep -qi "Players"; then
            log_success "✅ Found 'Players' in WASM"
            echo "$WASM_STRINGS" | grep -i "Players" | head -5
        else
            log_warning "⚠️  'Players' not found in WASM"
        fi
        
        rm -f /tmp/check_wasm.wasm
    else
        log_error "strings command not available on host either"
    fi
fi

# Check HTML/JS files
log_info ""
log_info "=== Checking HTML/JS Files ==="
HTML_DIR=""
if docker exec "$FRONTEND_CONTAINER" test -d /usr/share/nginx/html 2>/dev/null; then
    HTML_DIR="/usr/share/nginx/html"
elif docker exec "$FRONTEND_CONTAINER" test -d /usr/share/caddy 2>/dev/null; then
    HTML_DIR="/usr/share/caddy"
else
    log_error "Could not find HTML directory"
    exit 1
fi

log_info "HTML directory: $HTML_DIR"

# Check for "Search People" in HTML/JS
SEARCH_PEOPLE_COUNT=$(docker exec "$FRONTEND_CONTAINER" grep -r "Search People\|Search people" "$HTML_DIR" 2>/dev/null | grep -v ".map" | wc -l | tr -d ' ' || echo "0")
if [ "$SEARCH_PEOPLE_COUNT" -gt 0 ]; then
    log_error "❌ Found $SEARCH_PEOPLE_COUNT instances of 'Search People' in HTML/JS!"
    docker exec "$FRONTEND_CONTAINER" grep -r "Search People\|Search people" "$HTML_DIR" 2>/dev/null | grep -v ".map" | head -5
else
    log_success "✅ No 'Search People' found in HTML/JS"
fi

# Check for "Players" in HTML/JS
PLAYERS_COUNT=$(docker exec "$FRONTEND_CONTAINER" grep -r "Players" "$HTML_DIR" 2>/dev/null | grep -v ".map" | wc -l | tr -d ' ' || echo "0")
if [ "$PLAYERS_COUNT" -gt 0 ]; then
    log_success "✅ Found $PLAYERS_COUNT instances of 'Players' in HTML/JS"
else
    log_warning "⚠️  'Players' not found in HTML/JS"
fi

log_info ""
log_info "=== Summary ==="
log_info "If you see 'Search People' above, the container has old code."
log_info "If you see 'Players' but not 'Search People', the container has correct code."
log_info ""
log_info "If container has correct code but browser shows 'Search People':"
log_info "  1. Hard refresh browser (Ctrl+Shift+R or Cmd+Shift+R)"
log_info "  2. Clear browser cache"
log_info "  3. Check if there's a CDN/proxy cache"
log_info "  4. Try incognito/private browsing mode"
