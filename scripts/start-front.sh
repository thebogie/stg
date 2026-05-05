#!/bin/bash
# Start standalone frontend (Yew/Trunk). Run in terminal 2; start backend first with ./scripts/start-back.sh
# Usage: ./scripts/start-front.sh [dev|prod]   or   ENV=prod ./scripts/start-front.sh
# Default: dev.

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
FRONTEND_DIR="$ROOT/front/web"

ENV_ARG="${1:-${ENV:-dev}}"
source "$SCRIPT_DIR/load-env.sh" "$ENV_ARG"

if [ ! -d "$FRONTEND_DIR" ]; then
  echo "Error: Frontend not found: $FRONTEND_DIR"
  exit 1
fi

cd "$FRONTEND_DIR"
[ ! -d node_modules ] && npm install
command -v trunk >/dev/null 2>&1 || cargo install trunk
[ ! -f public/styles.css ] && [ -f node_modules/.bin/tailwindcss ] && ./node_modules/.bin/tailwindcss -i ./src/styles/main.css -o ./public/styles.css --minify 2>/dev/null || true

# Point Trunk proxy at backend root (frontend sends /api/api/... in dev so backend receives /api/...)
TRUNK_TOML="$FRONTEND_DIR/Trunk.toml"
BACKEND_URL="http://localhost:${BACKEND_PORT}"
if [ -f "$TRUNK_TOML" ] && grep -q 'backend = "' "$TRUNK_TOML" 2>/dev/null; then
  sed -i.bak "s|backend = \".*\"|backend = \"$BACKEND_URL\"|" "$TRUNK_TOML" 2>/dev/null || true
  sed -i.bak "s|^port = .*|port = ${FRONTEND_PORT}|" "$TRUNK_TOML" 2>/dev/null || true
fi

# Warn if backend is not reachable (avoids confusing "Connection refused" in browser)
if ! curl -sf --connect-timeout 2 "http://127.0.0.1:${BACKEND_PORT}/health" >/dev/null 2>&1; then
  echo "" >&2
  echo "*** Backend not reachable at http://127.0.0.1:${BACKEND_PORT} ***" >&2
  echo "    Start it first (in another terminal):  ./scripts/start-back.sh" >&2
  echo "    Then refresh the app. Check backend:    curl -s http://127.0.0.1:${BACKEND_PORT}/health" >&2
  echo "" >&2
fi

echo "==> Frontend at http://localhost:${FRONTEND_PORT} (backend: http://localhost:${BACKEND_PORT})"
# Trunk.toml sets [serve].open=true, but that tries to launch the browser via `gio open` which
# fails on minimal/headless Linux setups. Override to keep dev startup reliable.
exec trunk serve --no-default-features --features frontend --open false
