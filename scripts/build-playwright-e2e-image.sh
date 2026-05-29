#!/usr/bin/env bash
# Build the pre-baked Playwright E2E Docker image (npm ci + chromium at build time).
#
# Requires outbound network once (npm registry + Playwright browser CDN). After this,
# ./scripts/run-playwright-e2e-docker.sh and ./scripts/full-prod-test.sh do not download browsers.
#
# Usage (from repo root):
#   ./scripts/build-playwright-e2e-image.sh
#
# Env:
#   PLAYWRIGHT_E2E_IMAGE_NAME   default: stg-playwright-e2e
#   PLAYWRIGHT_E2E_IMAGE_TAG    default: @playwright/test version from package-lock.json

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

IMAGE_NAME="${PLAYWRIGHT_E2E_IMAGE_NAME:-stg-playwright-e2e}"

if command -v node >/dev/null 2>&1; then
  PW_VERSION="$(node -e "
    const lock = require('./package-lock.json');
    const p = lock.packages && lock.packages['node_modules/@playwright/test'];
    if (p && p.version) process.stdout.write(p.version);
  " 2>/dev/null || true)"
fi
if [ -z "${PW_VERSION:-}" ]; then
  PW_VERSION="$(grep -A3 '"node_modules/@playwright/test"' package-lock.json \
    | grep '"version"' | head -1 | sed -E 's/.*"version": "([^"]+)".*/\1/')"
fi
if [ -z "${PW_VERSION:-}" ]; then
  echo "Could not determine @playwright/test version from package-lock.json" >&2
  exit 1
fi

TAG="${PLAYWRIGHT_E2E_IMAGE_TAG:-$PW_VERSION}"

echo "==> Building ${IMAGE_NAME}:${TAG} (Playwright ${PW_VERSION}, npm ci + chromium)"
docker build -f deploy/Dockerfile.playwright-e2e \
  --build-arg "PLAYWRIGHT_VERSION=${PW_VERSION}" \
  -t "${IMAGE_NAME}:${TAG}" \
  -t "${IMAGE_NAME}:latest" \
  "$ROOT"

echo ""
echo "==> Done: ${IMAGE_NAME}:${TAG} and ${IMAGE_NAME}:latest"
echo "    Run E2E: PLAYWRIGHT_DOCKER_IMAGE=${IMAGE_NAME}:latest ./scripts/run-playwright-e2e-docker.sh"
echo "    Rebuild when package-lock.json @playwright/test version changes."
