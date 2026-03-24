#!/bin/bash
# Start only SurrealDB + Redis (no backend container). Use with local backend for quick iteration.
# Usage: ./scripts/start-deps.sh [dev] [--no-build]
#        Then run: just backend-watch (terminal 2) and ./scripts/start-tauri.sh (terminal 3)
#
# Data import (dev): same as start-back.sh — if backup zip exists, smacktalk.surql is removed,
# regenerated from the zip, then imported. Never use a cached .surql; never runs in production.
# Requires: config/.env.dev (./config/setup-env.sh dev)

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

ENV_ARG="${1:-dev}"
source "$SCRIPT_DIR/load-env.sh" "$ENV_ARG"

export COMPOSE_PROJECT_NAME=stg
COMPOSE_FILE="$ROOT/deploy/docker-compose.yml"
ENV_FILE="$ROOT/config/.env.${RUST_ENV}"

VOL_DEFAULT="$ROOT/docker-data"
[ "${RUST_ENV:-dev}" = "dev" ] || [ "${RUST_ENV:-dev}" = "development" ] && VOL_DEFAULT="$ROOT/_build/docker-data"
VOL_BASE="${VOLUME_PATH:-$VOL_DEFAULT}"
VOL_BASE="$(cd "$VOL_BASE" 2>/dev/null && pwd)" || VOL_BASE="$VOL_DEFAULT"
mkdir -p "$VOL_BASE/surrealdb_data" "$VOL_BASE/redis_data"
export VOLUME_PATH="$VOL_BASE"
chmod 777 "$VOLUME_PATH/surrealdb_data" 2>/dev/null || true

# Dev only: optional wipe + import (same logic as start-back.sh)
WIPED=""
if [ "$RUST_ENV" = "dev" ] || [ "$RUST_ENV" = "development" ]; then
  SURQL_PATH="${SURREAL_IMPORT_SURQL:-$ROOT/_build/smacktalk.surql}"
  BACKUP_ZIP="${ARANGO_BACKUP_ZIP:-$HOME/work/_backups/smacktalk.zip}"
  if [ -f "$BACKUP_ZIP" ]; then
    echo "==> Dev: resetting SurrealDB for clean import..."
    docker run --rm -v "$VOL_BASE/surrealdb_data:/data" busybox:1.36 sh -c "rm -rf /data/*"
    chmod 777 "$VOL_BASE/surrealdb_data" 2>/dev/null || true
    WIPED=1
  fi
fi

# Start only SurrealDB and Redis (no backend, no full stack down/up)
echo "==> Starting SurrealDB and Redis only..."
docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d surrealdb redis

# If we wiped the volume, restart SurrealDB so it sees the empty data dir (otherwise it keeps old in-memory schema)
if [ -n "$WIPED" ]; then
  echo "==> Restarting SurrealDB to pick up empty volume..."
  docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" restart surrealdb
fi

# Optional import (dev only)
if [ "$RUST_ENV" = "dev" ] || [ "$RUST_ENV" = "development" ]; then
  SURQL_PATH="${SURREAL_IMPORT_SURQL:-$ROOT/_build/smacktalk.surql}"
  BACKUP_ZIP="${ARANGO_BACKUP_ZIP:-$HOME/work/_backups/smacktalk.zip}"
  SURREAL_NS="${SURREAL_NS:-stg_rd}"
  SURREAL_DB="${SURREAL_DB:-stg_rd}"
  SURREAL_USER="${SURREAL_USER:-root}"
  SURREAL_PASSWORD="${SURREAL_PASSWORD:-root}"

  SURREAL_ENDPOINT="http://127.0.0.1:${SURREALDB_PORT:-50001}"

  do_import() {
    SURQL_DIR="$(cd "$(dirname "$SURQL_PATH")" && pwd)"
    SURQL_FILE="$(basename "$SURQL_PATH")"
    if docker run --rm --network host \
      -v "$SURQL_DIR:/import:ro" \
      surrealdb/surrealdb:v3 \
      import \
      --endpoint "$SURREAL_ENDPOINT" \
      --username "$SURREAL_USER" --password "$SURREAL_PASSWORD" \
      --namespace "$SURREAL_NS" --database "$SURREAL_DB" \
      "/import/$SURQL_FILE"; then
      echo "==> SurrealDB import completed successfully."
    else
      echo "==> SurrealDB import failed (see above)." >&2
    fi
  }

  wait_surrealdb() {
    echo "==> Waiting for SurrealDB at $SURREAL_ENDPOINT (up to 60s)..."
    for i in $(seq 1 20); do
      if wget -q -O- --tries=1 "http://127.0.0.1:${SURREALDB_PORT}/health" >/dev/null 2>&1; then
        echo "==> SurrealDB is ready."
        return 0
      fi
      sleep 3
    done
    echo "Warning: SurrealDB not ready after 60s; skipping import." >&2
    return 1
  }

  if [ -f "$BACKUP_ZIP" ]; then
    rm -f "$SURQL_PATH"
    echo "==> Converting $BACKUP_ZIP to .surql (fresh) and importing ..."
    mkdir -p "$(dirname "$SURQL_PATH")"
    if cargo run --manifest-path tools/arango-to-surreal/Cargo.toml -- "$BACKUP_ZIP" -o "$SURQL_PATH"; then
      ( wait_surrealdb && do_import ) || true
    else
      echo "Warning: Conversion failed; skipping import." >&2
    fi
  fi

  # Apply optional SurrealDB functions (contest_row, contest_with_edges, etc.); use host network so we hit 127.0.0.1
  if [ -f "$ROOT/tools/arango-to-surreal/surreal-functions.surql" ]; then
    REMOVE_FILE="$ROOT/tools/arango-to-surreal/surreal-functions-remove.surql"
    if [ -f "$REMOVE_FILE" ]; then
      echo "==> Removing existing SurrealDB functions (if any) for idempotent apply..."
      docker run --rm --network host \
        -v "$ROOT/tools/arango-to-surreal:/import:ro" \
        surrealdb/surrealdb:v3 \
        import \
        --endpoint "$SURREAL_ENDPOINT" \
        --username "$SURREAL_USER" --password "$SURREAL_PASSWORD" \
        --namespace "$SURREAL_NS" --database "$SURREAL_DB" \
        "/import/surreal-functions-remove.surql" 2>/dev/null || true
    fi
    echo "==> Applying SurrealDB functions (tools/arango-to-surreal/surreal-functions.surql)..."
    if docker run --rm --network host \
      -v "$ROOT/tools/arango-to-surreal:/import:ro" \
      surrealdb/surrealdb:v3 \
      import \
      --endpoint "$SURREAL_ENDPOINT" \
      --username "$SURREAL_USER" --password "$SURREAL_PASSWORD" \
      --namespace "$SURREAL_NS" --database "$SURREAL_DB" \
      "/import/surreal-functions.surql"; then
      echo "==> SurrealDB functions applied."
    else
      echo "==> SurrealDB functions failed (run manually once DB is up: ./scripts/apply-surreal-functions.sh)." >&2
    fi
  fi
fi

echo ""
echo "==> SurrealDB: http://127.0.0.1:${SURREALDB_PORT}  |  Redis: 127.0.0.1:${REDIS_PORT}"
echo "==> Next: run backend and frontend (2 terminals):"
echo "      Terminal 2:  just backend-watch"
echo "      Terminal 3:  ./scripts/start-tauri.sh"
echo ""
