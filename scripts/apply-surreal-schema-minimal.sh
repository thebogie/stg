#!/usr/bin/env bash
# Apply minimal SurrealDB schema (player table) so integration tests can run without full production data.
# Use when the DB is empty or fresh. Safe to run after stack is up.
# Usage: source scripts/load-env.sh prod && ./scripts/apply-surreal-schema-minimal.sh

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

SURREAL_NS="${SURREAL_NS:-stg_rd}"
SURREAL_DB="${SURREAL_DB:-stg_rd}"
SURREAL_USER="${SURREAL_USER:-root}"
SURREAL_PASSWORD="${SURREAL_PASSWORD:-root}"
SURREALDB_PORT="${SURREALDB_PORT:-50001}"
ENDPOINT="http://127.0.0.1:${SURREALDB_PORT}"
SCHEMA_FILE="$ROOT/docs/surreal-schema-minimal-tests.surql"

if [ ! -f "$SCHEMA_FILE" ]; then
  echo "Missing $SCHEMA_FILE" >&2
  exit 1
fi

echo "==> Applying minimal schema (player table) to ns=$SURREAL_NS db=$SURREAL_DB"
docker run --rm --network host \
  -v "$ROOT/docs:/import:ro" \
  surrealdb/surrealdb:v3 \
  import \
  --endpoint "$ENDPOINT" \
  --username "$SURREAL_USER" --password "$SURREAL_PASSWORD" \
  --namespace "$SURREAL_NS" --database "$SURREAL_DB" \
  "/import/surreal-schema-minimal-tests.surql"

echo "==> Done (minimal schema for tests)."
