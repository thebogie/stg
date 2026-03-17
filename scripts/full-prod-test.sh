#!/usr/bin/env bash
# Full production-style test pipeline. Starts all pieces as production would run them.
# - Stops and clears the local docker-compose stack (including volumes) for this project
# - Builds fresh production-style backend image with same Dockerfile/labels as GHCR
# - Starts the full production stack (SurrealDB, Redis, backend) via deploy/docker-compose.yml
# - Waits for SurrealDB, Redis, and backend to be ready before running any tests
# - Applies minimal SurrealDB schema for tests
# - Runs backend unit tests, integration tests, and frontend E2E against the running stack
# - Prints a summary and writes detailed logs under _build/<build_version>/
#
# No services are assumed running: the script brings up the entire stack itself.
#
# Usage:
#   ./scripts/full-prod-test.sh
#
# Requires: config/.env.prod (see config/setup-env.sh prod), Docker, cargo
# Flow: run locally → commit → push to main → GHCR builds same image (sha + latest) → pull on production.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

# Load production env (BACKEND_PORT, SURREAL_*, etc.)
source "$SCRIPT_DIR/load-env.sh" prod

export COMPOSE_PROJECT_NAME=stg
COMPOSE_FILE="$ROOT/deploy/docker-compose.yml"
ENV_FILE="$ROOT/config/.env.prod"

# --- Phase 0: Clean project stack (containers + named volumes) ---
echo "==> [0/6] Stopping and cleaning project docker stack (containers + named volumes)"
if [ -f "$COMPOSE_FILE" ] && [ -f "$ENV_FILE" ]; then
  docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down -v || true
else
  echo "Warning: compose or env file missing; skipping docker compose down."
fi

# Build version: same identifier for image tag, footer, and _build output (YYYYMMDD-HHMMSS-<short-sha>)
GIT_COMMIT="$(git rev-parse --short HEAD)"
BUILD_DATE="$(date -u +"%Y-%m-%d %H:%M:%S UTC")"
BUILD_VERSION="$(date -u +%Y%m%d-%H%M%S)-${GIT_COMMIT}"
export GIT_COMMIT BUILD_DATE BUILD_VERSION

BUILD_DIR="_build/${BUILD_VERSION}"
mkdir -p "$BUILD_DIR"
SUMMARY_JSON="$BUILD_DIR/summary.json"
SUMMARY_TXT="$BUILD_DIR/summary.txt"

# Resolve VOLUME_PATH for compose
VOL_BASE="${VOLUME_PATH:-$ROOT/docker-data}"
VOL_BASE="$(cd "$VOL_BASE" 2>/dev/null && pwd)" || VOL_BASE="$ROOT/docker-data"
mkdir -p "$VOL_BASE/surrealdb_data" "$VOL_BASE/redis_data" "$VOL_BASE/backend_data"
export VOLUME_PATH="$VOL_BASE"
chmod 777 "$VOLUME_PATH/surrealdb_data" 2>/dev/null || true

echo "==> Build version: $BUILD_VERSION (commit $GIT_COMMIT, $BUILD_DATE)"
echo "==> Results will be in $BUILD_DIR"
echo ""

# --- Phase 1: Build production backend image ---
echo "==> [1/6] Building production backend image stg-backend:$BUILD_VERSION"
docker build -f back/api/Dockerfile.backend \
  --build-arg "GIT_COMMIT=$GIT_COMMIT" \
  --build-arg "BUILD_DATE=$BUILD_DATE" \
  --build-arg RUST_ENV=production \
  --build-arg RUST_LOG=info \
  --label "org.opencontainers.image.revision=$GIT_COMMIT" \
  --label "org.opencontainers.image.created=$BUILD_DATE" \
  --label "org.opencontainers.image.version=$BUILD_VERSION" \
  -t "stg-backend:$BUILD_VERSION" .

# --- Phase 2: Start full production stack (all pieces as production) ---
echo "==> [2/6] Starting full production stack (SurrealDB, Redis, backend)"
export BACKEND_IMAGE="stg-backend:$BUILD_VERSION"
export IMAGE_TAG="$BUILD_VERSION"
docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d

