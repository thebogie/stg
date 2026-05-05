#!/usr/bin/env bash
# Dev stack for local debugging (VS Code / lldb breakpoints on the Rust backend).
# Starts SurrealDB + Redis in Docker only; run the backend on the host (not in a container).
#
# Usage: ./scripts/dev-debug.sh [dev]
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

ENV_ARG="${1:-dev}"
"$SCRIPT_DIR/start-deps.sh" "$ENV_ARG"
# shellcheck source=/dev/null
source "$SCRIPT_DIR/load-env.sh" "$ENV_ARG"

echo ""
echo "==> Deps are up. Run the backend on the host (breakpoints work here):"
echo "      just backend-watch"
echo "    or:"
echo "      source scripts/load-env.sh $ENV_ARG && cargo run -p backend"
echo ""
echo "==> Frontend (separate terminal):"
echo "      ./scripts/start-front.sh"
echo "    or:"
echo "      ./scripts/start-tauri.sh"
echo ""
echo "==> URLs:  Surreal  http://127.0.0.1:${SURREALDB_PORT}   Backend  http://127.0.0.1:${BACKEND_PORT}"
echo "    VS Code: Run and Debug → add a Rust/CodeLLDB launch for the \`backend\` package binary if you want F5."
