#!/usr/bin/env bash
# Fast parse check for deploy-gate shell scripts (no Docker, no cargo, no tests).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

FILES=(
  scripts/full-prod-test.sh
  scripts/test-prod-gate.sh
  scripts/build-playwright-e2e-image.sh
  scripts/run-playwright-e2e-docker.sh
  scripts/load-env.sh
  scripts/apply-surreal-schema-minimal.sh
  scripts/apply-surreal-functions.sh
)
for rel in "${FILES[@]}"; do
  f="$ROOT/$rel"
  if [ ! -f "$f" ]; then
    echo "skip missing $rel" >&2
    continue
  fi
  bash -n "$f"
  echo "OK  $rel"
done
echo "All listed scripts pass bash -n."
