#!/bin/bash
# Run full CI locally (build, unit, integration, e2e) using Docker stack.
# Usage: ./scripts/ci.sh [build|unit|integration|e2e|all] [dev|prod]
#        or: ENV=dev ./scripts/ci.sh all
# Default env: dev. First arg is stage, second is dev|prod (or use ENV).

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

# Second arg or ENV for which env file; default dev
if [ -n "$2" ]; then ENV_ARG="$2"; else ENV_ARG="${ENV:-dev}"; fi
source "$SCRIPT_DIR/load-env.sh" "$ENV_ARG"

export COMPOSE_PROJECT_NAME=stg
COMPOSE_FILE="$ROOT/deploy/docker-compose.yml"
ENV_FILE="$ROOT/config/.env.${RUST_ENV}"

# Resolve VOLUME_PATH to absolute; use repo docker-data if requested path is not writable
VOL_BASE="${VOLUME_PATH:-$ROOT/docker-data}"
VOL_BASE="$(cd "$VOL_BASE" 2>/dev/null && pwd)" || VOL_BASE="$ROOT/docker-data"
if ! mkdir -p "$VOL_BASE/surrealdb_data" "$VOL_BASE/redis_data" "$VOL_BASE/backend_data" 2>/dev/null; then
  echo "Warning: Cannot create dirs under $VOL_BASE (permission denied?). Using $ROOT/docker-data"
  VOL_BASE="$ROOT/docker-data"
  mkdir -p "$VOL_BASE/surrealdb_data" "$VOL_BASE/redis_data" "$VOL_BASE/backend_data"
fi
export VOLUME_PATH="$VOL_BASE"
chmod 777 "$VOLUME_PATH/surrealdb_data" 2>/dev/null || true

run_build() {
  echo "==> Build"
  cargo build -p backend
  cargo build -p frontend 2>/dev/null || true
}

run_unit() {
  echo "==> Unit tests (backend)"
  cargo test -p backend --no-fail-fast
}

run_integration() {
  echo "==> Integration tests (stack must be up)"
  if ! docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" ps backend 2>/dev/null | grep -q Up; then
    echo "==> Starting stack (SurrealDB, Redis, backend)..."
    docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d --build
    echo "==> Waiting for backend /health..."
    for i in $(seq 1 30); do
      if curl -sf "http://127.0.0.1:${BACKEND_PORT}/health" >/dev/null 2>&1; then break; fi
      [ "$i" -eq 30 ] && { echo "Timeout"; docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs backend; exit 1; }
      sleep 2
    done
  fi
  export SURREAL_URL="http://127.0.0.1:${SURREALDB_PORT}"
  export REDIS_URL="redis://127.0.0.1:${REDIS_PORT}/"
  export SURREAL_NS="${SURREAL_NS:-stg_rd}"
  export SURREAL_DB="${SURREAL_DB:-stg_rd}"
  export SURREAL_USER="${SURREAL_USER:-root}"
  export SURREAL_PASSWORD="${SURREAL_PASSWORD:-root}"
  cargo test -p testing --no-fail-fast
}

run_e2e() {
  echo "==> E2E (stack up, smoke, then down)"
  docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d --build
  echo "==> Waiting for backend /health..."
  for i in $(seq 1 30); do
    if curl -sf "http://127.0.0.1:${BACKEND_PORT}/health" >/dev/null 2>&1; then
      echo "Backend healthy."
      break
    fi
    [ "$i" -eq 30 ] && { docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs backend; docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down; exit 1; }
    sleep 2
  done
  curl -sf "http://127.0.0.1:${BACKEND_PORT}/health" && echo " OK"
  docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down
}

case "${1:-all}" in
  build)        run_build ;;
  unit)         run_unit ;;
  integration)  run_integration ;;
  e2e)          run_e2e ;;
  all)
    run_build
    run_unit
    run_integration
    run_e2e
    ;;
  *)
    echo "Usage: $0 [build|unit|integration|e2e|all] [dev|prod]"
    echo "       Or set ENV=dev or ENV=prod to choose config/.env.dev or config/.env.prod"
    exit 1
    ;;
esac
echo "==> Done"
