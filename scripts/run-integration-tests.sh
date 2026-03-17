#!/usr/bin/env bash
# Run integration tests (including ignored auth tests) against the stack.
# Uses host-accessible URLs (127.0.0.1) so tests don't hit "name resolution" when
# .env.prod or env has Docker-internal hostnames (e.g. surrealdb).
#
# Usage: ./scripts/run-integration-tests.sh [-- test args...]
#         INTEGRATION_TEST_LOG=/tmp/api_tests.log ./scripts/run-integration-tests.sh -- --test api_tests --test-threads=1 --nocapture
#         (--test api_tests = run that binary only; without it a name filter can cause "7 filtered out" and 0 tests run)
#
# Example: ./scripts/run-integration-tests.sh -- --include-ignored --test-threads=1
# Example: ./scripts/run-integration-tests.sh -- --test api_tests --test-threads=1 --nocapture
# Example: INTEGRATION_TEST_LOG=/tmp/api_tests.log ./scripts/run-integration-tests.sh -- --test api_tests --nocapture
#
# Requires: stack up (docker compose -f deploy/docker-compose.yml --env-file config/.env.prod up -d)

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

source "$SCRIPT_DIR/load-env.sh" prod

# Force host-accessible URLs (tests run on host, not inside Docker).
# Unset first so we override any Docker-internal URL from parent env.
unset SURREAL_URL
unset REDIS_URL
export SURREAL_URL="http://127.0.0.1:${SURREALDB_PORT:-50001}"
export REDIS_URL="redis://127.0.0.1:${REDIS_PORT:-6379}/"
export SURREAL_NS="${SURREAL_NS:-stg_rd}"
export SURREAL_DB="${SURREAL_DB:-stg_rd}"
export SURREAL_USER="${SURREAL_USER:-root}"
export SURREAL_PASSWORD="${SURREAL_PASSWORD:-root}"

echo "==> Integration tests (SURREAL_URL=$SURREAL_URL REDIS_URL=$REDIS_URL)"
echo "    If tests hang or 'name resolution' fails, this URL must be reachable from this machine."

if [ -n "${INTEGRATION_TEST_LOG:-}" ]; then
  echo "    Logging to $INTEGRATION_TEST_LOG"
  cargo test -p testing "$@" 2>&1 | tee "$INTEGRATION_TEST_LOG"
else
  cargo test -p testing "$@"
fi
