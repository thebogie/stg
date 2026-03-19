#!/usr/bin/env bash
# Full production-style test pipeline. Builds and runs the full stack as production would.
# - Stops and clears the local docker-compose stack (including volumes) for this project
# - Builds production-style backend and frontend images (same Dockerfiles/labels as CI/GHCR)
# - Starts the full stack (SurrealDB, Redis, backend, frontend) via deploy/docker-compose.full.yml
# - Waits for all services (including frontend at FRONTEND_PORT) to be ready
# - Applies minimal SurrealDB schema and functions for tests
# - Runs backend unit tests, integration tests, and Playwright E2E against the stack
# - Prints a summary and writes detailed logs under _build/<build_version>/
#
# No services are assumed running: the script brings up the entire stack itself.
#
# Usage:
#   ./scripts/full-prod-test.sh
#
# Requires: config/.env.prod (see config/setup-env.sh prod), Docker, cargo, Node/npx (Playwright)
# Flow: run locally → all pass → commit & push to main → GHCR builds and labels backend (and
#       frontend artifact in CI) → on production: ./deploy/deploy_stg.sh <tag> (from repo) or ./deploy_stg.sh <tag> (from deploy/ on server)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

# Load production env (BACKEND_PORT, SURREAL_*, etc.)
source "$SCRIPT_DIR/load-env.sh" prod

export COMPOSE_PROJECT_NAME=stg
COMPOSE_FILE="$ROOT/deploy/docker-compose.full.yml"
ENV_FILE="$ROOT/config/.env.prod"

# --- Phase 0: Clean project stack (containers + named volumes) ---
echo "==> [0/7] Stopping and cleaning project docker stack (containers + named volumes)"
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

# Resolve VOLUME_PATH for compose (local test run: keep data under _build)
VOL_BASE="${VOLUME_PATH:-$ROOT/_build/docker-data}"
VOL_BASE="$(cd "$VOL_BASE" 2>/dev/null && pwd)" || VOL_BASE="$ROOT/_build/docker-data"
mkdir -p "$VOL_BASE/surrealdb_data" "$VOL_BASE/redis_data" "$VOL_BASE/backend_data"
export VOLUME_PATH="$VOL_BASE"
chmod 777 "$VOLUME_PATH/surrealdb_data" 2>/dev/null || true

echo "==> Build version: $BUILD_VERSION (commit $GIT_COMMIT, $BUILD_DATE)"
echo "==> Results will be in $BUILD_DIR"
echo ""

# --- Phase 1: Build production backend and frontend images ---
echo "==> [1/7] Building production backend image stg-backend:$BUILD_VERSION"
docker build -f back/api/Dockerfile.backend \
  --build-arg "GIT_COMMIT=$GIT_COMMIT" \
  --build-arg "BUILD_DATE=$BUILD_DATE" \
  --build-arg RUST_ENV=production \
  --build-arg RUST_LOG=info \
  --label "org.opencontainers.image.revision=$GIT_COMMIT" \
  --label "org.opencontainers.image.created=$BUILD_DATE" \
  --label "org.opencontainers.image.version=$BUILD_VERSION" \
  -t "stg-backend:$BUILD_VERSION" .

echo "==> [2/7] Building production frontend image stg-frontend:$BUILD_VERSION"
docker build -f front/web/Dockerfile.frontend.caddy \
  --build-arg "GIT_COMMIT=$GIT_COMMIT" \
  --build-arg "BUILD_DATE=$BUILD_DATE" \
  --build-arg "BUILD_VERSION=$BUILD_VERSION" \
  -t "stg-frontend:$BUILD_VERSION" .

# --- Phase 3: Start full production stack (SurrealDB, Redis, backend, frontend) ---
echo "==> [3/7] Starting full production stack (SurrealDB, Redis, backend, frontend)"
export BACKEND_IMAGE="stg-backend:$BUILD_VERSION"
export FRONTEND_IMAGE="stg-frontend:$BUILD_VERSION"
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

