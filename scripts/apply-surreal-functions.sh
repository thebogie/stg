#!/bin/bash
# Apply tools/arango-to-surreal/surreal-functions.surql to SurrealDB (same NS/DB as start-deps).
# Run after SurrealDB is up (e.g. ./scripts/start-deps.sh). Shows any import errors.
# Usage: ./scripts/apply-surreal-functions.sh

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

# Respect the caller's environment (full-prod-test / ci.sh already source the right env).
# Only fall back to dev defaults if the key Surreal vars are not set.
if [ -z "${SURREALDB_PORT+set}" ] || [ -z "${SURREAL_PASSWORD+set}" ] || [ -z "${SURREAL_USER+set}" ]; then
  source "$SCRIPT_DIR/load-env.sh" "${ENV:-dev}"
fi
SURREAL_NS="${SURREAL_NS:-stg_rd}"
SURREAL_DB="${SURREAL_DB:-stg_rd}"
SURREAL_USER="${SURREAL_USER:-root}"
SURREAL_PASSWORD="${SURREAL_PASSWORD:-root}"
SURREALDB_PORT="${SURREALDB_PORT:-50001}"
ENDPOINT="http://127.0.0.1:${SURREALDB_PORT}"
FUNCTIONS_FILE="$ROOT/tools/arango-to-surreal/surreal-functions.surql"
REMOVE_FILE="$ROOT/tools/arango-to-surreal/surreal-functions-remove.surql"

if [ ! -f "$FUNCTIONS_FILE" ]; then
  echo "Missing $FUNCTIONS_FILE" >&2
  exit 1
fi

echo "==> Applying SurrealDB functions to ns=$SURREAL_NS db=$SURREAL_DB (conn: $ENDPOINT)"

# Remove existing functions so re-runs (e.g. full-prod-test with persistent bind mount) are idempotent.
if [ -f "$REMOVE_FILE" ]; then
  echo "==> Removing existing application functions (if any)..."
  docker run --rm --network host \
    -v "$ROOT/tools/arango-to-surreal:/import:ro" \
    surrealdb/surrealdb:v3 \
    import \
    --endpoint "$ENDPOINT" \
    --username "$SURREAL_USER" --password "$SURREAL_PASSWORD" \
    --namespace "$SURREAL_NS" --database "$SURREAL_DB" \
    "/import/surreal-functions-remove.surql" 2>/dev/null || true
fi

docker run --rm --network host \
  -v "$ROOT/tools/arango-to-surreal:/import:ro" \
  surrealdb/surrealdb:v3 \
  import \
  --endpoint "$ENDPOINT" \
  --username "$SURREAL_USER" --password "$SURREAL_PASSWORD" \
  --namespace "$SURREAL_NS" --database "$SURREAL_DB" \
  "/import/surreal-functions.surql"

echo "==> Done. Test with: SELECT fn::contest_row(\"YOUR_CONTEST_KEY\") AS result FROM [1];"
