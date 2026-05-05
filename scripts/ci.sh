#!/usr/bin/env bash
# Local CI driver: build, unit tests, optional Docker stack, integration / smoke / e2e.
# Usage: ./scripts/ci.sh [build|unit|smoke|integration|e2e|all] [dev|prod]
#        or: ENV=dev ./scripts/ci.sh all
# Default first arg: all. Default env: dev (config/.env.dev vs config/.env.prod).
#
# smoke — prod-like but short: same stack prep as integration, then api_tests + one backend smoke test, then down.

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

# If the user exported VOLUME_PATH explicitly before running this script, respect it.
# Otherwise, use an isolated dir under data/ so CI runs do not clobber dev/prod local volumes.
VOLUME_PATH_PRE="${VOLUME_PATH:-}"

# Second arg or ENV for which env file; default dev
if [ -n "$2" ]; then ENV_ARG="$2"; else ENV_ARG="${ENV:-dev}"; fi
source "$SCRIPT_DIR/load-env.sh" "$ENV_ARG"

export COMPOSE_PROJECT_NAME=stg
COMPOSE_FILE="$ROOT/deploy/docker-compose.yml"
ENV_FILE="$ROOT/config/.env.${RUST_ENV}"

VOL_DEFAULT="$ROOT/data/ci-${RUST_ENV}"
if [ -n "$VOLUME_PATH_PRE" ]; then
  VOL_BASE="$VOLUME_PATH_PRE"
else
  VOL_BASE="$VOL_DEFAULT"
