#!/bin/bash
# Apply docs/surreal-functions.surql to SurrealDB (same NS/DB as start-deps).
# Run after SurrealDB is up (e.g. ./scripts/start-deps.sh). Shows any import errors.
# Usage: ./scripts/apply-surreal-functions.sh

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

source "$SCRIPT_DIR/load-env.sh" dev
SURREAL_NS="${SURREAL_NS:-stg_rd}"
SURREAL_DB="${SURREAL_DB:-stg_rd}"
SURREAL_USER="${SURREAL_USER:-root}"
SURREAL_PASSWORD="${SURREAL_PASSWORD:-root}"
SURREALDB_PORT="${SURREALDB_PORT:-50001}"
ENDPOINT="http://127.0.0.1:${SURREALDB_PORT}"
FUNCTIONS_FILE="$ROOT/docs/surreal-functions.surql"

if [ ! -f "$FUNCTIONS_FILE" ]; then
  echo "Missing $FUNCTIONS_FILE" >&2
  exit 1
fi

echo "==> Applying SurrealDB functions to ns=$SURREAL_NS db=$SURREAL_DB (conn: $ENDPOINT)"
docker run --rm --network host \
  -v "$ROOT/docs:/import:ro" \
  surrealdb/surrealdb:v3 \
  import \
  --endpoint "$ENDPOINT" \
  --username "$SURREAL_USER" --password "$SURREAL_PASSWORD" \
  --namespace "$SURREAL_NS" --database "$SURREAL_DB" \
  "/import/surreal-functions.surql"

echo "==> Done. Test with: SELECT fn::contest_row(\"YOUR_CONTEST_KEY\") AS result FROM [1];"