# Wait for each service to be reachable on host (same as production)
echo "==> Waiting for SurrealDB on 127.0.0.1:${SURREALDB_PORT}..."
for i in $(seq 1 30); do
  if curl -sf --connect-timeout 2 "http://127.0.0.1:${SURREALDB_PORT}" >/dev/null 2>&1; then
    echo "SurrealDB ready."
    break
  fi
  if [ "$i" -eq 30 ]; then
    docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs surrealdb
    docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down
    echo "{\"build_version\":\"$BUILD_VERSION\",\"unit\":\"skip\",\"integration\":\"skip\",\"e2e\":\"fail\",\"overall\":\"fail\"}" > "$SUMMARY_JSON"
    echo "FAIL: SurrealDB did not become ready" > "$SUMMARY_TXT"
    exit 1
  fi
  sleep 2
done

echo "==> Waiting for Redis on 127.0.0.1:${REDIS_PORT}..."
redis_ready() {
  if command -v redis-cli >/dev/null 2>&1; then
    redis-cli -p "${REDIS_PORT}" -h 127.0.0.1 ping 2>/dev/null | grep -q PONG
  else
  # Fallback: TCP port open (bash)
    (echo >/dev/tcp/127.0.0.1/${REDIS_PORT}) 2>/dev/null
  fi
}
for i in $(seq 1 30); do
  if redis_ready; then
    echo "Redis ready."
    break
  fi
  if [ "$i" -eq 30 ]; then
    docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs redis
    docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down
    echo "{\"build_version\":\"$BUILD_VERSION\",\"unit\":\"skip\",\"integration\":\"skip\",\"e2e\":\"fail\",\"overall\":\"fail\"}" > "$SUMMARY_JSON"
    echo "FAIL: Redis did not become ready" > "$SUMMARY_TXT"
    exit 1
  fi
  sleep 2
done

echo "==> Waiting for backend /health..."
for i in $(seq 1 30); do
  if curl -sf "http://127.0.0.1:${BACKEND_PORT}/health" >/dev/null 2>&1; then
    echo "Backend healthy."
    break
  fi
  if [ "$i" -eq 30 ]; then
    docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs backend
    docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down
    echo "{\"build_version\":\"$BUILD_VERSION\",\"unit\":\"skip\",\"integration\":\"skip\",\"e2e\":\"fail\",\"overall\":\"fail\"}" > "$SUMMARY_JSON"
    echo "FAIL: backend did not become healthy" > "$SUMMARY_TXT"
    exit 1
  fi
  sleep 2
done

# Apply minimal schema (player table) so integration tests work without production data
echo "==> Applying minimal SurrealDB schema for integration tests..."
"$ROOT/scripts/apply-surreal-schema-minimal.sh" || true

# --- Phase 3: Unit tests ---
echo "==> [3/6] Unit tests (backend)"
set +e
cargo test -p backend --no-fail-fast 2>&1 | tee "$BUILD_DIR/unit.log"
UNIT_OK=${PIPESTATUS[0]:-$?}
set -e
[ "$UNIT_OK" -eq 0 ] && UNIT_RESULT="pass" || UNIT_RESULT="fail"

# --- Phase 4: Integration tests ---
echo "==> [4/6] Integration tests (against production stack)"
export SURREAL_URL="http://127.0.0.1:${SURREALDB_PORT}"
export REDIS_URL="redis://127.0.0.1:${REDIS_PORT}/"
export SURREAL_NS="${SURREAL_NS:-stg_rd}"
export SURREAL_DB="${SURREAL_DB:-stg_rd}"
export SURREAL_USER="${SURREAL_USER:-root}"
export SURREAL_PASSWORD="${SURREAL_PASSWORD:-root}"

# Single thread so Surreal scope (USE NS/DB) is reused within each test (register then login)
# Fail fast on core auth/API smoke (api_tests) to avoid burning time running the full suite when login is broken.
set +e
echo "==> Integration smoke: testing::api_tests (fast fail)"
cargo test -p testing --test api_tests --no-fail-fast -- --test-threads 1 2>&1 | tee "$BUILD_DIR/integration.api_tests.log"
API_OK=${PIPESTATUS[0]:-$?}

if [ "$API_OK" -ne 0 ]; then
  echo "==> Integration smoke failed (api_tests). Skipping remaining integration tests."
  INTEG_OK="$API_OK"
