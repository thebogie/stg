#!/bin/bash
# Start dependency containers (no backend). Use with local backend for quick iteration.
# Default: SurrealDB + Redis + Ollama.
# When PLAYWRIGHT_MODE=queue in config/.env.*, also starts playwright-worker (Sell → BGG).
# Usage: ./scripts/start-deps.sh [dev] [--no-build]
#        Then run: just backend-watch (terminal 2) and ./scripts/start-front.sh (terminal 3)
#
# Data import (dev): same as start-back.sh — if backup zip exists, smacktalk.surql is removed,
# regenerated from the zip, then imported. Never use a cached .surql; never runs in production.
# Requires: config/.env.dev (./config/setup-env.sh dev)

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

NO_BUILD=""
ENV_ARG="dev"
for a in "$@"; do
  if [ "$a" = "--no-build" ]; then
    NO_BUILD=1
  elif [ "$a" = "dev" ] || [ "$a" = "prod" ]; then
    ENV_ARG="$a"
  fi
done
source "$SCRIPT_DIR/load-env.sh" "$ENV_ARG"

export COMPOSE_PROJECT_NAME=stg
COMPOSE_FILE="$ROOT/deploy/docker-compose.yml"
ENV_FILE="$ROOT/config/.env.${RUST_ENV}"

mkdir -p "$VOLUME_PATH/surrealdb_data" "$VOLUME_PATH/redis_data"
chmod 777 "$VOLUME_PATH/surrealdb_data" 2>/dev/null || true

PLAYWRIGHT_MODE="${PLAYWRIGHT_MODE:-local}"
DEPS_SERVICES=(surrealdb redis ollama)
if [ "${PLAYWRIGHT_MODE,,}" = "queue" ]; then
  mkdir -p "$VOLUME_PATH/backend_data/sell-images" "$VOLUME_PATH/playwright_jobs"
  DEPS_SERVICES+=(playwright-worker)
fi

stg_compose() {
  docker compose -p stg --project-directory "$ROOT/deploy" "$@"
}

# Dev only: optional wipe + import (same logic as start-back.sh)
WIPED=""
if [ "$RUST_ENV" = "dev" ] || [ "$RUST_ENV" = "development" ]; then
  SURQL_PATH="${SURREAL_IMPORT_SURQL:-$ROOT/_build/smacktalk.surql}"
  BACKUP_ZIP="${ARANGO_BACKUP_ZIP:-$HOME/work/_backups/smacktalk.zip}"
  SEED_DIR_EARLY="${SURREAL_SEED_DIR:-}"
  SEED_FORCE="${SURREAL_SEED_FORCE:-0}"
  if [ "$SEED_FORCE" = "1" ] && [ -n "$SEED_DIR_EARLY" ] && [ -d "$SEED_DIR_EARLY" ]; then
    echo "==> Dev: SURREAL_SEED_FORCE=1 — wiping SurrealDB volume for reseed..."
    docker run --rm -v "$VOLUME_PATH/surrealdb_data:/data" busybox:1.36 sh -c "rm -rf /data/*"
    chmod 777 "$VOLUME_PATH/surrealdb_data" 2>/dev/null || true
    WIPED=1
  fi
  if [ -f "$BACKUP_ZIP" ]; then
    echo "==> Dev: resetting SurrealDB for clean import..."
    docker run --rm -v "$VOLUME_PATH/surrealdb_data:/data" busybox:1.36 sh -c "rm -rf /data/*"
    chmod 777 "$VOLUME_PATH/surrealdb_data" 2>/dev/null || true
    WIPED=1
  fi
fi

# Start deps (no backend container)
if [ "${PLAYWRIGHT_MODE,,}" = "queue" ]; then
  echo "==> Starting SurrealDB, Redis, Ollama, and Playwright worker (PLAYWRIGHT_MODE=queue)..."
else
  echo "==> Starting SurrealDB, Redis, and Ollama..."
