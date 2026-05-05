#!/bin/bash
# Stop the Docker backend stack (SurrealDB, Redis, backend). No rebuild.
# Usage: ./scripts/stop-back.sh [dev|prod]   or   ENV=prod ./scripts/stop-back.sh
# Default: dev (must match the env you used for start-back.sh to find the same project).

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

ENV_ARG="${1:-${ENV:-dev}}"
source "$SCRIPT_DIR/load-env.sh" "$ENV_ARG"

export COMPOSE_PROJECT_NAME=stg
COMPOSE_FILE="$ROOT/deploy/docker-compose.yml"
ENV_FILE="$ROOT/config/.env.${RUST_ENV}"

echo "==> Stopping backend stack..."
docker compose --project-directory "$ROOT" -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down
