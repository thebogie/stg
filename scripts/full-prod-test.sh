#!/usr/bin/env bash
#
# Production deploy gate — push-ready only when every test tier passes.
#
# Exit 0 requires ALL of the following (any failure → exit 1, summary shows which tier failed):
#   1) Unit:        cargo test -p backend (full backend crate against live Redis/Surreal ports below)
#   2) Integration: cargo test -p testing --test api_tests (smoke), then
#                   cargo test -p testing -- --include-ignored --test-threads 1 (full crate, incl. ignored)
#   3) E2E:         npx playwright test (full Playwright suite vs stack frontend; USE_PRODUCTION_CONTAINERS=1)
#
# BGG CSV import and docker iterate/clean skips only affect speed/data — they do not substitute for the three tiers above.
#
# What else a green run validates (same Dockerfiles / compose as production):
#   • Stack: docker compose (optional volume wipe unless iterating), images stg-backend / stg-frontend :BUILD_VERSION
#   • Services: SurrealDB, Redis, backend /health, frontend at FRONTEND_PORT
#   • Schema + Surreal functions applied
#   • Optional bgg_catalog import when data/bgg/boardgames_ranks.csv exists (see BGG block; timeouts in header below)
#
# Artifacts: _build/<BUILD_VERSION>/summary.txt, summary.json, deploy_gate.txt (on success), and per-phase logs.
#
# Usage (from repo root, after config/.env.prod exists — see config/setup-env.sh prod):
#   ./scripts/full-prod-test.sh
#
# Faster iteration after a failure (keep volumes; skip BGG re-import when catalog is already large):
#   FULL_PROD_TEST_ITERATE=1 ./scripts/full-prod-test.sh
#   • Skips docker compose down -v so Surreal/Redis volumes keep bgg_catalog from the last run.
#   • With FULL_PROD_TEST_REUSE_BGG_CATALOG=1: skips import when row count is high enough (jq + HTTP /sql).
#   Full catalog reuse threshold defaults to 100000 rows; capped imports use ~90% of BGG_IMPORT_MAX_ROWS.
#   Force a fresh import: FULL_PROD_TEST_FORCE_BGG_IMPORT=1
#   Only keep volumes (still re-import): FULL_PROD_TEST_KEEP_VOLUMES=1 (without ITERATE)
#
# If a run stalls for hours, typical causes:
#   • BGG import: FULL_PROD_TEST_BGG_CATALOG_FULL=1 or huge CSV — use FULL_PROD_TEST_SKIP_BGG_IMPORT=1 or
#     FULL_PROD_TEST_BGG_IMPORT_TIMEOUT_SEC (default 10800 = 3h; 0 = no timeout).
#   • cargo clean every run: FULL_PROD_TEST_SKIP_CLEAN=1 (faster rebuilds when iterating).
#   • Playwright: PLAYWRIGHT_GLOBAL_TIMEOUT_MS (default 7200000 = 2h for the whole E2E run).
#
# Requires: Docker, cargo, Node/npx (Playwright), jq recommended for ITERATE / REUSE_BGG_CATALOG
#
# For line coverage reports (optional, separate from this gate): just coverage  (see docs/testing/COVERAGE_GUIDE.md)
#
# After green: commit & push → CI/GHCR → on server: ./deploy_stg.sh <tag> from deploy/ (see deploy/_instructions.txt).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

stamp() { echo "==> [$(date -u +%Y-%m-%dT%H:%M:%SZ)] $*"; }

# Load production env (BACKEND_PORT, SURREAL_*, etc.)
source "$SCRIPT_DIR/load-env.sh" prod

export COMPOSE_PROJECT_NAME=stg
COMPOSE_FILE="$ROOT/deploy/docker-compose.full.yml"
ENV_FILE="$ROOT/config/.env.prod"

# Shorthand: keep DB volumes + skip redundant bgg_catalog import when already populated
if [ "${FULL_PROD_TEST_ITERATE:-0}" = "1" ]; then
  export FULL_PROD_TEST_KEEP_VOLUMES="${FULL_PROD_TEST_KEEP_VOLUMES:-1}"
  export FULL_PROD_TEST_REUSE_BGG_CATALOG="${FULL_PROD_TEST_REUSE_BGG_CATALOG:-1}"
fi

# --- Phase 0: Clean project stack (containers + named volumes) ---
if [ "${FULL_PROD_TEST_KEEP_VOLUMES:-0}" = "1" ]; then
  echo "==> [0/7] Stopping project docker stack (keeping named volumes — FULL_PROD_TEST_KEEP_VOLUMES=1)"
