#!/usr/bin/env bash
#
# Production deploy gate — push-ready only when every test tier passes.
#
# Exit 0 requires ALL of the following (any failure → exit 1, summary shows which tier failed):
#   1) Unit:        cargo test -p backend (full backend crate against live Redis/Surreal ports below)
#   2) Integration: cargo test -p testing --test api_tests (smoke), then
#                   cargo test -p testing -- --include-ignored --test-threads 1 (full crate, incl. ignored)
#   3) E2E:         Playwright vs stack frontend (USE_PRODUCTION_CONTAINERS=1); default is pre-baked Docker
#                   image (see scripts/build-playwright-image.sh + run-playwright-e2e-docker.sh).
#                   Host run: FULL_PROD_TEST_PLAYWRIGHT_HOST=1 (needs Node).
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
#   • Playwright image: rebuilt when no local stg-playwright:latest (or stg-playwright:$BUILD_VERSION).
#     Set FULL_PROD_TEST_FORCE_PLAYWRIGHT_BUILD=1 to pull/rebuild anyway (needs mcr.microsoft.com).
#     On MCR/network failure during build, reuses stg-playwright:latest when present.
#     Host E2E: FULL_PROD_TEST_PLAYWRIGHT_HOST=1.
#
# Quick checks without running the full gate (Docker builds + tests):
#   ./scripts/verify-gate-syntax.sh
#   FULL_PROD_TEST_STOP_AFTER=env ./scripts/full-prod-test.sh      # exit after env + _build prep (no image builds)
#   FULL_PROD_TEST_STOP_AFTER=images ./scripts/full-prod-test.sh   # exit after backend + frontend Docker builds
#
# Requires: Docker, cargo; jq recommended for ITERATE / REUSE_BGG_CATALOG. Node only if FULL_PROD_TEST_PLAYWRIGHT_HOST=1.
#
# For line coverage reports (optional, separate from this gate): just coverage  (see docs/testing/COVERAGE_GUIDE.md)
#
# After green: commit & push → CI/GHCR → on server: ./deploy_stg.sh <tag> from deploy/ (see deploy/_instructions.txt).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

# Use deploy/ as project dir so ./Caddyfile.frontend in docker-compose.full.yml resolves next to that file
# (not under repo root, where the Caddyfile does not exist).
stg_compose() {
  docker compose -p stg --project-directory "$ROOT/deploy" "$@"
}

# deploy/docker-compose*.yml use fixed container_name values (stg-surrealdb, etc.).
# `compose down` only removes containers for its project label; orphans from project
# "deploy" (older scripts) or manual runs block a fresh `up`.
stg_remove_named_containers() {
  local names=(
    stg-surrealdb stg-redis stg-wait-for-surrealdb stg-backend stg-frontend
    stg-playwright-worker stg-ollama
  )
  for name in "${names[@]}"; do
    if docker inspect "$name" >/dev/null 2>&1; then
      echo "==> Removing leftover container /$name"
      docker rm -f "$name" || true
    fi
  done
}

stamp() { echo "==> [$(date -u +%Y-%m-%dT%H:%M:%SZ)] $*"; }

