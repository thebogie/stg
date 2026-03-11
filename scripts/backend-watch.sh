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

exec cargo watch -w back/api -w shared -x "run -p backend"