else
  echo "==> Full integration suite (include ignored)"
  cargo test -p testing --no-fail-fast -- --include-ignored --test-threads 1 2>&1 | tee "$BUILD_DIR/integration.log"
  INTEG_OK=${PIPESTATUS[0]:-$?}
fi
set -e

[ "$INTEG_OK" -eq 0 ] && INTEG_RESULT="pass" || INTEG_RESULT="fail"

# --- Phase 5: Full E2E (frontend + backend up, Playwright) ---
echo "==> [5/6] Starting frontend (Trunk) for E2E..."
FRONTEND_PID=""
( "$ROOT/scripts/start-front.sh" prod ) & FRONTEND_PID=$!
cleanup_frontend() {
  if [ -n "$FRONTEND_PID" ]; then
    kill "$FRONTEND_PID" 2>/dev/null || true
    wait "$FRONTEND_PID" 2>/dev/null || true
  fi
}
trap cleanup_frontend EXIT

echo "==> Waiting for frontend at http://127.0.0.1:${FRONTEND_PORT} (up to 180s)..."
E2E_FRONTEND_READY=0
for i in $(seq 1 90); do
  if curl -sf --connect-timeout 2 "http://127.0.0.1:${FRONTEND_PORT}" >/dev/null 2>&1; then
    E2E_FRONTEND_READY=1
    echo "Frontend ready."
    break
  fi
  sleep 2
done
if [ "$E2E_FRONTEND_READY" -eq 0 ]; then
  echo "Frontend did not become ready in time."
  E2E_RESULT="fail"
  E2E_OK=1
else
  echo "==> [6/6] Running Playwright E2E against http://127.0.0.1:${FRONTEND_PORT}"
  export USE_PRODUCTION_CONTAINERS=1
  export PLAYWRIGHT_BASE_URL="http://127.0.0.1:${FRONTEND_PORT}"
  export CI=1
  set +e
  npx playwright test 2>&1 | tee "$BUILD_DIR/e2e.log"
  E2E_OK=$?
  set -e
  [ "$E2E_OK" -eq 0 ] && E2E_RESULT="pass" || E2E_RESULT="fail"
  mkdir -p "$BUILD_DIR/e2e"
  [ -d "_build/test-results" ] && cp -r _build/test-results "$BUILD_DIR/e2e/" 2>/dev/null || true
  [ -d "_build/playwright-report" ] && cp -r _build/playwright-report "$BUILD_DIR/e2e/" 2>/dev/null || true
fi

cleanup_frontend
trap - EXIT

# --- Tear down stack ---
echo "==> Tearing down production stack..."
docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down

# --- Summary ---
OVERALL="pass"
[ "$UNIT_RESULT" != "pass" ] || [ "$INTEG_RESULT" != "pass" ] || [ "$E2E_RESULT" != "pass" ] && OVERALL="fail"

cat > "$SUMMARY_JSON" << EOF
{
  "build_version": "$BUILD_VERSION",
  "git_commit": "$GIT_COMMIT",
  "build_date": "$BUILD_DATE",
  "unit": "$UNIT_RESULT",
  "integration": "$INTEG_RESULT",
  "e2e": "$E2E_RESULT",
  "overall": "$OVERALL"
}
EOF

{
  echo "Build version: $BUILD_VERSION"
  echo "Git commit:   $GIT_COMMIT"
  echo "Build date:   $BUILD_DATE"
  echo "Unit:         $UNIT_RESULT"
  echo "Integration:  $INTEG_RESULT"
  echo "E2E:          $E2E_RESULT"
  echo "Overall:      $OVERALL"
} > "$SUMMARY_TXT"
cat "$SUMMARY_TXT"

if [ "$OVERALL" = "fail" ]; then
  echo ""
  echo "==> One or more phases failed. See $BUILD_DIR for logs."
  exit 1
fi

echo ""
echo "==> All passed. Image stg-backend:$BUILD_VERSION is ready."
echo "==> Commit and push to main to trigger GHCR build (same Dockerfile and labels)."
echo "==> On production: docker pull ghcr.io/<owner>/<repo>/backend:<sha-or-latest> && restart stack with that image."