else
  echo "==> [0/7] Stopping and cleaning project docker stack (containers + named volumes)"
fi
if [ -f "$COMPOSE_FILE" ] && [ -f "$ENV_FILE" ]; then
  if [ "${FULL_PROD_TEST_KEEP_VOLUMES:-0}" = "1" ]; then
    docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down || true
  else
    docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down -v || true
  fi
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
  --build-arg "SOURCE_HASH=$GIT_COMMIT" \
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

# Optional: fill bgg_catalog from boardgames_ranks.csv (gitignored). Full file is ~175k rows and is slow
# (one UPSERT per row). Default here: import 10k games with newest yearpublished first (enough to exercise the catalog tier).
# For a full import: FULL_PROD_TEST_BGG_CATALOG_FULL=1 ./scripts/full-prod-test.sh
# Or override row cap: BGG_IMPORT_MAX_ROWS=50000 ./scripts/full-prod-test.sh
# Schema is already applied above (minimal surql includes bgg_catalog). Without this file, game search uses `game` + BGG API only.
#
# count_bgg_catalog_rows: Surreal HTTP /sql (same pattern as scripts/run-surreal-script.sh)
count_bgg_catalog_rows() {
  local body out
  body="USE NS ${SURREAL_NS}; USE DB ${SURREAL_DB}; SELECT count() FROM bgg_catalog GROUP ALL;"
  out=$(curl -sS --connect-timeout 5 --max-time 30 -X POST \
    -H "Accept: application/json" \
    -u "${SURREAL_USER}:${SURREAL_PASSWORD}" \
    --data-binary "$body" \
    "http://127.0.0.1:${SURREALDB_PORT}/sql" 2>/dev/null) || true
  echo "$out" | jq -r '[.. | objects | select(has("count")) | .count] | last // 0' 2>/dev/null || echo "0"
}

CSV_BGG="$ROOT/data/bgg/boardgames_ranks.csv"
BGG_GATE_STATUS=""
if [ -f "$CSV_BGG" ] && [ "${FULL_PROD_TEST_SKIP_BGG_IMPORT:-0}" != "1" ]; then
  SKIP_BGG_IMPORT=0
  if [ "${FULL_PROD_TEST_FORCE_BGG_IMPORT:-0}" = "1" ]; then
    echo "==> FULL_PROD_TEST_FORCE_BGG_IMPORT=1 — will run BGG import even if bgg_catalog is already populated."
  elif [ "${FULL_PROD_TEST_REUSE_BGG_CATALOG:-0}" = "1" ]; then
    if command -v jq >/dev/null 2>&1; then
      if [ "${FULL_PROD_TEST_BGG_CATALOG_FULL:-0}" = "1" ]; then
        REUSE_THRESHOLD="${BGG_CATALOG_REUSE_MIN_ROWS:-100000}"
      else
        _CAP="${BGG_IMPORT_MAX_ROWS:-10000}"
        REUSE_THRESHOLD="${BGG_CATALOG_REUSE_MIN_ROWS:-$(( (_CAP * 9 + 9) / 10 ))}"
      fi
      EXISTING="$(count_bgg_catalog_rows | tr -d '[:space:]')"
      EXISTING="${EXISTING:-0}"
      case "$EXISTING" in
        ''|*[!0-9]*) EXISTING=0 ;;
      esac
      if [ "$EXISTING" -ge "$REUSE_THRESHOLD" ] 2>/dev/null; then
        echo "==> bgg_catalog already has ${EXISTING} rows (>= ${REUSE_THRESHOLD}); skipping import (FULL_PROD_TEST_REUSE_BGG_CATALOG=1). Use FULL_PROD_TEST_FORCE_BGG_IMPORT=1 to re-import."
        SKIP_BGG_IMPORT=1
      else
        echo "==> bgg_catalog has ${EXISTING} rows (< ${REUSE_THRESHOLD}); running import."
      fi
    else
      echo "Warning: jq not found; cannot check bgg_catalog size — running import anyway." >&2
    fi
  fi

  if [ "$SKIP_BGG_IMPORT" = "0" ]; then
    if [ "${FULL_PROD_TEST_BGG_CATALOG_FULL:-0}" = "1" ]; then
      unset BGG_IMPORT_MAX_ROWS
      echo "==> Importing full BGG catalog (FULL_PROD_TEST_BGG_CATALOG_FULL=1) from $CSV_BGG — this may take many minutes."
    else
      export BGG_IMPORT_MAX_ROWS="${BGG_IMPORT_MAX_ROWS:-10000}"
      echo "==> Importing BGG catalog (${BGG_IMPORT_MAX_ROWS} newest-by-year games; full import: FULL_PROD_TEST_BGG_CATALOG_FULL=1) from $CSV_BGG ..."
    fi
    stamp "Starting BGG catalog import (skip entirely: FULL_PROD_TEST_SKIP_BGG_IMPORT=1 next run)"
    (
      cd "$ROOT"
      export ENV_FILE_PATH="$ENV_FILE"
      _bgg_timeout="${FULL_PROD_TEST_BGG_IMPORT_TIMEOUT_SEC:-10800}"
      if command -v timeout >/dev/null 2>&1 && [ "$_bgg_timeout" != "0" ]; then
        stamp "BGG import: GNU timeout ${_bgg_timeout}s (no cap: FULL_PROD_TEST_BGG_IMPORT_TIMEOUT_SEC=0)"
        timeout "${_bgg_timeout}" cargo run -p backend --bin import_bgg_catalog -- "$CSV_BGG" "full-prod-test-$BUILD_VERSION"
      else
        stamp "BGG import: no timeout (install coreutils timeout, or set FULL_PROD_TEST_BGG_IMPORT_TIMEOUT_SEC=0)"
        cargo run -p backend --bin import_bgg_catalog -- "$CSV_BGG" "full-prod-test-$BUILD_VERSION"
      fi
    ) || {
      echo "Warning: BGG catalog import failed or hit timeout; continuing (game search may not use bgg_catalog tier)." >&2
    }
    BGG_GATE_STATUS="bgg_catalog import attempted (capped or full; warnings above if timeout/failure)"
  else
    BGG_GATE_STATUS="bgg_catalog unchanged (reuse: row count >= threshold)"
  fi
