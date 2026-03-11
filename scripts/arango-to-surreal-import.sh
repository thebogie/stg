#!/bin/bash
# One-shot production migration: convert ArangoDB backup zip to .surql and import into SurrealDB.
# Always uses --remap-all-ids (new UUID per document/edge, relationships preserved) and --fresh (reset ns/db before import).
# Usage: ./scripts/arango-to-surreal-import.sh [path/to/smacktalk.zip] [--no-schema]
#   Zip path: optional if ARANGO_BACKUP_ZIP is set.
#   --no-schema: converter emits INSERTs only (no DEFINE); use if DB already has schema.
# Default: converter emits full schema (DEFINE TABLE/FIELD/INDEX) + data for one-time production migration.
# Requires: SurrealDB listening at localhost:${SURREALDB_PORT:-50001} (e.g. from start-back.sh or Surrealist).
# Uses Docker for surreal import (no local surreal CLI required). Loads config via load-env.sh dev.

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

source "$SCRIPT_DIR/load-env.sh" dev

# Parse args
BACKUP_ZIP=""
NO_SCHEMA=0
for a in "$@"; do
  if [ "$a" = "--no-schema" ]; then NO_SCHEMA=1;
  elif [ -z "$BACKUP_ZIP" ] && [ -f "$a" ]; then BACKUP_ZIP="$a"; fi
done
if [ -z "$BACKUP_ZIP" ]; then
  BACKUP_ZIP="${ARANGO_BACKUP_ZIP:-$HOME/work/_backups/smacktalk.zip}"
fi
if [ ! -f "$BACKUP_ZIP" ]; then
  echo "Error: Backup zip not found: $BACKUP_ZIP" >&2
  echo "Usage: $0 [path/to/smacktalk.zip] [--no-schema]" >&2
  exit 1
fi

SURQL_PATH="${SURREAL_IMPORT_SURQL:-$ROOT/_build/smacktalk.surql}"
SURREAL_NS="${SURREAL_NS:-stg_rd}"
SURREAL_DB="${SURREAL_DB:-stg_rd}"
SURREAL_USER="${SURREAL_USER:-root}"
SURREAL_PASSWORD="${SURREAL_PASSWORD:-root}"
PORT="${SURREALDB_PORT:-50001}"
CONN="http://host.docker.internal:${PORT}"

mkdir -p "$(dirname "$SURQL_PATH")"

echo "==> Converting $BACKUP_ZIP to .surql (full schema + data, --remap-all-ids) ..."
CONVERT_ARGS=(-o "$SURQL_PATH" --remap-all-ids)
if [ "$NO_SCHEMA" = "1" ]; then
  CONVERT_ARGS+=(--no-schema)
  echo "     (--no-schema: INSERTs only)"
fi
if ! cargo run -p arango-to-surreal -- "$BACKUP_ZIP" "${CONVERT_ARGS[@]}"; then
  echo "Error: Conversion failed." >&2
  exit 1
fi

echo "==> Resetting SurrealDB namespace/database at $CONN (--fresh) ..."
FRESH_SQL="DEFINE NAMESPACE ${SURREAL_NS}; USE NS ${SURREAL_NS}; REMOVE DATABASE ${SURREAL_DB}; DEFINE DATABASE ${SURREAL_DB};"
if ! echo "$FRESH_SQL" | docker run -i --rm --add-host=host.docker.internal:host-gateway \
  surrealdb/surrealdb:v2 sql \
  --conn "$CONN" --user "$SURREAL_USER" --pass "$SURREAL_PASSWORD" \
  --ns "$SURREAL_NS" --db "$SURREAL_DB" \
  --hide-welcome 2>/dev/null; then
  echo "Warning: Fresh reset failed (namespace may not have existed). Trying import anyway." >&2
fi

echo "==> Importing $SURQL_PATH into $CONN (ns=$SURREAL_NS db=$SURREAL_DB) ..."
SURQL_DIR="$(cd "$(dirname "$SURQL_PATH")" && pwd)"
SURQL_FILE="$(basename "$SURQL_PATH")"
if docker run --rm --add-host=host.docker.internal:host-gateway \
  -v "$SURQL_DIR:/import:ro" \
  surrealdb/surrealdb:v2 import \
  --conn "$CONN" --user "$SURREAL_USER" --pass "$SURREAL_PASSWORD" \
  --ns "$SURREAL_NS" --db "$SURREAL_DB" \
  "/import/$SURQL_FILE"; then
  echo "==> Import completed. SurrealDB at http://localhost:${PORT}"
else
  echo "Error: Import failed. Is SurrealDB running at http://localhost:${PORT}?" >&2
  exit 1
fi
