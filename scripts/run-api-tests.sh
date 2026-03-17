#!/usr/bin/env bash
# Run only the api_tests (7 player tests) and write output to /tmp/api_tests.log.
# Use this to get a useful log with pass/fail and any failure messages.
#
# Usage: ./scripts/run-api-tests.sh
# Then:  cat /tmp/api_tests.log   (or open in editor)
#
# Requires: stack up (SurrealDB + Redis)

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"
LOG_FILE="/tmp/api_tests.log"

source "$SCRIPT_DIR/load-env.sh" prod
unset SURREAL_URL REDIS_URL
export SURREAL_URL="http://127.0.0.1:${SURREALDB_PORT:-50001}"
export REDIS_URL="redis://127.0.0.1:${REDIS_PORT:-6379}/"
export SURREAL_NS="${SURREAL_NS:-stg_rd}"
export SURREAL_DB="${SURREAL_DB:-stg_rd}"
export SURREAL_USER="${SURREAL_USER:-root}"
export SURREAL_PASSWORD="${SURREAL_PASSWORD:-root}"

echo "==> Running api_tests (7 tests), logging to $LOG_FILE"
echo "    (If you see 'name resolution' failure, run this script from a system terminal, not the IDE.)"
cargo test -p testing --test api_tests -- --test-threads=1 --nocapture 2>&1 | tee "$LOG_FILE"
echo ""
echo "==> Log written to $LOG_FILE (failures: search for 'FAILED' or 'failures:')"
