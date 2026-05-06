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
ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
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
  # docker-compose.yml lives under deploy/. Use that as project dir so build contexts like `..`
  # resolve to repo root (deploy/..), not to a parent of the repo root.
  docker compose --project-directory "$ROOT/deploy" "$@"
}

# Optional seed import: set SURREAL_SEED_DIR to a dir containing *.surql or *.surql.gz exports.
try_seed_import() {
  local seed_dir="${SURREAL_SEED_DIR:-}"
  if [ -z "$seed_dir" ] || [ ! -d "$seed_dir" ]; then
    return 0
  fi
  # only seed when Surreal volume is empty OR we are explicitly forcing a reseed
  if [ "${SURREAL_SEED_FORCE:-0}" != "1" ] && [ -n "$(ls -A "$VOLUME_PATH/surrealdb_data" 2>/dev/null)" ]; then
    return 0
  fi
  local surreal_endpoint="http://127.0.0.1:${SURREALDB_PORT:-50001}"
  local surreal_ns="${SURREAL_NS:-stg_rd}"
  local surreal_db="${SURREAL_DB:-stg_rd}"
  local surreal_user="${SURREAL_USER:-root}"
  local surreal_password="${SURREAL_PASSWORD:-root}"

  echo "==> Seeding SurrealDB from $seed_dir (volume empty)..."
  for f in "$seed_dir"/*.surql "$seed_dir"/*.surql.gz; do
    [ -f "$f" ] || continue
    local seed_dir_abs seed_file tmp_dir tmp_file to_import_dir to_import_file
    seed_dir_abs="$(cd "$(dirname "$f")" && pwd)"
    seed_file="$(basename "$f")"
    to_import_dir="$seed_dir_abs"
    to_import_file="$seed_file"
    if [[ "$seed_file" == *.gz ]]; then
      tmp_dir="$ROOT/_build/surreal-seed"
      mkdir -p "$tmp_dir"
      tmp_file="${tmp_dir}/${seed_file%.gz}"
      echo "==> Decompressing seed $seed_file -> $(basename "$tmp_file") ..."
      if command -v gzip >/dev/null 2>&1; then
        gzip -dc "$f" > "$tmp_file"
      else
        python3 - "$f" "$tmp_file" <<'PY'
import gzip, sys
src, dst = sys.argv[1], sys.argv[2]
with gzip.open(src, 'rb') as fin, open(dst, 'wb') as fout:
    fout.write(fin.read())
PY
      fi
      to_import_dir="$tmp_dir"
      to_import_file="$(basename "$tmp_file")"
    fi

    echo "==> Import seed $to_import_file ..."
    docker run --rm --network host \
      -v "$to_import_dir:/import:ro" \
      surrealdb/surrealdb:v3 \
      import \
      --endpoint "$surreal_endpoint" \
      --username "$surreal_user" --password "$surreal_password" \
      --namespace "$surreal_ns" --database "$surreal_db" \
      "/import/$to_import_file" || true
  done
}

# Tear down and remove network so next up gets a fresh stg (backend can resolve surrealdb/redis)
stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down 2>/dev/null || true
docker network rm stg 2>/dev/null || true

# Dev only: if we have .surql or backup zip, wipe SurrealDB so import gets a clean snapshot
if [ "$RUST_ENV" = "dev" ] || [ "$RUST_ENV" = "development" ]; then
  SURQL_PATH="${SURREAL_IMPORT_SURQL:-$ROOT/_build/smacktalk.surql}"
  BACKUP_ZIP="${ARANGO_BACKUP_ZIP:-$HOME/work/_backups/smacktalk.zip}"
  SEED_DIR_EARLY="${SURREAL_SEED_DIR:-}"
  SEED_FORCE="${SURREAL_SEED_FORCE:-0}"
  if [ "$SEED_FORCE" = "1" ] && [ -n "$SEED_DIR_EARLY" ] && [ -d "$SEED_DIR_EARLY" ]; then
    echo "==> Dev: SURREAL_SEED_FORCE=1 — wiping SurrealDB volume for reseed..."
    docker run --rm -v "$VOLUME_PATH/surrealdb_data:/data" busybox:1.36 sh -c "rm -rf /data/*"
    chmod 777 "$VOLUME_PATH/surrealdb_data" 2>/dev/null || true
  fi
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

  # Optional seed import (only if volume is empty and SURREAL_SEED_DIR is set)
  ( wait_surrealdb && try_seed_import ) || true
fi

echo "==> Backend: http://127.0.0.1:${BACKEND_PORT} (SurrealDB: ${SURREALDB_PORT}, Redis: ${REDIS_PORT})"
echo "==> Run ./scripts/start-front.sh in another terminal for the frontend."
