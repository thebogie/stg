#!/bin/bash
# Build STG Android APK (universal release) for sideload / Drive install.
#
# Usage:
#   ./scripts/build-tauri-android.sh
#   STG_API_URL=https://smacktalkgaming.com ./scripts/build-tauri-android.sh
#
# Prereqs: Java 17, Android SDK/NDK, rustup Android targets.
# One-time: cd front/tauri && cargo tauri android init
# APK output: front/tauri/src-tauri/gen/android/app/build/outputs/apk/

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
TAURI_DIR="$ROOT/front/tauri"
WEB_DIR="$ROOT/front/web"
ANDROID_DIR="$TAURI_DIR/src-tauri/gen/android"

export STG_API_URL="${STG_API_URL:-https://smacktalkgaming.com}"

if [[ ! -f "$ANDROID_DIR/gradlew" ]]; then
  echo "Error: Android project missing. Run once:" >&2
  echo "  cd front/tauri && cargo tauri android init" >&2
  exit 1
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

echo "==> Tauri Android build STG_API_URL=$STG_API_URL"
cd "$TAURI_DIR"
cargo tauri android build

echo ""
echo "==> APK(s):"
find "$ANDROID_DIR/app/build/outputs/apk" -name '*.apk' 2>/dev/null || true
