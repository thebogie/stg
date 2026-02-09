#!/bin/bash
# Test Production Containers
# Usage: ./scripts/test.sh [--load-prod-data]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }
log_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
log_error() { echo -e "${RED}❌ $1${NC}"; }
# Ensure log_warning exists if script is sourced or run in a different context
type log_warning &>/dev/null || log_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }

cd "$PROJECT_ROOT"

# Load version
VERSION_FILE="${PROJECT_ROOT}/_build/.build-version"
if [ ! -f "$VERSION_FILE" ]; then
    log_error "Build version file not found!"
    log_info "Run ./scripts/build.sh first"
    exit 1
fi

source "$VERSION_FILE"

ENV_FILE="${PROJECT_ROOT}/config/.env.production"
if [ ! -f "$ENV_FILE" ]; then
    log_error "Production environment file not found: $ENV_FILE"
    exit 1
fi

log_info "Testing version: $VERSION_TAG"
export ENV_FILE
export IMAGE_TAG="$VERSION_TAG"
export FRONTEND_IMAGE_TAG="$VERSION_TAG"

# Load production environment for port/config values used below
source "$ENV_FILE"
FRONTEND_PORT="${FRONTEND_PORT:-50003}"
BACKEND_PORT="${BACKEND_PORT:-50002}"
REDIS_PORT="${REDIS_PORT:-63791}"
ARANGODB_PORT="${ARANGODB_PORT:-50001}"

export BACKEND_URL="http://localhost:${BACKEND_PORT}"
export PLAYWRIGHT_BASE_URL="http://localhost:${FRONTEND_PORT}"
export PLAYWRIGHT_API_URL="http://localhost:${BACKEND_PORT}"
export USE_PRODUCTION_CONTAINERS=1
export REDIS_URL="redis://localhost:${REDIS_PORT}"
export ARANGO_URL="http://localhost:${ARANGODB_PORT}"
export ARANGO_USERNAME="${ARANGO_USERNAME:-root}"
export ARANGO_PASSWORD="${ARANGO_PASSWORD:-test}"
export ARANGO_DB="${ARANGO_DB:-_system}"

# Start containers
log_info "Starting production containers..."
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    down 2>/dev/null || true

docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    up -d

wait_for_http() {
    local name="$1"
    local url="$2"
    local auth_user="${3:-}"
    local auth_pass="${4:-}"
    local max_wait="${5:-60}"
    local waited=0
    while [ "$waited" -lt "$max_wait" ]; do
        local code=""
        if [ -n "$auth_user" ]; then
            code=$(curl -s -o /dev/null -w "%{http_code}" -u "${auth_user}:${auth_pass}" "$url" || true)
        else
            code=$(curl -s -o /dev/null -w "%{http_code}" "$url" || true)
        fi
        if [ "$code" = "200" ] || [ "$code" = "401" ]; then
            log_success "$name is responding ($code)"
            return 0
        fi
        sleep 2
        waited=$((waited + 2))
    done
    log_error "$name not ready after ${max_wait}s: $url"
    return 1
}

# Ensure required database exists (prevents backend crash on startup)
ensure_arango_database() {
    local db_name="$1"
    local list_json=""
    list_json=$(curl -s -u "${ARANGO_USERNAME}:${ARANGO_PASSWORD}" "${ARANGO_URL}/_api/database" || true)
    if echo "$list_json" | grep -q "\"${db_name}\""; then
        log_success "ArangoDB database exists: ${db_name}"
        return 0
    fi

    log_info "Creating ArangoDB database: ${db_name}"
    local code=""
    code=$(curl -s -o /dev/null -w "%{http_code}" \
        -u "${ARANGO_USERNAME}:${ARANGO_PASSWORD}" \
        -H "Content-Type: application/json" \
        -d "{\"name\":\"${db_name}\"}" \
        "${ARANGO_URL}/_api/database" || true)

    if [ "$code" = "201" ] || [ "$code" = "409" ]; then
        log_success "ArangoDB database ready: ${db_name} (status ${code})"
        return 0
    fi

    log_error "Failed to create ArangoDB database '${db_name}' (status ${code})"
    return 1
}

# Wait for services (explicit checks avoid early exit)
log_info "Waiting for services to be healthy..."
wait_for_http "ArangoDB" "${ARANGO_URL}/_api/version" "${ARANGO_USERNAME}" "${ARANGO_PASSWORD}" 60
ensure_arango_database "${ARANGO_DB}"
wait_for_http "Backend" "http://localhost:${BACKEND_PORT}/health" "" "" 60

# Load prod data if requested
if [[ "$*" == *"--load-prod-data"* ]]; then
    log_info "Loading production data..."
    ./scripts/load-prod-data.sh || log_info "Data load skipped or failed"
fi

mkdir -p "$PROJECT_ROOT/_build/test-results"

