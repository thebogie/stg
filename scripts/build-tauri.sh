#!/bin/bash
# Build STG Tauri desktop installers (.deb on Linux).
#
# Usage:
#   ./scripts/build-tauri.sh              # prod API, .deb only
#   ./scripts/build-tauri.sh dev          # local API default in debug builds
#   ./scripts/build-tauri.sh prod deb,rpm # custom bundle types (avoid appimage in CI)
#
# Output (workspace target-dir): _build/target/release/bundle/
# Requires: cargo install tauri-cli --version "^2.0.0"
# Linux deps: libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
TAURI_DIR="$ROOT/front/tauri"
WEB_DIR="$ROOT/front/web"

ENV_ARG="${1:-prod}"
shift || true
BUNDLES="${1:-deb}"

if [[ "$ENV_ARG" == "dev" ]]; then
  source "$SCRIPT_DIR/load-env.sh" dev
  export STG_API_URL="${STG_API_URL:-http://127.0.0.1:${BACKEND_PORT}}"
else
  source "$SCRIPT_DIR/load-env.sh" prod
  export STG_API_URL="${STG_API_URL:-https://smacktalkgaming.com}"
fi

if ! cargo tauri --version >/dev/null 2>&1; then
  echo "Error: install Tauri CLI: cargo install tauri-cli --version \"^2.0.0\"" >&2
  exit 1
fi

if [[ "${SKIP_TAILWIND:-0}" != "1" ]] && command -v npm >/dev/null 2>&1 && [[ -f "$WEB_DIR/package.json" ]]; then
  echo "==> Tailwind (SKIP_TAILWIND=1 to skip)"
  (cd "$WEB_DIR" && npm run -s build:css:prod)
fi

export GIT_COMMIT="${GIT_COMMIT:-$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo local)}"
export BUILD_DATE="${BUILD_DATE:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}"

echo "==> Tauri build bundles=$BUNDLES STG_API_URL=$STG_API_URL"
cd "$TAURI_DIR"
cargo tauri build --bundles "$BUNDLES"

OUT="$ROOT/_build/target/release/bundle"
echo ""
echo "==> Done. Installers under: $OUT"
find "$OUT" -type f \( -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' \) 2>/dev/null || true
