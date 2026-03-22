#!/usr/bin/env bash
# Load boardgames_ranks.csv into Surreal table bgg_catalog using the backend Docker image.
# Use on production hosts that do not have Rust/cargo (same binary as CI builds).
#
# Prerequisites: deploy/migrations applied; CSV on disk; SurrealDB + Redis reachable on host ports.
#
# Usage (from deploy/ on the server):
#   ./import_bgg_catalog.sh /opt/stg/data/bgg/boardgames_ranks.csv [batch_id]
#
# BACKEND_IMAGE is read from /etc/stg/stg.env if present (written by deploy_stg.sh), else must be set.
# Optional: BGG_IMPORT_MAX_ROWS=10000 ./import_bgg_catalog.sh ...

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_ROOT="${DEPLOY_ROOT:-$SCRIPT_DIR}"
ENV_FILE="${ENV_FILE:-$DEPLOY_ROOT/config/.env.prod}"
STG_ENV_FILE="${STG_ENV_FILE:-/etc/stg/stg.env}"

usage() {
  echo "Usage: $0 <path_to_boardgames_ranks.csv> [batch_id]" >&2
  exit 1
}

[ "${1:-}" ] || usage
CSV="$1"
BATCH="${2:-import-$(date -u +%Y%m%dT%H%M%SZ)}"

if [ ! -f "$CSV" ]; then
  echo "CSV not found: $CSV" >&2
  exit 1
fi

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

CSV_ABS="$(cd "$(dirname "$CSV")" && pwd)/$(basename "$CSV")"

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

EXTRA_ENV=()
if [ -n "${BGG_IMPORT_MAX_ROWS:-}" ]; then
  EXTRA_ENV+=( -e "BGG_IMPORT_MAX_ROWS=${BGG_IMPORT_MAX_ROWS}" )
fi

docker run --rm --network host \
  --env-file "$ENV_FILE" \
  -e "SURREAL_URL=${SURREAL_URL}" \
  -e "REDIS_URL=${REDIS_URL}" \
  "${EXTRA_ENV[@]}" \
  -v "${CSV_ABS}:/data/bgg/boardgames_ranks.csv:ro" \
  "${BACKEND_IMAGE}" \
  import_bgg_catalog /data/bgg/boardgames_ranks.csv "${BATCH}"
