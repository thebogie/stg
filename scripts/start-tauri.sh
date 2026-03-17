#!/bin/bash
# Start Tauri desktop app (embeds Yew from front/web). Backend should be running first.
# Usage: ./scripts/start-tauri.sh [dev|prod]   or   ENV=prod ./scripts/start-tauri.sh
# Default: dev.
# Requires: cargo install tauri-cli

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
TAURI_DIR="$ROOT/front/tauri"
WEB_DIR="$ROOT/front/web"

ENV_ARG="${1:-${ENV:-dev}}"
source "$SCRIPT_DIR/load-env.sh" "$ENV_ARG"

if [ ! -d "$TAURI_DIR/src-tauri" ]; then
  echo "Error: Tauri app not found: $TAURI_DIR/src-tauri" >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "Error: cargo not found. Install Rust first." >&2
  exit 1
fi
if ! cargo tauri --version >/dev/null 2>&1; then
  echo "Error: Tauri CLI not found. Install with:  cargo install tauri-cli" >&2
  exit 1
fi

# Trunk (started by Tauri) proxies API to localhost:50002; backend should be there
if ! curl -sf --connect-timeout 2 "http://127.0.0.1:${BACKEND_PORT}/health" >/dev/null 2>&1; then
  echo "" >&2
  echo "*** Backend not reachable at http://127.0.0.1:${BACKEND_PORT} ***" >&2
  echo "    Start it first (in another terminal):  ./scripts/start-back.sh" >&2
  echo "    Then use the app. Check:  curl -s http://127.0.0.1:${BACKEND_PORT}/health" >&2
  echo "" >&2
fi

# Ensure Tailwind output is up to date (disable with SKIP_TAILWIND=1)
if [ "${SKIP_TAILWIND:-0}" != "1" ]; then
  if command -v npm >/dev/null 2>&1; then
    if [ -f "$WEB_DIR/package.json" ]; then
      echo "==> Rebuilding Tailwind CSS (SKIP_TAILWIND=1 to skip)"
      (cd "$WEB_DIR" && npm run -s build:css:prod)
    fi
  else
    echo "==> npm not found; skipping Tailwind build" >&2
  fi
fi

echo "==> Starting Tauri (Yew at http://localhost:${FRONTEND_PORT}; backend ${BACKEND_PORT})"
cd "$TAURI_DIR"
exec cargo tauri dev
