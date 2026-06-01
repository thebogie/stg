#!/bin/sh
# If SURREAL_URL/REDIS_URL are not set, use default gateway IP so backend reaches
# host-published ports. Works on Docker Desktop (WSL2) and production Ubuntu.
# Override by setting SURREAL_URL and REDIS_URL in compose (e.g. ws://surrealdb:8000 on Linux).
set -e

# Bind-mounted /app/data is often root-owned on the host; backend runs as appuser (uid 1000).
prepare_app_data() {
  image_dir="${CONTEST_IMAGE_DIR:-/app/data/contest-images}"
  mkdir -p /app/data "$image_dir" 2>/dev/null || true
  if [ "$(id -u)" = "0" ]; then
    chown -R appuser:appuser /app/data 2>/dev/null || chown -R 1000:1000 /app/data 2>/dev/null || true
  fi
}

run_as_appuser() {
  if [ "$(id -u)" = "0" ]; then
    exec gosu appuser "$@"
  fi
  exec "$@"
}

prepare_app_data

if [ -z "${SURREAL_URL}" ] || [ -z "${REDIS_URL}" ]; then
  HOST_IP=$(ip route show default 2>/dev/null | awk '/default/ {print $3}' | head -1)
  if [ -n "$HOST_IP" ]; then
    export SURREAL_URL="${SURREAL_URL:-ws://${HOST_IP}:${SURREALDB_PORT:-50001}}"
    export REDIS_URL="${REDIS_URL:-redis://${HOST_IP}:${REDIS_PORT:-6379}/}"
  fi
fi
if [ "$#" -ge 1 ] && [ "$1" = "import_bgg_catalog" ]; then
  shift
  run_as_appuser /usr/local/bin/import_bgg_catalog "$@"
fi
if [ "$#" -ge 1 ] && [ "$1" = "backfill_contest_names" ]; then
  shift
  run_as_appuser /usr/local/bin/backfill_contest_names "$@"
fi
run_as_appuser /usr/local/bin/backend "$@"
