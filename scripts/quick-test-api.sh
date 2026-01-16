#!/bin/bash

# Quick API test script - faster than running full test suite
# Tests the db_search endpoints to see actual response structure
# Usage: ./scripts/quick-test-api.sh

set -euo pipefail

BACKEND_URL="${BACKEND_URL:-http://localhost:50002}"

echo "Testing API endpoints..."
echo "BACKEND_URL: $BACKEND_URL"
echo ""

# Test games search
echo "=== Games Search ==="
curl -s "${BACKEND_URL}/api/games/db_search?query=har" | jq '.[0]' 2>/dev/null || curl -s "${BACKEND_URL}/api/games/db_search?query=har"
echo ""
echo ""

# Test venues search
echo "=== Venues Search ==="
curl -s "${BACKEND_URL}/api/venues/db_search?query=coffee" | jq '.[0]' 2>/dev/null || curl -s "${BACKEND_URL}/api/venues/db_search?query=coffee"
echo ""
echo ""

# Test players search
echo "=== Players Search ==="
curl -s "${BACKEND_URL}/api/players/db_search?query=mit" | jq '.[0]' 2>/dev/null || curl -s "${BACKEND_URL}/api/players/db_search?query=mit"
echo ""