elif [ "${FULL_PROD_TEST_SKIP_BGG_IMPORT:-0}" = "1" ]; then
  echo "==> FULL_PROD_TEST_SKIP_BGG_IMPORT=1 — skipping BGG catalog import block."
  BGG_GATE_STATUS="bgg_catalog skipped (FULL_PROD_TEST_SKIP_BGG_IMPORT=1)"
else
  echo "==> No $CSV_BGG — skipping BGG catalog import (place CSV there to exercise the catalog search tier)."
  BGG_GATE_STATUS="bgg_catalog skipped (no CSV at data/bgg/boardgames_ranks.csv)"
fi

# Match integration-test env so `cargo test -p backend` (including Redis-backed tests) uses the same
# host ports as the compose stack (REDIS_PORT may differ from the tests' default 6379).
export SURREAL_URL="http://127.0.0.1:${SURREALDB_PORT}"
export REDIS_URL="redis://127.0.0.1:${REDIS_PORT}/"
export SURREAL_NS="${SURREAL_NS:-stg_rd}"
export SURREAL_DB="${SURREAL_DB:-stg_rd}"
export SURREAL_USER="${SURREAL_USER:-root}"
export SURREAL_PASSWORD="${SURREAL_PASSWORD:-root}"

# --- Phase 4: Unit tests ---
# cargo test flag as an array element so a stray newline after `--no-f` cannot leave `ail-fast`
# as the next shell command (bash: "ail-fast: command not found").
CARGO_NO_FAIL_FAST=(--no-fail-fast)

# Clean test crates so unit/integration always use current source (avoids stale backend artifact).
if [ "${FULL_PROD_TEST_SKIP_CLEAN:-0}" = "1" ]; then
  stamp "[4/7] Skipping cargo clean (FULL_PROD_TEST_SKIP_CLEAN=1 — faster; use when source is unchanged)"
else
  echo "==> [4/7] Cleaning backend and testing crates (fresh build for tests)"
  cargo clean -p backend -p testing 2>/dev/null || true
fi

echo ""
echo "==> Production gate — all three tiers must pass (unit + integration + E2E). Any failure aborts green exit."
echo ""

stamp "[4/7] Unit tests (backend)"
set +e
cargo test -p backend "${CARGO_NO_FAIL_FAST[@]}" 2>&1 | tee "$BUILD_DIR/unit.log"
UNIT_OK=${PIPESTATUS[0]:-$?}
set -e
[ "$UNIT_OK" -eq 0 ] && UNIT_RESULT="pass" || UNIT_RESULT="fail"