# Ensure stg-playwright:$BUILD_VERSION exists without re-pulling MCR on every gate run.
ensure_playwright_image() {
  local pw_version="$1"
  local target="stg-playwright:$BUILD_VERSION"
  if docker image inspect "$target" >/dev/null 2>&1; then
    stamp "[2b/7] Playwright image $target already exists — skipping build"
    docker tag "$target" stg-playwright:latest
    return 0
  fi
  if [ "${FULL_PROD_TEST_FORCE_PLAYWRIGHT_BUILD:-0}" != "1" ] \
    && docker image inspect stg-playwright:latest >/dev/null 2>&1; then
    stamp "[2b/7] Reusing stg-playwright:latest as $target (set FULL_PROD_TEST_FORCE_PLAYWRIGHT_BUILD=1 to rebuild from MCR)"
    docker tag stg-playwright:latest "$target"
    return 0
  fi
  if [ "${FULL_PROD_TEST_SKIP_PLAYWRIGHT_BUILD:-0}" = "1" ]; then
    if docker image inspect stg-playwright:latest >/dev/null 2>&1; then
      stamp "[2b/7] FULL_PROD_TEST_SKIP_PLAYWRIGHT_BUILD=1 — reusing stg-playwright:latest as $target"
      docker tag stg-playwright:latest "$target"
      return 0
    fi
    echo "FAIL: FULL_PROD_TEST_SKIP_PLAYWRIGHT_BUILD=1 but stg-playwright:latest not found." >&2
    echo "      Build once when MCR is reachable: ./scripts/build-playwright-image.sh" >&2
    return 1
  fi
  stamp "[2b/7] Building unified Playwright image $target (Playwright $pw_version)"
  if docker build -f deploy/Dockerfile.playwright \
    --build-arg "PLAYWRIGHT_VERSION=$pw_version" \
    -t "$target" \
    -t stg-playwright:latest \
    .; then
    return 0
  fi
  if docker image inspect stg-playwright:latest >/dev/null 2>&1; then
    echo "WARN: Playwright build failed (often MCR TLS timeout); reusing stg-playwright:latest as $target" >&2
    docker tag stg-playwright:latest "$target"
    return 0
  fi
  echo "FAIL: Playwright image build failed and no stg-playwright:latest to reuse." >&2
  echo "      Retry when mcr.microsoft.com is reachable, or: FULL_PROD_TEST_SKIP_PLAYWRIGHT_BUILD=1 ./scripts/test-prod-gate.sh" >&2
  return 1
}

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
DEV_COMPOSE_FILE="$ROOT/deploy/docker-compose.yml"
DEV_ENV_FILE="$ROOT/config/.env.dev"
if [ -f "$DEV_COMPOSE_FILE" ] && [ -f "$DEV_ENV_FILE" ]; then
  echo "==> [0/7] Stopping dev dependency stack (shares container names with prod gate)"
  for compose_proj in stg deploy; do
    docker compose -p "$compose_proj" --project-directory "$ROOT/deploy" \
      -f "$DEV_COMPOSE_FILE" --env-file "$DEV_ENV_FILE" down || true
  done
fi
if [ "${FULL_PROD_TEST_KEEP_VOLUMES:-0}" = "1" ]; then
  echo "==> [0/7] Stopping project docker stack (keeping named volumes — FULL_PROD_TEST_KEEP_VOLUMES=1)"
else
  echo "==> [0/7] Stopping and cleaning project docker stack (containers + named volumes)"
fi
if [ -f "$COMPOSE_FILE" ] && [ -f "$ENV_FILE" ]; then
  # docker-compose.full.yml requires BACKEND_IMAGE / FRONTEND_IMAGE even for `down`.
  # Provide placeholders so `down -v` actually executes and wipes stale SurrealDB credentials.
  export BACKEND_IMAGE="${BACKEND_IMAGE:-stg-backend:local}"
  export FRONTEND_IMAGE="${FRONTEND_IMAGE:-stg-frontend:local}"
  export PLAYWRIGHT_WORKER_IMAGE="${PLAYWRIGHT_WORKER_IMAGE:-stg-playwright:local}"
  for compose_proj in stg deploy; do
    if [ "${FULL_PROD_TEST_KEEP_VOLUMES:-0}" = "1" ]; then
      docker compose -p "$compose_proj" --project-directory "$ROOT/deploy" \
        -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down || true
    else
      docker compose -p "$compose_proj" --project-directory "$ROOT/deploy" \
        -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down -v || true
    fi
  done
else
  echo "Warning: compose or env file missing; skipping docker compose down."
fi
stg_remove_named_containers

# Build version: same identifier for image tag, footer, and _build output (YYYYMMDD-HHMMSS-<short-sha>)
GIT_COMMIT="$(git rev-parse --short HEAD)"
BUILD_DATE="$(date -u +"%Y-%m-%d %H:%M:%S UTC")"
BUILD_VERSION="$(date -u +%Y%m%d-%H%M%S)-${GIT_COMMIT}"
export GIT_COMMIT BUILD_DATE BUILD_VERSION

