#!/bin/bash
# Load .env.dev or .env.prod. Used by start-front.sh (dev), start-back.sh and ci.sh (prod).
# Usage: source scripts/load-env.sh [dev|prod]
#        or set RUST_ENV=dev|prod and source scripts/load-env.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
ENV="${1:-${RUST_ENV:-dev}}"
case "$ENV" in
  dev|development)  ENV_FILE="$ROOT/config/.env.dev"  ;;
  prod|production) ENV_FILE="$ROOT/config/.env.prod" ;;
  *)
    echo "Error: Unknown environment '$ENV'. Use dev or prod." >&2
    return 1 2>/dev/null || exit 1
    ;;
esac
if [ ! -f "$ENV_FILE" ]; then
  echo "Error: $ENV_FILE not found. Run: ./config/setup-env.sh ${ENV}" >&2
  return 1 2>/dev/null || exit 1
fi
# Defaults first so ${VAR} in .env file expand
export BACKEND_PORT="${BACKEND_PORT:-50002}"
export FRONTEND_PORT="${FRONTEND_PORT:-50003}"
export SURREALDB_PORT="${SURREALDB_PORT:-50001}"
export REDIS_PORT="${REDIS_PORT:-6379}"
export VOLUME_PATH="${VOLUME_PATH:-$ROOT/docker-data}"
set -a
# shellcheck source=/dev/null
source "$ENV_FILE"
set +a
export RUST_ENV="$ENV"
# Re-apply defaults so env file cannot unset
export BACKEND_PORT="${BACKEND_PORT:-50002}"
export FRONTEND_PORT="${FRONTEND_PORT:-50003}"
export SURREALDB_PORT="${SURREALDB_PORT:-50001}"
export REDIS_PORT="${REDIS_PORT:-6379}"
export VOLUME_PATH="${VOLUME_PATH:-$ROOT/docker-data}"
