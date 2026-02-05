#!/bin/bash

# Verify what's actually in the production container's WASM file
# This helps diagnose if the deployed container has the correct code

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

# Check if container is running
FRONTEND_CONTAINER=""
if docker ps --format '{{.Names}}' | grep -qE "^frontend$|^stg.*frontend|^e2e_frontend"; then
    FRONTEND_CONTAINER=$(docker ps --format '{{.Names}}' | grep -E "^frontend$|^stg.*frontend|^e2e_frontend" | head -1)
elif docker ps -a --format '{{.Names}}' | grep -qE "^frontend$|^stg.*frontend|^e2e_frontend"; then
    FRONTEND_CONTAINER=$(docker ps -a --format '{{.Names}}' | grep -E "^frontend$|^stg.*frontend|^e2e_frontend" | head -1)
    log_warning "Container exists but is not running: $FRONTEND_CONTAINER"
    log_info "Starting container to check..."
    docker start "$FRONTEND_CONTAINER" || true
    sleep 2
else
    log_error "No frontend container found!"
    log_info "Available containers:"
    docker ps -a --format '{{.Names}}' | head -10
    exit 1
fi

log_info "Checking container: $FRONTEND_CONTAINER"

# Check WASM file in container (hashed *_bg.wasm first, then legacy optimized name)
log_info "Checking WASM file in container..."
WASM_FILE=""
WASM_FILE=$(docker exec "$FRONTEND_CONTAINER" find /usr/share/nginx/html -name '*_bg.wasm' -type f 2>/dev/null | head -1 || echo "")
if [ -z "$WASM_FILE" ]; then
    WASM_FILE=$(docker exec "$FRONTEND_CONTAINER" find /usr/share/caddy -name '*_bg.wasm' -type f 2>/dev/null | head -1 || echo "")
fi
if [ -z "$WASM_FILE" ] && docker exec "$FRONTEND_CONTAINER" test -f /usr/share/nginx/html/frontend_bg.optimized.wasm 2>/dev/null; then
    WASM_FILE="/usr/share/nginx/html/frontend_bg.optimized.wasm"
elif [ -z "$WASM_FILE" ]; then
    log_warning "WASM not found, listing..."
    docker exec "$FRONTEND_CONTAINER" find /usr/share -name "*.wasm" -type f 2>/dev/null | head -5
fi

if [ -z "$WASM_FILE" ]; then
    log_error "No WASM file found in container!"
    exit 1
fi

log_info "Found WASM file: $WASM_FILE"

# Check file modification time
log_info "WASM file modification time:"
docker exec "$FRONTEND_CONTAINER" stat "$WASM_FILE" 2>/dev/null | grep -E "Modify|Change" || true

# Extract strings from WASM to check for "People" vs "Players"
log_info "Extracting strings from WASM file..."
TEMP_FILE="/tmp/wasm_strings_$$.txt"
docker exec "$FRONTEND_CONTAINER" strings "$WASM_FILE" 2>/dev/null > "$TEMP_FILE" || {
    log_warning "Could not extract strings (strings command not available in container)"
    log_info "Trying alternative method..."
    docker cp "${FRONTEND_CONTAINER}:${WASM_FILE}" /tmp/check_wasm.wasm 2>/dev/null || {
        log_error "Could not copy WASM file from container"
        exit 1
    }
    strings /tmp/check_wasm.wasm > "$TEMP_FILE" 2>/dev/null || {
        log_warning "strings command not available on host either"
        rm -f /tmp/check_wasm.wasm
    }
}

if [ -f "$TEMP_FILE" ]; then
    log_info "Checking for 'People' in WASM strings..."
    if grep -q "People" "$TEMP_FILE" 2>/dev/null && ! grep -q "Players" "$TEMP_FILE" 2>/dev/null; then
        log_error "❌ FOUND 'People' in WASM file (and no 'Players')!"
        log_error "This confirms the container has old code."
        grep "People" "$TEMP_FILE" | head -3
        rm -f "$TEMP_FILE" /tmp/check_wasm.wasm 2>/dev/null || true
        exit 1
    elif grep -q "Players" "$TEMP_FILE" 2>/dev/null; then
        log_success "✅ Found 'Players' in WASM file"
        if grep -q "People" "$TEMP_FILE" 2>/dev/null; then
            log_warning "⚠️  Also found 'People' (might be in comments or old code paths)"
            grep "People" "$TEMP_FILE" | head -3
        fi
        log_info "Sample 'Players' matches:"
        grep "Players" "$TEMP_FILE" | head -3
    else
        log_warning "⚠️  Neither 'People' nor 'Players' found in WASM strings"
        log_info "This might mean the text is compiled differently"
    fi
    rm -f "$TEMP_FILE" /tmp/check_wasm.wasm 2>/dev/null || true
else
    log_warning "Could not extract strings from WASM"
fi

# Check HTML/JS files too
log_info "Checking HTML/JS files in container..."
if docker exec "$FRONTEND_CONTAINER" test -d /usr/share/nginx/html 2>/dev/null; then
    HTML_DIR="/usr/share/nginx/html"
elif docker exec "$FRONTEND_CONTAINER" test -d /usr/share/caddy 2>/dev/null; then
    HTML_DIR="/usr/share/caddy"
else
    log_error "Could not find HTML directory"
    exit 1
fi

log_info "Checking for 'People' in HTML/JS files..."
PEOPLE_COUNT=$(docker exec "$FRONTEND_CONTAINER" grep -r "People" "$HTML_DIR" 2>/dev/null | grep -v "Players" | grep -v ".map" | wc -l | tr -d ' ' || echo "0")
PLAYERS_COUNT=$(docker exec "$FRONTEND_CONTAINER" grep -r "Players" "$HTML_DIR" 2>/dev/null | wc -l | tr -d ' ' || echo "0")

if [ "$PEOPLE_COUNT" -gt 0 ]; then
    log_error "❌ Found $PEOPLE_COUNT instances of 'People' in HTML/JS files!"
    docker exec "$FRONTEND_CONTAINER" grep -r "People" "$HTML_DIR" 2>/dev/null | grep -v "Players" | grep -v ".map" | head -5
    exit 1
elif [ "$PLAYERS_COUNT" -gt 0 ]; then
    log_success "✅ Found $PLAYERS_COUNT instances of 'Players' in HTML/JS files"
    log_info "Container appears to have correct code"
else
    log_warning "⚠️  Neither 'People' nor 'Players' found in HTML/JS files"
fi

log_success "Verification complete!"
