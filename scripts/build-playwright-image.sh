#!/usr/bin/env bash
# Build the unified STG Playwright image (worker daemon + E2E test runner).
#
# Usage (from repo root):
#   ./scripts/build-playwright-image.sh
#
# Env:
#   PLAYWRIGHT_IMAGE_NAME   default: stg-playwright
#   PLAYWRIGHT_IMAGE_TAG    default: local (also tags :latest)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

IMAGE_NAME="${PLAYWRIGHT_IMAGE_NAME:-stg-playwright}"
TAG="${PLAYWRIGHT_IMAGE_TAG:-local}"
PW_VERSION="$(bash "$SCRIPT_DIR/playwright-version.sh")"

echo "==> Building ${IMAGE_NAME}:${TAG} (Playwright ${PW_VERSION}, worker + E2E)"
docker build -f deploy/Dockerfile.playwright \
  --build-arg "PLAYWRIGHT_VERSION=${PW_VERSION}" \
  -t "${IMAGE_NAME}:${TAG}" \
  -t "${IMAGE_NAME}:latest" \
  "$ROOT"

# Back-compat aliases (same image, old names).
docker tag "${IMAGE_NAME}:${TAG}" "stg-playwright-worker:${TAG}"
docker tag "${IMAGE_NAME}:latest" "stg-playwright-worker:latest"
docker tag "${IMAGE_NAME}:${TAG}" "stg-playwright-e2e:${TAG}"
docker tag "${IMAGE_NAME}:latest" "stg-playwright-e2e:latest"

echo ""
echo "==> Done: ${IMAGE_NAME}:${TAG} and ${IMAGE_NAME}:latest"
echo "    Production worker: PLAYWRIGHT_WORKER_IMAGE=${IMAGE_NAME}:${TAG}"
echo "    CI E2E:            PLAYWRIGHT_DOCKER_IMAGE=${IMAGE_NAME}:latest ./scripts/run-playwright-e2e-docker.sh"
echo "    Rebuild when package-lock.json @playwright/test version changes."
