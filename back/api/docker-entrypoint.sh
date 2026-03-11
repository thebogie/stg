#!/bin/sh
# If SURREAL_URL/REDIS_URL are not set, use default gateway IP so backend reaches
# host-published ports. Works on Docker Desktop (WSL2) and production Ubuntu.
# Override by setting SURREAL_URL and REDIS_URL in compose (e.g. ws://surrealdb:8000 on Linux).
set -e
if [ -z "${SURREAL_URL}" ] || [ -z "${REDIS_URL}" ]; then
  HOST_IP=$(ip route show default 2>/dev/null | awk '/default/ {print $3}' | head -1)
  if [ -n "$HOST_IP" ]; then
    export SURREAL_URL="${SURREAL_URL:-ws://${HOST_IP}:${SURREALDB_PORT:-50001}}"
    export REDIS_URL="${REDIS_URL:-redis://${HOST_IP}:${REDIS_PORT:-6379}/}"
  fi
fi
exec /usr/local/bin/backend "$@"
