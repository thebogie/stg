#!/usr/bin/env bash
# Production: pull GHCR images tagged in CI and restart the full stack (same images the gate exercises).
# Run on the server from a checkout that includes deploy/, or set DEPLOY_ROOT to your copied deploy/ tree.
#
# Usage: ./scripts/install-from-ci.sh <tag>
#   <tag> = image label from GitHub Actions (e.g. latest, or short sha from the build)
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEPLOY_ROOT="${DEPLOY_ROOT:-$ROOT/deploy}"
if [ ! -x "$DEPLOY_ROOT/deploy_stg.sh" ]; then
  echo "Missing executable $DEPLOY_ROOT/deploy_stg.sh" >&2
  echo "Set DEPLOY_ROOT to the directory that contains deploy_stg.sh (usually .../stg/deploy)." >&2
  exit 1
fi
exec "$DEPLOY_ROOT/deploy_stg.sh" "$@"
