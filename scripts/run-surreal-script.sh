#!/bin/bash
# Run a .surql script against SurrealDB at http://localhost:50001 (or SURREAL_VERIFY_URL).
# Usage: ./scripts/run-surreal-script.sh <file.surql>
# Requires: SurrealDB at localhost:50001, curl. Loads SurrealDB vars from config/.env.dev via load-env.sh dev.

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

if [ -z "$1" ] || [ ! -f "$1" ]; then
  echo "Usage: $0 <file.surql>" >&2
  echo "Example: $0 docs/verify-surreal-contest-list.surql" >&2
  exit 1
fi

source "$SCRIPT_DIR/load-env.sh" dev

SURREAL_NS="${SURREAL_NS:-stg_rd}"
SURREAL_DB="${SURREAL_DB:-stg_rd}"
SURREAL_USER="${SURREAL_USER:-root}"
SURREAL_PASSWORD="${SURREAL_PASSWORD:-root}"
PORT="${SURREALDB_PORT:-50001}"
HTTP_URL="${SURREAL_VERIFY_URL:-http://127.0.0.1:${PORT}}"

run_one() {
  local q="$1"
  q="$(echo "$q" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  [ -z "$q" ] && return 0
  # Prefix USE so ns/db are set in request body (some SurrealDB setups ignore NS/DB headers)
  local body="USE NS $SURREAL_NS; USE DB $SURREAL_DB; $q"
  curl -s --connect-timeout 5 --max-time 15 -X POST \
    -H "Accept: application/json" \
    -u "$SURREAL_USER:$SURREAL_PASSWORD" \
    --data "$body" \
    "$HTTP_URL/sql" 2>/dev/null || echo '{"error":"request failed"}'
}

echo "==> Running $1 against $HTTP_URL (ns=$SURREAL_NS db=$SURREAL_DB)"
echo "    (Ensure SurrealDB is running, e.g. ./scripts/start-back.sh)"
echo ""

# Strip full-line -- comments and blank lines; split on ; and run each statement
while IFS= read -r line; do
  [[ "$line" =~ ^[[:space:]]*-- ]] && continue
  buf="${buf:-}${buf:+ }$line"
  if [[ "$buf" = *";"* ]]; then
    # Emit up to and including the first ;
    while [[ "$buf" = *";"* ]]; do
      stmt="${buf%%;*}"
      buf="${buf#*;}"
      buf="$(echo "$buf" | sed 's/^[[:space:]]*//')"
      stmt="$(echo "$stmt" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
      [ -z "$stmt" ] && continue
      echo ">>> $stmt"
      result="$(run_one "$stmt")"
      echo "$result" | jq -c '.' 2>/dev/null || echo "$result"
      echo ""
    done
  fi
done < "$1"
# Run any remainder
stmt="$(echo "$buf" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
if [ -n "$stmt" ]; then
  echo ">>> $stmt"
  result="$(run_one "$stmt")"
  echo "$result" | jq -c '.' 2>/dev/null || echo "$result"
fi