fi
if [[ "$VOL_BASE" != /* ]]; then
  VOL_BASE="$ROOT/${VOL_BASE#./}"
fi
if ! mkdir -p "$VOL_BASE/surrealdb_data" "$VOL_BASE/redis_data" "$VOL_BASE/backend_data" 2>/dev/null; then
  echo "Warning: Cannot create dirs under $VOL_BASE (permission denied?). Using $VOL_DEFAULT"
  VOL_BASE="$VOL_DEFAULT"
  mkdir -p "$VOL_BASE/surrealdb_data" "$VOL_BASE/redis_data" "$VOL_BASE/backend_data"
fi
export VOLUME_PATH="$VOL_BASE"
chmod 777 "$VOLUME_PATH/surrealdb_data" 2>/dev/null || true

stg_compose() {
  docker compose --project-directory "$ROOT" "$@"
}

run_build() {
  echo "==> Build"
  cargo build -p backend
  cargo build -p frontend 2>/dev/null || true
}

run_unit() {
  echo "==> Unit tests (backend)"
  cargo test -p backend --no-fail-fast
}

compose_up_stack() {
  if stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d --build; then
    return 0
  fi
  echo "==> docker compose up failed; attempting cleanup and retry..."
  stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down --remove-orphans || true
  stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d --build
}

wait_for_backend_health() {
  echo "==> Waiting for backend /health..."
  for i in $(seq 1 30); do
    if curl -sf "http://127.0.0.1:${BACKEND_PORT}/health" >/dev/null 2>&1; then
      return 0
    fi
    if [ "$i" -eq 30 ]; then
      echo "Timeout"
      stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs backend || true
      return 1
    fi
    sleep 2
  done
}

wait_for_surreal_health() {
  echo "==> Waiting for SurrealDB /health..."
  for i in $(seq 1 30); do
    if curl -sf "http://127.0.0.1:${SURREALDB_PORT}/health" >/dev/null 2>&1; then
      return 0
    fi
    if [ "$i" -eq 30 ]; then
      echo "Timeout"
      stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs surrealdb || true
      return 1
    fi
    sleep 2
  done
}

wait_for_redis_ping() {
  echo "==> Waiting for Redis ping..."
  for i in $(seq 1 30); do
    if docker exec stg-redis redis-cli ping >/dev/null 2>&1; then
      return 0
    fi
    if [ "$i" -eq 30 ]; then
      echo "Timeout"
      stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs redis || true
      return 1
    fi
    sleep 2
  done
}

restore_prod_surreal_snapshot_if_present() {
  local snapshot_dir="${PROD_DB_SNAPSHOT_DIR:-/home/thebogie/work/stg-data/prod-db}"
  if [ "${RUST_ENV}" != "prod" ]; then
    return 0
  fi
  if [ ! -d "$snapshot_dir" ]; then
    echo "==> Prod snapshot dir not found at $snapshot_dir; skipping seed import."
    return 0
  fi
  if ! ls "$snapshot_dir"/*.surql.gz >/dev/null 2>&1; then
    echo "==> No *.surql.gz in $snapshot_dir; skipping seed import."
    return 0
  fi

  echo "==> Seeding SurrealDB from prod snapshot in $snapshot_dir"

  local tmpdir
  tmpdir="$(mktemp -d)"
  chmod 755 "$tmpdir" 2>/dev/null || true
  trap 'rm -rf "$tmpdir"' RETURN

  for f in $(ls -1 "$snapshot_dir"/*.surql.gz | sort); do
    local base out
    base="$(basename "$f")"
    out="$tmpdir/${base%.gz}"
    echo "==> Decompressing snapshot: $base"
    gunzip -c "$f" | sed \
      -e "s/^INSERT \\[/UPSERT [/" \
      -e "s/DEFINE INDEX bgg_catalog_bgg_id ON bgg_catalog FIELDS bgg_id UNIQUE;/DEFINE INDEX bgg_catalog_bgg_id ON bgg_catalog FIELDS bgg_id;/" \
      > "$out"
    chmod a+r "$out" 2>/dev/null || true
    echo "==> Importing snapshot: ${base%.gz}"
    docker run --rm --network host \
      -v "$tmpdir:/import:ro" \
      surrealdb/surrealdb:v3 \
      import \
      --endpoint "http://127.0.0.1:${SURREALDB_PORT}" \
      --username "${SURREAL_USER:-root}" --password "${SURREAL_PASSWORD:-root}" \
      --namespace "${SURREAL_NS:-stg_rd}" --database "${SURREAL_DB:-stg_rd}" \
      "/import/$(basename "$out")"
  done
}

# Shared Docker prep for integration and smoke (stack left up after — caller may down).
integration_stack_up() {
  echo "==> Preparing stack for integration-style tests"
  stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down --remove-orphans || true

  if [ "${RUST_ENV}" = "prod" ] && [ -d "${PROD_DB_SNAPSHOT_DIR:-/home/thebogie/work/stg-data/prod-db}" ]; then
    echo "==> Clearing local data dirs for prod snapshot seed"
    rm -rf "$VOLUME_PATH/surrealdb_data"/* "$VOLUME_PATH/redis_data"/* "$VOLUME_PATH/backend_data"/* 2>/dev/null || true
  fi

  echo "==> Starting deps (SurrealDB, Redis)..."
  stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d surrealdb redis wait-for-surrealdb
  wait_for_surreal_health
  wait_for_redis_ping

  restore_prod_surreal_snapshot_if_present

  echo "==> Applying SurrealDB migrations"
  DEPLOY_ROOT="$ROOT/deploy" \
    ENV_FILE="$ENV_FILE" \
    MIGRATIONS_DIR="$ROOT/deploy/migrations" \
    "$ROOT/deploy/run_surreal_migrations.sh"

  echo "==> Starting backend..."
  stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d backend
  wait_for_backend_health
}

integration_export_urls_for_host_tests() {
  export SURREAL_URL="http://127.0.0.1:${SURREALDB_PORT}"
  export REDIS_URL="redis://127.0.0.1:${REDIS_PORT}/"
  export SURREAL_NS="${SURREAL_NS:-stg_rd}"
  export SURREAL_DB="${SURREAL_DB:-stg_rd}"
  export SURREAL_USER="${SURREAL_USER:-root}"
  export SURREAL_PASSWORD="${SURREAL_PASSWORD:-root}"
}

run_integration() {
  echo "==> Integration tests (full testing crate; stack stays up for debugging)"
  integration_stack_up
  integration_export_urls_for_host_tests
  cargo test -p testing --no-fail-fast

  echo "==> Backend API smoke tests (db_search_integration_test)"
  BACKEND_URL="http://127.0.0.1:${BACKEND_PORT}" \
    cargo test -p backend --test db_search_integration_test -- --ignored
}

run_smoke() {
  echo "==> Prod-like smoke (shorter than full integration)"
  run_build
  run_unit
  integration_stack_up
  integration_export_urls_for_host_tests
  cargo test -p testing --test api_tests --no-fail-fast
  BACKEND_URL="http://127.0.0.1:${BACKEND_PORT}" \
    cargo test -p backend --test db_search_integration_test -- --ignored
  echo "==> Bringing stack down after smoke"
  stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down --remove-orphans || true
}

run_e2e() {
  echo "==> E2E (stack up, smoke, then down)"
  compose_up_stack
  wait_for_backend_health
  echo "Backend healthy."
  curl -sf "http://127.0.0.1:${BACKEND_PORT}/health" && echo " OK"
  stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down
}

case "${1:-all}" in
  build)         run_build ;;
  unit)          run_unit ;;
  smoke)         run_smoke ;;
  integration)   run_integration ;;
  e2e)           run_e2e ;;
  all)
    run_build
    run_unit
    run_integration
    run_e2e
    ;;
  *)
    echo "Usage: $0 [build|unit|smoke|integration|e2e|all] [dev|prod]"
    echo "       Or set ENV=dev or ENV=prod to choose config/.env.dev or config/.env.prod"
    exit 1
    ;;
esac
echo "==> Done"