# --- E2E test users (created per BUILD_VERSION) ---
# These are used by Playwright to make login/admin flows deterministic against prod-copied data.
E2E_USER_EMAIL="${E2E_USER_EMAIL:-e2e-user+${BUILD_VERSION}@example.test}"
E2E_USER_PASSWORD="${E2E_USER_PASSWORD:-e2e-user-${BUILD_VERSION}-password123}"
E2E_ADMIN_EMAIL="${E2E_ADMIN_EMAIL:-e2e-admin+${BUILD_VERSION}@example.test}"
E2E_ADMIN_PASSWORD="${E2E_ADMIN_PASSWORD:-e2e-admin-${BUILD_VERSION}-password123}"
export E2E_USER_EMAIL E2E_USER_PASSWORD E2E_ADMIN_EMAIL E2E_ADMIN_PASSWORD

# Grant admin by env override (backend maps ADMIN_EMAILS -> isAdmin without DB mutation)
export ADMIN_EMAILS="${ADMIN_EMAILS:-}"
if [ -z "${ADMIN_EMAILS}" ]; then
  ADMIN_EMAILS="${E2E_ADMIN_EMAIL}"
else
  ADMIN_EMAILS="${ADMIN_EMAILS},${E2E_ADMIN_EMAIL}"
fi
export ADMIN_EMAILS

BUILD_DIR="_build/${BUILD_VERSION}"
mkdir -p "$BUILD_DIR"
SUMMARY_JSON="$BUILD_DIR/summary.json"
SUMMARY_TXT="$BUILD_DIR/summary.txt"

# VOLUME_PATH is absolute (see scripts/load-env.sh).
mkdir -p "$VOLUME_PATH/surrealdb_data" "$VOLUME_PATH/redis_data" "$VOLUME_PATH/backend_data/contest-images" "$VOLUME_PATH/playwright_jobs"
chmod 777 "$VOLUME_PATH/surrealdb_data" 2>/dev/null || true
chown -R 1000:1000 "$VOLUME_PATH/backend_data" 2>/dev/null || chmod -R a+rwx "$VOLUME_PATH/backend_data" 2>/dev/null || true

