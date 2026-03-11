#!/usr/bin/env bash
# Run full CI locally (build, unit, integration, e2e). Wrapper for scripts/ci.sh.
exec "$(dirname "$0")/scripts/ci.sh" "$@"
