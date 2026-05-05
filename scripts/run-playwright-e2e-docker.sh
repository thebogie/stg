#!/usr/bin/env bash
# Run Playwright E2E inside Microsoft's Playwright Docker image (Node + browsers).
# No host Node/npm/playwright install required. Targets the stack on the host (default: --network host).
#
# After npm install we run `npm exec playwright install` so browser binaries match the repo's @playwright/test
# (the image alone can skew vs package-lock and cause "Executable doesn't exist at .../chromium_headless_shell-...").
#
# Env (optional):
#   PLAYWRIGHT_DOCKER_IMAGE   (default: mcr.microsoft.com/playwright:v1.49.1-noble)
#   PLAYWRIGHT_BASE_URL       (default: http://127.0.0.1:${FRONTEND_PORT:-50003})
#   PLAYWRIGHT_GLOBAL_TIMEOUT_MS
#   PLAYWRIGHT_E2E_LOG        if set, tee stdout/stderr to this file
#   PLAYWRIGHT_DOCKER_NETWORK  "host" (default on Linux) or "bridge" — if bridge, set
#                              PLAYWRIGHT_BASE_URL=http://host.docker.internal:PORT (Docker Desktop)
#
# Usage (from repo root):
#   source scripts/load-env.sh prod
#   export USE_PRODUCTION_CONTAINERS=1
#   ./scripts/run-playwright-e2e-docker.sh

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

IMAGE="${PLAYWRIGHT_DOCKER_IMAGE:-mcr.microsoft.com/playwright:v1.49.1-noble}"
NETWORK="${PLAYWRIGHT_DOCKER_NETWORK:-host}"
PW_GLOBAL="${PLAYWRIGHT_GLOBAL_TIMEOUT_MS:-7200000}"
BASE_URL="${PLAYWRIGHT_BASE_URL:-http://127.0.0.1:${FRONTEND_PORT:-50003}}"

if ! command -v docker >/dev/null 2>&1; then
  echo "docker not found; install Docker or set FULL_PROD_TEST_PLAYWRIGHT_HOST=1 for host Playwright." >&2
  exit 1
fi

run_docker() {
  local -a args=(docker run --rm)
  if [ "$NETWORK" = "host" ]; then
    args+=(--network host)
  else
    args+=(--add-host=host.docker.internal:host-gateway)
  fi
  args+=(
    -v "$ROOT:/work"
    -w /work
    -e USE_PRODUCTION_CONTAINERS="${USE_PRODUCTION_CONTAINERS:-1}"
    -e PLAYWRIGHT_BASE_URL="$BASE_URL"
    -e CI=1
    "$IMAGE"
    bash -lc "set -euo pipefail; npm install; npm exec playwright install; mkdir -p _build/test-results _build/playwright-report; exec npm exec playwright test --global-timeout=$PW_GLOBAL"
  )
  "${args[@]}"
}

if [ -n "${PLAYWRIGHT_E2E_LOG:-}" ]; then
  mkdir -p "$(dirname "$PLAYWRIGHT_E2E_LOG")"
  run_docker 2>&1 | tee "$PLAYWRIGHT_E2E_LOG"
else
  run_docker
fi