echo "==> Waiting for frontend on 127.0.0.1:${FRONTEND_PORT:-50003}..."
for i in $(seq 1 30); do
  if curl -sf --connect-timeout 2 "http://127.0.0.1:${FRONTEND_PORT:-50003}" >/dev/null 2>&1; then
    echo "Frontend ready."
    break
  fi
  if [ "$i" -eq 30 ]; then
    docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs frontend
    docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down
    echo "{\"build_version\":\"$BUILD_VERSION\",\"unit\":\"skip\",\"integration\":\"skip\",\"e2e\":\"fail\",\"overall\":\"fail\"}" > "$SUMMARY_JSON"
    echo "FAIL: frontend did not become ready" > "$SUMMARY_TXT"
    exit 1
  fi
  sleep 2
done

# Apply minimal schema (player table) so integration tests work without production data
echo "==> Applying minimal SurrealDB schema for integration tests..."
bash "$ROOT/scripts/apply-surreal-schema-minimal.sh" || true

# Apply SurrealDB application functions so function-first codepaths are exercised in prod tests
echo "==> Applying SurrealDB functions for integration tests..."
if ! "$ROOT/scripts/apply-surreal-functions.sh"; then
  echo "FAIL: Could not apply SurrealDB functions (required for prod-style tests)." >&2
  docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs surrealdb || true
  docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down || true
  echo "{\"build_version\":\"$BUILD_VERSION\",\"unit\":\"skip\",\"integration\":\"skip\",\"e2e\":\"fail\",\"overall\":\"fail\"}" > "$SUMMARY_JSON"
  echo "FAIL: Could not apply SurrealDB functions" > "$SUMMARY_TXT"
  exit 1
fi

# --- Phase 4: Unit tests ---
# Clean test crates so unit/integration always use current source (avoids stale backend artifact).
echo "==> [4/7] Cleaning backend and testing crates (fresh build for tests)"
cargo clean -p backend -p testing 2>/dev/null || true

echo "==> Unit tests (backend)"
set +e
cargo test -p backend --no-fail-fast 2>&1 | tee "$BUILD_DIR/unit.log"
UNIT_OK=${PIPESTATUS[0]:-$?}
set -e
[ "$UNIT_OK" -eq 0 ] && UNIT_RESULT="pass" || UNIT_RESULT="fail"

# --- Phase 5: Integration tests ---
echo "==> [5/7] Integration tests (against production stack)"
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
cargo test -p testing --test api_tests --no-fail-fast -- --test-threads 1 --nocapture 2>&1 | tee "$BUILD_DIR/integration.api_tests.log"
API_OK=${PIPESTATUS[0]:-$?}

if [ "$API_OK" -ne 0 ]; then
  echo "==> Integration smoke failed (api_tests). Skipping remaining integration tests."
  INTEG_OK="$API_OK"
else
  echo "==> Full integration suite (include ignored)"
  cargo test -p testing --no-fail-fast -- --include-ignored --test-threads 1 --nocapture 2>&1 | tee "$BUILD_DIR/integration.log"
  INTEG_OK=${PIPESTATUS[0]:-$?}
fi
set -e

[ "$INTEG_OK" -eq 0 ] && INTEG_RESULT="pass" || INTEG_RESULT="fail"

# --- Phase 6: Full E2E (Playwright against stack frontend) ---
echo "==> [6/7] Running Playwright E2E against stack frontend http://127.0.0.1:${FRONTEND_PORT:-50003}"
export USE_PRODUCTION_CONTAINERS=1
export PLAYWRIGHT_BASE_URL="http://127.0.0.1:${FRONTEND_PORT:-50003}"
export CI=1
set +e
npx playwright test 2>&1 | tee "$BUILD_DIR/e2e.log"
E2E_OK=$?
set -e
[ "$E2E_OK" -eq 0 ] && E2E_RESULT="pass" || E2E_RESULT="fail"
mkdir -p "$BUILD_DIR/e2e"
[ -d "_build/test-results" ] && cp -r _build/test-results "$BUILD_DIR/e2e/" 2>/dev/null || true
[ -d "_build/playwright-report" ] && cp -r _build/playwright-report "$BUILD_DIR/e2e/" 2>/dev/null || true

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
echo "==> All passed. Images stg-backend:$BUILD_VERSION and stg-frontend:$BUILD_VERSION are ready."
echo "==> Commit and push to main → GHCR builds and labels backend (and CI produces frontend artifact)."
echo "==> On production (from deploy/ on server): ./deploy_stg.sh $BUILD_VERSION  (or ./deploy_stg.sh latest after CI)."
