#!/usr/bin/env bash
# Retroactively rename contest titles using the backend Docker image (no Rust on server).
#
# Prerequisites: SurrealDB reachable on host ports; deploy/migrations applied.
#
# Usage (from deploy/ on the server):
#   ./backfill_contest_names.sh              # dry-run
#   ./backfill_contest_names.sh --apply      # write to SurrealDB
#   ./backfill_contest_names.sh --limit 50   # sample
#
# BACKEND_IMAGE is read from /etc/stg/stg.env if present (written by deploy_stg.sh).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_ROOT="${DEPLOY_ROOT:-$SCRIPT_DIR}"
ENV_FILE="${ENV_FILE:-$DEPLOY_ROOT/config/.env.prod}"
STG_ENV_FILE="${STG_ENV_FILE:-/etc/stg/stg.env}"

if [ -f "$STG_ENV_FILE" ]; then
  set -a
  # shellcheck disable=SC1090
  source "$STG_ENV_FILE"
  set +a
fi

BACKEND_IMAGE="${BACKEND_IMAGE:?Set BACKEND_IMAGE or run deploy_stg.sh as root once so ${STG_ENV_FILE} exists}"

if [ ! -f "$ENV_FILE" ]; then
  echo "Missing env file: $ENV_FILE" >&2
  exit 1
fi

set +u
set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a
set -u

SURREALDB_PORT="${SURREALDB_PORT:-50001}"
REDIS_PORT="${REDIS_PORT:-6379}"
SURREAL_URL="${SURREAL_URL:-http://127.0.0.1:${SURREALDB_PORT}}"
REDIS_URL="${REDIS_URL:-redis://127.0.0.1:${REDIS_PORT}/}"

docker run --rm --network host \
  --env-file "$ENV_FILE" \
  -e "SURREAL_URL=${SURREAL_URL}" \
  -e "REDIS_URL=${REDIS_URL}" \
  "${BACKEND_IMAGE}" \
  backfill_contest_names "$@"