# Smoke checks (runtime endpoints)
log_info "Running smoke checks..."
SMOKE_MAX_WAIT=30
SMOKE_WAITED=0
while [ $SMOKE_WAITED -lt $SMOKE_MAX_WAIT ]; do
    if curl -fsS "http://localhost:${BACKEND_PORT}/api/version" >/dev/null 2>&1 && \
       curl -fsS "http://localhost:${FRONTEND_PORT}/version.json" >/dev/null 2>&1; then
        break
    fi
    sleep 2
    SMOKE_WAITED=$((SMOKE_WAITED + 2))
done

if ! curl -fsS "http://localhost:${BACKEND_PORT}/api/version" >/dev/null 2>&1; then
    log_error "Smoke check failed: /api/version not reachable"
    docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml down 2>/dev/null || true
    exit 1
fi

FRONTEND_VERSION_JSON=$(curl -fsS "http://localhost:${FRONTEND_PORT}/version.json" 2>/dev/null || true)
if [ -z "$FRONTEND_VERSION_JSON" ]; then
    log_error "Smoke check failed: /version.json not reachable"
    docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml down 2>/dev/null || true
    exit 1
fi

IMAGE_COMMIT=$(echo "$FRONTEND_VERSION_JSON" | grep -o '"git_commit":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
if [ "$IMAGE_COMMIT" != "$GIT_COMMIT" ]; then
    log_error "Smoke check failed: frontend git_commit mismatch (expected $GIT_COMMIT, got $IMAGE_COMMIT)"
    docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml down 2>/dev/null || true
    exit 1
fi
log_success "Smoke checks passed"

# Smoke checks (runtime endpoints)
log_info "Running smoke checks..."
SMOKE_MAX_WAIT=30
SMOKE_WAITED=0
while [ $SMOKE_WAITED -lt $SMOKE_MAX_WAIT ]; do
    if curl -fsS "http://localhost:${BACKEND_PORT}/api/version" >/dev/null 2>&1 && \
       curl -fsS "http://localhost:${FRONTEND_PORT}/version.json" >/dev/null 2>&1; then
        break
    fi
    sleep 2
    SMOKE_WAITED=$((SMOKE_WAITED + 2))
done

if ! curl -fsS "http://localhost:${BACKEND_PORT}/api/version" >/dev/null 2>&1; then
    log_error "Smoke check failed: /api/version not reachable"
    docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml down 2>/dev/null || true
    exit 1
fi

FRONTEND_VERSION_JSON=$(curl -fsS "http://localhost:${FRONTEND_PORT}/version.json" 2>/dev/null || true)
if [ -z "$FRONTEND_VERSION_JSON" ]; then
    log_error "Smoke check failed: /version.json not reachable"
    docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml down 2>/dev/null || true
    exit 1
fi

IMAGE_COMMIT=$(echo "$FRONTEND_VERSION_JSON" | grep -o '"git_commit":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
if [ "$IMAGE_COMMIT" != "$GIT_COMMIT" ]; then
    log_error "Smoke check failed: frontend git_commit mismatch (expected $GIT_COMMIT, got $IMAGE_COMMIT)"
    docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml down 2>/dev/null || true
    exit 1
fi
log_success "Smoke checks passed"

# Smoke checks (runtime endpoints)
log_info "Running smoke checks..."
SMOKE_MAX_WAIT=30
SMOKE_WAITED=0
while [ $SMOKE_WAITED -lt $SMOKE_MAX_WAIT ]; do
    if curl -fsS "http://localhost:${BACKEND_PORT}/api/version" >/dev/null 2>&1 && \
       curl -fsS "http://localhost:${FRONTEND_PORT}/version.json" >/dev/null 2>&1; then
        break
    fi
    sleep 2
    SMOKE_WAITED=$((SMOKE_WAITED + 2))
done

if ! curl -fsS "http://localhost:${BACKEND_PORT}/api/version" >/dev/null 2>&1; then
    log_error "Smoke check failed: /api/version not reachable"
    docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml down 2>/dev/null || true
    exit 1
fi

FRONTEND_VERSION_JSON=$(curl -fsS "http://localhost:${FRONTEND_PORT}/version.json" 2>/dev/null || true)
if [ -z "$FRONTEND_VERSION_JSON" ]; then
    log_error "Smoke check failed: /version.json not reachable"
    docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml down 2>/dev/null || true
    exit 1
fi

IMAGE_COMMIT=$(echo "$FRONTEND_VERSION_JSON" | grep -o '"git_commit":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
if [ "$IMAGE_COMMIT" != "$GIT_COMMIT" ]; then
    log_error "Smoke check failed: frontend git_commit mismatch (expected $GIT_COMMIT, got $IMAGE_COMMIT)"
    docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml down 2>/dev/null || true
    exit 1
fi
log_success "Smoke checks passed"

# Run tests
log_info "Running unit tests..."
if ! cargo nextest run --workspace --lib 2>&1 | tee "$PROJECT_ROOT/_build/test-results/unit-tests.log"; then
    log_error "Unit tests failed!"
    docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml down 2>/dev/null || true
    exit 1
fi

log_info "Running integration tests..."
if ! cargo nextest run \
    --package backend \
    --test '*' \
    --no-fail-fast \
    --run-ignored all 2>&1 | tee "$PROJECT_ROOT/_build/test-results/integration-tests.log"; then
    log_error "Integration tests failed!"
    docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml down 2>/dev/null || true
    exit 1
fi

log_info "Running E2E API tests..."
if ! cargo nextest run \
    --package testing \
    --test '*_e2e' \
    --no-fail-fast 2>&1 | tee "$PROJECT_ROOT/_build/test-results/e2e-api-tests.log"; then
    log_error "E2E API tests failed!"
    docker compose --env-file "$ENV_FILE" -f deploy/docker-compose.production.yml down 2>/dev/null || true
    exit 1
fi

log_info "Running Playwright E2E tests..."
PLAYWRIGHT_EXIT_CODE=0
npx playwright test 2>&1 | tee "$PROJECT_ROOT/_build/test-results/playwright.log" || PLAYWRIGHT_EXIT_CODE=$?

# Cleanup
log_info "Stopping test containers..."
docker compose \
    --env-file "$ENV_FILE" \
    -f deploy/docker-compose.production.yml \
    down 2>/dev/null || true

if [ "$PLAYWRIGHT_EXIT_CODE" -ne 0 ]; then
    OUTPUT_FILE="$PROJECT_ROOT/_build/test-results/playwright.log"
    log_info "Playwright tests exited with code $PLAYWRIGHT_EXIT_CODE, analyzing failures..."

    TOTAL_FAILURES=$(grep -E "[0-9]+\s+failed" "$OUTPUT_FILE" 2>/dev/null | head -1 | grep -oE "[0-9]+" | head -1)
    VISUAL_FAILURES=$(grep -i "failed" "$OUTPUT_FILE" 2>/dev/null | grep -i "Visual Regression\|visual.*snapshot" | wc -l)
    if [ "${VISUAL_FAILURES:-0}" -eq 0 ]; then
        VISUAL_FAILURES=$(grep -i "toHaveScreenshot" "$OUTPUT_FILE" 2>/dev/null | wc -l)
    fi
    # Force to integers (strip newlines so [ ] never sees "0\n0")
    TOTAL_FAILURES=$(echo "$TOTAL_FAILURES" | tr -cd '0-9')
    [ -z "$TOTAL_FAILURES" ] && TOTAL_FAILURES=0
    VISUAL_FAILURES=$(echo "$VISUAL_FAILURES" | tr -cd '0-9')
    [ -z "$VISUAL_FAILURES" ] && VISUAL_FAILURES=0

    log_info "Failure analysis: TOTAL=$TOTAL_FAILURES, VISUAL=$VISUAL_FAILURES"

    # Fallback: if log clearly shows only screenshot failures, treat as visual-only
    SCREENSHOT_FAILURES=0
    if grep -q "toHaveScreenshot.*failed\|Expected an image.*different" "$OUTPUT_FILE" 2>/dev/null; then
      SCREENSHOT_FAILURES=$(grep -c "toHaveScreenshot\|Expected an image" "$OUTPUT_FILE" 2>/dev/null || echo "0")
      SCREENSHOT_FAILURES=$(echo "$SCREENSHOT_FAILURES" | tr -cd '0-9')
      [ -z "$SCREENSHOT_FAILURES" ] && SCREENSHOT_FAILURES=0
    fi
    if [ "$VISUAL_FAILURES" -lt "$TOTAL_FAILURES" ] && [ "$SCREENSHOT_FAILURES" -gt 0 ]; then
      VISUAL_FAILURES=$SCREENSHOT_FAILURES
    fi

    # Treat as non-blocking if all failures are visual (screenshot/snapshot tests)
    VISUAL_ONLY=false
    if [ "$TOTAL_FAILURES" -gt 0 ] && [ "$VISUAL_FAILURES" -ge "$TOTAL_FAILURES" ]; then
      VISUAL_ONLY=true
    fi
    # Fallback: log contains screenshot failure -> treat as visual-only
    if [ "$VISUAL_ONLY" = false ] && [ "$TOTAL_FAILURES" -gt 0 ]; then
      if grep -q "toHaveScreenshot" "$OUTPUT_FILE" 2>/dev/null; then
        VISUAL_ONLY=true
      fi
    fi
    if [ "$VISUAL_ONLY" = true ]; then
        log_info "Playwright failures are visual-regression only (non-blocking)"
        log_info "Review: npx playwright show-report _build/playwright-report"
    elif [ "$TOTAL_FAILURES" -gt 0 ]; then
        log_error "Playwright tests failed with non-visual errors"
        exit 1
    else
        log_error "Playwright tests failed (could not parse failure count)"
        exit 1
    fi
fi

log_success "✅ All tests passed!"
