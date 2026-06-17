#!/usr/bin/env bash
# Run Playwright E2E using the unified stg-playwright image (no runtime browser download).
#
# Build the image once (needs network): ./scripts/build-playwright-image.sh
#
# Targets the stack on the host (default: --network host).
#
# Env (optional):
#   PLAYWRIGHT_DOCKER_IMAGE       (default: stg-playwright:latest)
#   PLAYWRIGHT_BASE_URL           (default: http://127.0.0.1:${FRONTEND_PORT:-50003})
#   PLAYWRIGHT_GLOBAL_TIMEOUT_MS
#   PLAYWRIGHT_E2E_LOG            if set, tee stdout/stderr to this file
#   PLAYWRIGHT_DOCKER_NETWORK     "host" (default on Linux) or "bridge"
#   FULL_PROD_TEST_BUILD_PLAYWRIGHT_IMAGE=1  build image if missing before run
#
# Usage (from repo root):
#   source scripts/load-env.sh prod
#   export USE_PRODUCTION_CONTAINERS=1
#   ./scripts/run-playwright-e2e-docker.sh
#   ./scripts/run-playwright-e2e-docker.sh testing/e2e/auth.spec.ts --retries=0
#   ./scripts/run-playwright-e2e-docker.sh -g "should allow user to login" --project=chromium

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

IMAGE="${PLAYWRIGHT_DOCKER_IMAGE:-stg-playwright:latest}"
NETWORK="${PLAYWRIGHT_DOCKER_NETWORK:-host}"
PW_GLOBAL="${PLAYWRIGHT_GLOBAL_TIMEOUT_MS:-7200000}"
BASE_URL="${PLAYWRIGHT_BASE_URL:-http://127.0.0.1:${FRONTEND_PORT:-50003}}"

if ! command -v docker >/dev/null 2>&1; then
  echo "docker not found; install Docker or set FULL_PROD_TEST_PLAYWRIGHT_HOST=1 for host Playwright." >&2
  exit 1
fi

if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  if [ "${FULL_PROD_TEST_BUILD_PLAYWRIGHT_IMAGE:-0}" = "1" ]; then
    echo "==> Playwright image $IMAGE not found; building…"
    bash "$SCRIPT_DIR/build-playwright-image.sh"
    IMAGE="${PLAYWRIGHT_DOCKER_IMAGE:-stg-playwright:latest}"
  else
    echo "Playwright image not found: $IMAGE" >&2
    echo "Build once (requires network to npm + Playwright CDN):" >&2
    echo "  ./scripts/build-playwright-image.sh" >&2
    echo "Or: FULL_PROD_TEST_BUILD_PLAYWRIGHT_IMAGE=1 $0" >&2
    exit 1
  fi
fi

mkdir -p "$ROOT/_build/test-results" "$ROOT/_build/playwright-report"

run_docker() {
  local -a args=(docker run --rm)
  if [ "$NETWORK" = "host" ]; then
    args+=(--network host)
  else
    args+=(--add-host=host.docker.internal:host-gateway)
  fi
  args+=(
    -v "$ROOT/testing/e2e:/app/testing/e2e:ro"
    -v "$ROOT/playwright.config.ts:/app/playwright.config.ts:ro"
    -v "$ROOT/_build:/app/_build"
    -e USE_PRODUCTION_CONTAINERS="${USE_PRODUCTION_CONTAINERS:-1}"
    -e PLAYWRIGHT_BASE_URL="$BASE_URL"
    -e CI=1
    -e PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1
    -e E2E_USER_EMAIL="${E2E_USER_EMAIL:-}"
    -e E2E_USER_PASSWORD="${E2E_USER_PASSWORD:-}"
    -e E2E_ADMIN_EMAIL="${E2E_ADMIN_EMAIL:-}"
    -e E2E_ADMIN_PASSWORD="${E2E_ADMIN_PASSWORD:-}"
    -e E2E_BACKEND_URL="${E2E_BACKEND_URL:-http://127.0.0.1:${BACKEND_PORT:-50002}}"
    "$IMAGE"
    bash -lc 'set -euo pipefail; cd /app; mkdir -p _build/test-results _build/playwright-report; exec npm exec -- playwright test --global-timeout='"$PW_GLOBAL"' "$@"' \
      _ "$@"
  )
  "${args[@]}"
}

if [ -n "${PLAYWRIGHT_E2E_LOG:-}" ]; then
  mkdir -p "$(dirname "$PLAYWRIGHT_E2E_LOG")"
  run_docker "$@" 2>&1 | tee "$PLAYWRIGHT_E2E_LOG"
else
  run_docker "$@"
fi
