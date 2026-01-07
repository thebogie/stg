#!/bin/bash

# Check production environment variables
# Usage: ./scripts/check-env-prod.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

ENV_FILE="${PROJECT_ROOT}/config/.env.production"

if [ ! -f "$ENV_FILE" ]; then
    echo "❌ Environment file not found: $ENV_FILE"
    exit 1
fi

echo "📋 Checking environment variables from: $ENV_FILE"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Load the env file
set -a
source "$ENV_FILE"
set +a

echo "ArangoDB Configuration:"
echo "  ARANGO_URL: ${ARANGO_URL:-'NOT SET'}"
echo "  ARANGO_DB: ${ARANGO_DB:-'NOT SET'}"
echo "  ARANGO_USERNAME: ${ARANGO_USERNAME:-'NOT SET'}"
echo "  ARANGO_ROOT_PASSWORD: ${ARANGO_ROOT_PASSWORD:+SET (hidden)}"
echo "  ARANGODB_INTERNAL_PORT: ${ARANGODB_INTERNAL_PORT:-'NOT SET'}"
echo ""

echo "Redis Configuration:"
echo "  REDIS_URL: ${REDIS_URL:-'NOT SET'}"
echo ""

echo "Backend Configuration:"
echo "  SERVER_HOST: ${SERVER_HOST:-'NOT SET'}"
echo "  SERVER_PORT: ${SERVER_PORT:-'NOT SET'}"
echo "  BACKEND_URL: ${BACKEND_URL:-'NOT SET'}"
echo ""

echo "Network Check:"
echo "  Backend container ARANGO_URL (what backend sees):"
docker exec backend env | grep ARANGO_URL || echo "  ❌ Backend container not running or ARANGO_URL not set"
echo ""

echo "ArangoDB Connectivity Test:"
echo "  Testing from backend container to ArangoDB..."
docker exec backend curl -f -s http://arangodb:8529/_api/version || echo "  ❌ Cannot connect to ArangoDB"
echo ""

