#!/usr/bin/env bash
# Print @playwright/test version from package-lock.json (matches mcr.microsoft.com/playwright image tag).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if command -v node >/dev/null 2>&1; then
  PW_VERSION="$(node -e "
    const lock = require('${ROOT}/package-lock.json');
    const p = lock.packages && lock.packages['node_modules/@playwright/test'];
    if (p && p.version) process.stdout.write(p.version);
  " 2>/dev/null || true)"
fi

if [ -z "${PW_VERSION:-}" ]; then
  PW_VERSION="$(grep -A3 '"node_modules/@playwright/test"' "$ROOT/package-lock.json" \
    | grep '"version"' | head -1 | sed -E 's/.*"version": "([^"]+)".*/\1/')"
fi

if [ -z "${PW_VERSION:-}" ]; then
  echo "Could not determine @playwright/test version from package-lock.json" >&2
  exit 1
fi

printf '%s' "$PW_VERSION"
