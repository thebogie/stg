#!/bin/bash

# Check what version is deployed and what containers are using
# Usage: ./scripts/check-deployed-version.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Deployed Version Check"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

VERSION_FILE="${PROJECT_ROOT}/_build/.deployed-version"
if [ -f "$VERSION_FILE" ]; then
    echo "✅ Deployed version file exists: $VERSION_FILE"
    echo ""
    echo "Contents:"
    cat "$VERSION_FILE"
    echo ""
else
    echo "❌ Deployed version file NOT found: $VERSION_FILE"
    echo "   This means no deployment has been run yet, or the file wasn't created."
    echo ""
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Container Images"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

docker ps --format "table {{.Names}}\t{{.Image}}" | grep -E "^(frontend|backend|NAMES)" || true

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Environment Variables in Containers"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if docker ps --format "{{.Names}}" | grep -q "^backend$"; then
    echo "Backend container:"
    echo "  IMAGE_TAG: $(docker exec backend sh -c 'echo ${IMAGE_TAG:-not set}' 2>/dev/null || echo 'not accessible')"
    echo "  BACKEND_IMAGE: $(docker exec backend sh -c 'echo ${BACKEND_IMAGE:-not set}' 2>/dev/null || echo 'not accessible')"
    echo ""
fi

if docker ps --format "{{.Names}}" | grep -q "^frontend$"; then
    echo "Frontend container:"
    echo "  FRONTEND_IMAGE_TAG: $(docker exec frontend sh -c 'echo ${FRONTEND_IMAGE_TAG:-not set}' 2>/dev/null || echo 'not accessible')"
    echo "  FRONTEND_IMAGE: $(docker exec frontend sh -c 'echo ${FRONTEND_IMAGE:-not set}' 2>/dev/null || echo 'not accessible')"
    echo ""
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Version API Response"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

BACKEND_PORT="${BACKEND_PORT:-50002}"
if curl -s -f "http://localhost:${BACKEND_PORT}/api/version" > /dev/null 2>&1; then
    curl -s "http://localhost:${BACKEND_PORT}/api/version" | python3 -m json.tool 2>/dev/null || curl -s "http://localhost:${BACKEND_PORT}/api/version"
else
    echo "❌ Cannot reach backend at http://localhost:${BACKEND_PORT}/api/version"
fi
