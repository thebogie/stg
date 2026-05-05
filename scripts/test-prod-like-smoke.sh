#!/usr/bin/env bash
# Faster prod-like check: build + unit + Surreal/Redis/backend in Docker + short integration smoke, then stack down.
# Default env: prod (config/.env.prod). Override: ./scripts/test-prod-like-smoke.sh dev
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENV_ARG="${1:-prod}"
exec "$SCRIPT_DIR/ci.sh" smoke "$ENV_ARG"