fi
if [ -n "$NO_BUILD" ]; then
  stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d "${DEPS_SERVICES[@]}"
else
  stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d --build "${DEPS_SERVICES[@]}"
fi

# If we wiped the volume, restart SurrealDB so it sees the empty data dir (otherwise it keeps old in-memory schema)
if [ -n "$WIPED" ]; then
  echo "==> Restarting SurrealDB to pick up empty volume..."
  stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" restart surrealdb
fi

# Optional import (dev only)
if [ "$RUST_ENV" = "dev" ] || [ "$RUST_ENV" = "development" ]; then
  SURQL_PATH="${SURREAL_IMPORT_SURQL:-$ROOT/_build/smacktalk.surql}"
  BACKUP_ZIP="${ARANGO_BACKUP_ZIP:-$HOME/work/_backups/smacktalk.zip}"
  SEED_DIR="${SURREAL_SEED_DIR:-}"
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

  try_seed_import() {
    # Seed format: directory with *.surql or *.surql.gz exports (e.g. stg_rd.surql.gz, system.surql.gz)
    if [ -z "$SEED_DIR" ]; then
      return 0
    fi
    if [ ! -d "$SEED_DIR" ]; then
      return 0
    fi
    # Only seed when Surreal volume dir looks empty OR we are explicitly forcing a reseed.
    # Note: after (re)starting SurrealDB, it creates a rocksdb directory immediately, so the volume
    # won't be "empty" even though it contains no user data yet.
    if [ "${SURREAL_SEED_FORCE:-0}" != "1" ] && [ -n "$(ls -A "$VOLUME_PATH/surrealdb_data" 2>/dev/null)" ]; then
      return 0
    fi

    # Import any seed files found. Surreal CLI supports reading compressed files in many builds; if it doesn't,
    # the import will fail and we'll log it (then user can provide an uncompressed .surql).
    local any=0
    for f in "$SEED_DIR"/*.surql "$SEED_DIR"/*.surql.gz; do
      [ -f "$f" ] || continue
      any=1
      local seed_dir_abs seed_file tmp_dir tmp_file to_import_dir to_import_file
      seed_dir_abs="$(cd "$(dirname "$f")" && pwd)"
      seed_file="$(basename "$f")"

      # Surreal's import expects plain-text .surql. Our seed is usually gzipped (.surql.gz),
      # so decompress to a temp file first.
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

      echo "==> Seeding SurrealDB from $to_import_file ..."
      if docker run --rm --network host \
        -v "$to_import_dir:/import:ro" \
        surrealdb/surrealdb:v3 \
        import \
        --endpoint "$SURREAL_ENDPOINT" \
        --username "$SURREAL_USER" --password "$SURREAL_PASSWORD" \
        --namespace "$SURREAL_NS" --database "$SURREAL_DB" \
        "/import/$to_import_file"; then
        echo "==> Seed import ok: $to_import_file"
      else
        echo "==> Warning: seed import failed for $to_import_file" >&2
      fi
    done
    if [ "$any" = "0" ]; then
      echo "==> Warning: no seed files found in $SEED_DIR" >&2
    fi
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

  # Seed import (optional): if SURREAL_SEED_DIR is set and the Surreal volume is empty, import seed dumps.
  # This runs after the optional Arango→Surreal conversion/import block above.
  ( wait_surrealdb && try_seed_import ) || true

  if [ -x "$ROOT/deploy/run_surreal_migrations.sh" ]; then
    ( wait_surrealdb && DEPLOY_ROOT="$ROOT/deploy" ENV_FILE="$ENV_FILE" "$ROOT/deploy/run_surreal_migrations.sh" ) || true
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
if [ "${PLAYWRIGHT_MODE,,}" = "queue" ]; then
  echo "==> Playwright worker: docker logs -f stg-playwright-worker"
fi
echo "==> Next: run backend and frontend (2 terminals):"
echo "      Terminal 2:  just backend-watch"
echo "      Terminal 3:  ./scripts/start-front.sh"
echo ""
