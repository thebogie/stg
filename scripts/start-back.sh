#!/bin/bash
# Start Docker backend (SurrealDB + Redis + backend). Run in terminal 1.
# Usage: ./scripts/start-back.sh [dev|prod] [--no-build]
#        ./scripts/start-back.sh --no-build    # start only, no image rebuild (uses dev)
# Default: dev. Use --no-build to start existing images without rebuilding.
#
# Data import (dev only): SurrealDB is reset when backup zip exists; smacktalk.surql is removed,
# regenerated from the zip, then imported. Never use a cached .surql. Never runs in production.

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

NO_BUILD=""
ENV_ARG="${ENV:-dev}"
for a in "$@"; do
  if [ "$a" = "--no-build" ]; then NO_BUILD=1; elif [ "$a" = "dev" ] || [ "$a" = "prod" ]; then ENV_ARG="$a"; fi
done
source "$SCRIPT_DIR/load-env.sh" "$ENV_ARG"

export COMPOSE_PROJECT_NAME=stg
COMPOSE_FILE="$ROOT/deploy/docker-compose.yml"
ENV_FILE="$ROOT/config/.env.${RUST_ENV}"

# VOLUME_PATH is absolute after load-env.sh (defaults: data/dev, data/prod).
mkdir -p "$VOLUME_PATH/surrealdb_data" "$VOLUME_PATH/redis_data" "$VOLUME_PATH/backend_data"
chmod 777 "$VOLUME_PATH/surrealdb_data" 2>/dev/null || true

stg_compose() {
  docker compose --project-directory "$ROOT" "$@"
}

# Tear down and remove network so next up gets a fresh stg (backend can resolve surrealdb/redis)
stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down 2>/dev/null || true
docker network rm stg 2>/dev/null || true

# Dev only: if we have .surql or backup zip, wipe SurrealDB so import gets a clean snapshot
if [ "$RUST_ENV" = "dev" ] || [ "$RUST_ENV" = "development" ]; then
  SURQL_PATH="${SURREAL_IMPORT_SURQL:-$ROOT/_build/smacktalk.surql}"
  BACKUP_ZIP="${ARANGO_BACKUP_ZIP:-$HOME/work/_backups/smacktalk.zip}"
  if [ -f "$BACKUP_ZIP" ]; then
    echo "==> Dev: resetting SurrealDB for clean import..."
    # Wipe volume via container (files are root-owned from SurrealDB container)
    docker run --rm -v "$VOLUME_PATH/surrealdb_data:/data" busybox:1.36 sh -c "rm -rf /data/*"
    chmod 777 "$VOLUME_PATH/surrealdb_data" 2>/dev/null || true
  fi
fi

if [ -n "$NO_BUILD" ]; then
  echo "==> Starting backend stack (no build)..."
  stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d
else
  echo "==> Starting backend stack (SurrealDB, Redis, backend)..."
  stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d --build
fi

# Optional: import Arango→Surreal data (dev only; never in production)
if [ "$RUST_ENV" = "dev" ] || [ "$RUST_ENV" = "development" ]; then
  SURQL_PATH="${SURREAL_IMPORT_SURQL:-$ROOT/_build/smacktalk.surql}"
  BACKUP_ZIP="${ARANGO_BACKUP_ZIP:-$HOME/work/_backups/smacktalk.zip}"
  SURREAL_NS="${SURREAL_NS:-stg_rd}"
  SURREAL_DB="${SURREAL_DB:-stg_rd}"
  SURREAL_USER="${SURREAL_USER:-root}"
  SURREAL_PASSWORD="${SURREAL_PASSWORD:-root}"

  do_import() {
  SURQL_DIR="$(cd "$(dirname "$SURQL_PATH")" && pwd)"
  SURQL_FILE="$(basename "$SURQL_PATH")"
    if docker run --rm --network stg \
      -v "$SURQL_DIR:/import:ro" \
      surrealdb/surrealdb:v3 \
      import \
      --endpoint "http://surrealdb:8000" \
      --username "$SURREAL_USER" --password "$SURREAL_PASSWORD" \
      --namespace "$SURREAL_NS" --database "$SURREAL_DB" \
      "/import/$SURQL_FILE"; then
    echo "==> SurrealDB import completed successfully."
  else
    echo "==> SurrealDB import failed (see above)." >&2
  fi
}

wait_surrealdb() {
  for i in 1 2 3 4 5 6 7 8 9 10; do
    if wget -q -O- --tries=1 "http://127.0.0.1:${SURREALDB_PORT}/health" >/dev/null 2>&1; then return 0; fi
    sleep 2
  done
  echo "Warning: SurrealDB not ready; skipping import."
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
fi

echo "==> Backend: http://127.0.0.1:${BACKEND_PORT} (SurrealDB: ${SURREALDB_PORT}, Redis: ${REDIS_PORT})"
echo "==> Run ./scripts/start-front.sh in another terminal for the frontend."
