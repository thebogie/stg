#!/bin/bash
# Quick check that /api/version returns JSON (run while stack is up).
# Usage: ./scripts/check-version-api.sh

set -e
BACKEND_PORT="${BACKEND_PORT:-50002}"
FRONTEND_PORT="${FRONTEND_PORT:-50003}"

echo "=== Direct to backend (port $BACKEND_PORT) ==="
curl -s -o /tmp/version_backend.json -w "HTTP %{http_code}\n" "http://localhost:${BACKEND_PORT}/api/version"
echo "Body:"
cat /tmp/version_backend.json | head -c 500
echo ""
echo ""

echo "=== Via frontend/nginx (port $FRONTEND_PORT) ==="
curl -s -o /tmp/version_frontend.json -w "HTTP %{http_code}\n" "http://localhost:${FRONTEND_PORT}/api/version"
echo "Body:"
cat /tmp/version_frontend.json | head -c 500
echo ""

if ! grep -q '"version"' /tmp/version_frontend.json 2>/dev/null; then
  echo "WARNING: Response from frontend port does not look like version JSON. Check nginx proxy."
  exit 1
fi
echo "OK: Version API returns JSON."
