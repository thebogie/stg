#!/bin/bash
# Load .env.dev or .env.prod. Used by start-front.sh (dev), start-back.sh and ci.sh (prod).
# Usage: source scripts/load-env.sh [dev|prod]
#        or set RUST_ENV=dev|prod and source scripts/load-env.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
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
# Default VOLUME_PATH: all local Docker bind mounts live under repo-root data/ (gitignored).
# dev vs prod local stacks use different subdirs so switching RUST_ENV does not clobber the other DB.
# Production servers: set VOLUME_PATH in .env.prod to an absolute path outside the repo (see deploy/README.md).
case "$ENV" in
  dev|development) VOLUME_PATH_DEFAULT="$ROOT/data/dev" ;;
  *)               VOLUME_PATH_DEFAULT="$ROOT/data/prod" ;;
esac
# Defaults first so ${VAR} in .env file expand
export BACKEND_PORT="${BACKEND_PORT:-50002}"
export FRONTEND_PORT="${FRONTEND_PORT:-50003}"
export SURREALDB_PORT="${SURREALDB_PORT:-50001}"
export REDIS_PORT="${REDIS_PORT:-6379}"
export VOLUME_PATH="${VOLUME_PATH:-$VOLUME_PATH_DEFAULT}"
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
export VOLUME_PATH="${VOLUME_PATH:-$VOLUME_PATH_DEFAULT}"
# Relative VOLUME_PATH (e.g. ./docker-data) must be repo-root–absolute: Compose files live under
# deploy/, and Docker often resolves bind-mount sources relative to that directory, so wipes under
# $ROOT and Surreal's actual data dir could diverge. Stale RocksDB + new SURREAL_PASSWORD → import
# auth failures while Surreal logs "existing root users were found" (--pass is then ignored).
case "${VOLUME_PATH:-}" in
  /*) ;;
  *) export VOLUME_PATH="$ROOT/${VOLUME_PATH#./}" ;;
esac
