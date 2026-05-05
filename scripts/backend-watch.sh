#!/bin/bash
# Run backend with auto-restart on save, using config/.env.dev. No need to source load-env.sh.
# Usage: ./scripts/backend-watch.sh [dev]   or   just backend-watch

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

ENV_ARG="${1:-dev}"
# shellcheck source=/dev/null
source "$SCRIPT_DIR/load-env.sh" "$ENV_ARG"

if cargo watch --version >/dev/null 2>&1; then
  exec cargo watch -w back/api -w shared -x "run -p backend"
fi

echo "cargo-watch is not installed (cargo: no such command: watch)." >&2
echo "Install it with: cargo install cargo-watch" >&2
echo "Falling back to: cargo run -p backend" >&2
exec cargo run -p backend