# --- Phase 5: Integration tests ---
stamp "[5/7] Integration tests (against production stack)"
# SURREAL_*/REDIS_URL already exported before unit tests (same stack).

# Single thread so Surreal scope (USE NS/DB) is reused within each test (register then login)
# Fail fast on core auth/API smoke (api_tests) to avoid burning time running the full suite when login is broken.
set +e
echo "==> Integration smoke: testing::api_tests (fast fail)"
cargo test -p testing --test api_tests "${CARGO_NO_FAIL_FAST[@]}" -- --test-threads 1 --nocapture 2>&1 | tee "$BUILD_DIR/integration.api_tests.log"
API_OK=${PIPESTATUS[0]:-$?}

if [ "$API_OK" -ne 0 ]; then
  echo "==> Integration smoke failed (api_tests). Skipping remaining integration tests."
  INTEG_OK="$API_OK"
else
  echo "==> Full integration suite (include ignored)"
  cargo test -p testing "${CARGO_NO_FAIL_FAST[@]}" -- --include-ignored --test-threads 1 --nocapture 2>&1 | tee "$BUILD_DIR/integration.log"
  INTEG_OK=${PIPESTATUS[0]:-$?}
fi
set -e

[ "$INTEG_OK" -eq 0 ] && INTEG_RESULT="pass" || INTEG_RESULT="fail"

# --- Phase 6: Full E2E (Playwright against stack frontend) ---
stamp "[6/7] Playwright E2E — http://127.0.0.1:${FRONTEND_PORT:-50003} (global timeout ${PLAYWRIGHT_GLOBAL_TIMEOUT_MS:-7200000} ms)"
export USE_PRODUCTION_CONTAINERS=1
export PLAYWRIGHT_BASE_URL="http://127.0.0.1:${FRONTEND_PORT:-50003}"
export CI=1
set +e
PW_GLOBAL="${PLAYWRIGHT_GLOBAL_TIMEOUT_MS:-7200000}"
npx playwright test --global-timeout="$PW_GLOBAL" 2>&1 | tee "$BUILD_DIR/e2e.log"
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

DEPLOY_GATE_TXT="$BUILD_DIR/deploy_gate.txt"
if [ "$OVERALL" = "pass" ]; then
  {
    echo "Production deploy gate: PASS"
    echo ""
    echo "Build version: $BUILD_VERSION"
    echo "Git commit:    $GIT_COMMIT"
    echo "Build date:    $BUILD_DATE (UTC in summary.json)"
    echo ""
    echo "Validated for deploy (all required):"
    echo "  - Unit:         PASS — cargo test -p backend (see unit.log)"
    echo "  - Integration:  PASS — cargo test -p testing incl. --include-ignored (see integration*.log)"
    echo "  - E2E:          PASS — npx playwright test (see e2e.log)"
    echo ""
    echo "Stack / data (supporting):"
    echo "  - Images:       stg-backend:$BUILD_VERSION, stg-frontend:$BUILD_VERSION"
    echo "  - Stack:        SurrealDB, Redis, backend /health, frontend http://127.0.0.1:${FRONTEND_PORT:-50003}"
    echo "  - Schema:       minimal surql + Surreal functions applied"
    echo "  - bgg_catalog:  ${BGG_GATE_STATUS:-unknown}"
    echo ""
    echo "Next: push main → CI tags GHCR → ./deploy_stg.sh $BUILD_VERSION (see deploy/_instructions.txt)"
  } | tee "$DEPLOY_GATE_TXT"
fi

if [ "$OVERALL" = "fail" ]; then
  echo ""
  echo "==> One or more phases failed. See $BUILD_DIR for logs."
  echo "    To retry tests without wiping DB volumes or re-importing a full bgg_catalog: FULL_PROD_TEST_ITERATE=1 ./scripts/full-prod-test.sh"
  exit 1
fi

echo ""
echo "==> Production deploy gate: PASS (unit + integration + E2E)"
echo "==> Proof written to $DEPLOY_GATE_TXT"
echo "==> Images stg-backend:$BUILD_VERSION and stg-frontend:$BUILD_VERSION built; full stack + all three test tiers passed."
echo "==> Commit and push to main → GHCR builds and labels backend (and CI produces frontend artifact)."
echo "==> On production (from deploy/ on server): ./deploy_stg.sh $BUILD_VERSION  (or ./deploy_stg.sh latest after CI)."