# When not iterating/keeping volumes, also clear bind-mounted data dirs.
# docker compose `down -v` does not remove bind-mount contents, and stale SurrealDB root users
# cause auth failures if SURREAL_PASSWORD changes between runs.
if [ "${FULL_PROD_TEST_KEEP_VOLUMES:-0}" != "1" ]; then
  echo "==> Clearing bind-mounted data dirs under $VOLUME_PATH"
  rm -rf "$VOLUME_PATH/surrealdb_data"/* "$VOLUME_PATH/redis_data"/* "$VOLUME_PATH/backend_data"/* 2>/dev/null || true
fi

echo "==> Build version: $BUILD_VERSION (commit $GIT_COMMIT, $BUILD_DATE)"
echo "==> Results will be in $BUILD_DIR"
echo ""

case "${FULL_PROD_TEST_STOP_AFTER:-}" in
  env)
    echo "==> FULL_PROD_TEST_STOP_AFTER=env — exiting before Docker image builds."
    exit 0
    ;;
esac

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

PW_VERSION="$(bash "$SCRIPT_DIR/playwright-version.sh")"
if ! ensure_playwright_image "$PW_VERSION"; then
  exit 1
fi

case "${FULL_PROD_TEST_STOP_AFTER:-}" in
  images|docker-images)
    echo "==> FULL_PROD_TEST_STOP_AFTER=images — exiting before compose stack (Surreal/Redis/backend/frontend)."
    exit 0
    ;;
esac

# --- Phase 3: Data tier → Surreal schema/functions → app tier (backend needs NS/DB + tables) ---
echo "==> [3/7] Starting SurrealDB + Redis, applying schema/functions, then backend + frontend"
export BACKEND_IMAGE="stg-backend:$BUILD_VERSION"
export FRONTEND_IMAGE="stg-frontend:$BUILD_VERSION"
export PLAYWRIGHT_WORKER_IMAGE="stg-playwright:$BUILD_VERSION"
export IMAGE_TAG="$BUILD_VERSION"
# Start stores first so import scripts hit a live server before the backend binary connects.
stg_remove_named_containers
stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d surrealdb redis wait-for-surrealdb

# Wait for each data service on the host (same ports compose publishes)
echo "==> Waiting for SurrealDB on 127.0.0.1:${SURREALDB_PORT}..."
for i in $(seq 1 30); do
  if curl -sf --connect-timeout 2 "http://127.0.0.1:${SURREALDB_PORT}/health" >/dev/null 2>&1; then
    echo "SurrealDB ready."
    break
  fi
  if [ "$i" -eq 30 ]; then
    stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs surrealdb
    stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down
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
    stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs redis
    stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down
    echo "{\"build_version\":\"$BUILD_VERSION\",\"unit\":\"skip\",\"integration\":\"skip\",\"e2e\":\"fail\",\"overall\":\"fail\"}" > "$SUMMARY_JSON"
    echo "FAIL: Redis did not become ready" > "$SUMMARY_TXT"
    exit 1
  fi
  sleep 2
done

echo "==> Applying SurrealDB minimal schema + functions (before backend starts)"
bash "$ROOT/scripts/apply-surreal-schema-minimal.sh" || true
echo "==> Applying SurrealDB migrations (player isActive, etc.)"
ENV_FILE="$ENV_FILE" bash "$ROOT/deploy/run_surreal_migrations.sh"
if ! bash "$ROOT/scripts/apply-surreal-functions.sh"; then
  echo "FAIL: Could not apply SurrealDB functions (required for prod-style tests)." >&2
  stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs surrealdb || true
  stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down || true
  echo "{\"build_version\":\"$BUILD_VERSION\",\"unit\":\"skip\",\"integration\":\"skip\",\"e2e\":\"fail\",\"overall\":\"fail\"}" > "$SUMMARY_JSON"
  echo "FAIL: Could not apply SurrealDB functions" > "$SUMMARY_TXT"
  exit 1
fi

echo "==> Starting backend + frontend"
stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d backend frontend

echo "==> Waiting for backend /health..."
for i in $(seq 1 30); do
  if curl -sf "http://127.0.0.1:${BACKEND_PORT}/health" >/dev/null 2>&1; then
    echo "Backend healthy."
    break
  fi
  if [ "$i" -eq 30 ]; then
    stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs backend
    stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down
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
    stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs frontend
    stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down
    echo "{\"build_version\":\"$BUILD_VERSION\",\"unit\":\"skip\",\"integration\":\"skip\",\"e2e\":\"fail\",\"overall\":\"fail\"}" > "$SUMMARY_JSON"
    echo "FAIL: frontend did not become ready" > "$SUMMARY_TXT"
    exit 1
  fi
  sleep 2
done

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
export PLAYWRIGHT_GLOBAL_TIMEOUT_MS="$PW_GLOBAL"
rm -f "$ROOT/_build/.auth/user.json"

# Preflight: if frontend is down here, Playwright will just burn time on navigation timeouts.
if ! curl -sf --connect-timeout 2 --max-time 5 "$PLAYWRIGHT_BASE_URL" >/dev/null 2>&1; then
  echo "FAIL: Frontend not reachable at $PLAYWRIGHT_BASE_URL right before E2E." | tee "$BUILD_DIR/e2e.log"
  stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" ps | tee -a "$BUILD_DIR/e2e.log" >/dev/null 2>&1 || true
  stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs frontend backend | tee -a "$BUILD_DIR/e2e.log" >/dev/null 2>&1 || true
  E2E_OK=1
fi

if [ "${FULL_PROD_TEST_PLAYWRIGHT_HOST:-0}" = "1" ]; then
  # Host-run Playwright (requires Node + repo npm install / npm ci).
  cd "$ROOT"
  if [ ! -d "node_modules/@playwright/test" ]; then
    stamp "Installing Node deps for Playwright (npm ci in repo root)…"
    npm ci 2>&1 | tee "$BUILD_DIR/e2e.log"
    NPM_OK=${PIPESTATUS[0]:-$?}
    if [ "$NPM_OK" -ne 0 ]; then
      echo "FAIL: npm ci failed." | tee -a "$BUILD_DIR/e2e.log"
      E2E_OK=1
    fi
  fi
  if [ "${E2E_OK:-0}" -eq 0 ]; then
    npm exec -- playwright test --global-timeout="$PW_GLOBAL" 2>&1 | tee "$BUILD_DIR/e2e.log"
    E2E_OK=${PIPESTATUS[0]:-$?}
  fi
else
  # Default: unified Playwright image (build once: ./scripts/build-playwright-image.sh).
  export PLAYWRIGHT_DOCKER_IMAGE="${PLAYWRIGHT_DOCKER_IMAGE:-stg-playwright:$BUILD_VERSION}"
  # Provision per-run E2E users (idempotent: treat "already exists" as OK for iterate runs).
  ensure_e2e_user() {
    local email="$1"
    local password="$2"
    local username="$3"
    local url="http://127.0.0.1:${BACKEND_PORT}/api/players/register"
    local body
    body="$(cat <<EOF
{"username":"${username}","email":"${email}","password":"${password}"}
EOF
)"
    # Capture HTTP status; accept 201 Created or 400 AlreadyExists (backend uses bad_request).
    local code
    code="$(curl -sS -o /dev/null -w '%{http_code}' \
      -H 'Content-Type: application/json' \
      --data-binary "$body" \
      "$url" || echo "000")"
    if [ "$code" = "201" ] || [ "$code" = "400" ]; then
      return 0
    fi
    echo "FAIL: could not provision E2E user ${email} (HTTP ${code})" >&2
    return 1
  }

  export E2E_BACKEND_URL="http://127.0.0.1:${BACKEND_PORT}"
  stamp "Provisioning E2E users (per-run) via backend /api/players/register"
  ensure_e2e_user "$E2E_USER_EMAIL" "$E2E_USER_PASSWORD" "e2e_user_${BUILD_VERSION//[^a-zA-Z0-9_]/_}" || E2E_OK=1
  ensure_e2e_user "$E2E_ADMIN_EMAIL" "$E2E_ADMIN_PASSWORD" "e2e_admin_${BUILD_VERSION//[^a-zA-Z0-9_]/_}" || E2E_OK=1

  if ! docker image inspect "$PLAYWRIGHT_DOCKER_IMAGE" >/dev/null 2>&1; then
    if [ "${FULL_PROD_TEST_BUILD_PLAYWRIGHT_IMAGE:-1}" = "1" ]; then
      stamp "Pre-baked Playwright image missing; building ${PLAYWRIGHT_DOCKER_IMAGE} (CDN access required once)…"
      bash "$SCRIPT_DIR/build-playwright-image.sh"
    else
      echo "FAIL: Playwright image ${PLAYWRIGHT_DOCKER_IMAGE} not found. Run ./scripts/build-playwright-image.sh" >&2
      E2E_OK=1
    fi
  fi
  if [ "${E2E_OK:-0}" -eq 0 ]; then
    stamp "Running Playwright in container (${PLAYWRIGHT_DOCKER_IMAGE})…"
    export PLAYWRIGHT_E2E_LOG="$BUILD_DIR/e2e.log"
    bash "$SCRIPT_DIR/run-playwright-e2e-docker.sh"
    E2E_OK=$?
  fi
fi
set -e
[ "$E2E_OK" -eq 0 ] && E2E_RESULT="pass" || E2E_RESULT="fail"
mkdir -p "$BUILD_DIR/e2e"
[ -d "_build/test-results" ] && cp -r _build/test-results "$BUILD_DIR/e2e/" 2>/dev/null || true
[ -d "_build/playwright-report" ] && cp -r _build/playwright-report "$BUILD_DIR/e2e/" 2>/dev/null || true

# --- Tear down stack ---
echo "==> Tearing down production stack..."
stg_compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down

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
    echo "  - E2E:          PASS — Playwright (Docker by default; see e2e.log)"
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
