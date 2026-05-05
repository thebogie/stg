#!/usr/bin/env bash
# Thin wrapper: ./ci-local.sh [build|unit|smoke|integration|e2e|all] [dev|prod]
#   smoke — fast prod-like check: build + unit + stack + api_tests + one ignored backend smoke test, then down
set -e
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$ROOT/scripts/ci.sh" "$@"
