#!/usr/bin/env bash
# Test SurrealDB auth: try root/root and root/<password from .env.prod>.
# Use to confirm which credentials the running container accepts (compose uses .env.prod).
# Usage: ./scripts/test-surrealdb-auth.sh
# Requires: stack up, config/.env.prod (source scripts/load-env.sh prod first if not in same shell)

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

# Load .env.prod so we have SURREAL_PASSWORD (and ports/ns/db)
if [ -z "${SURREAL_PASSWORD+set}" ] || [ -z "${SURREALDB_PORT+set}" ]; then
  source "$SCRIPT_DIR/load-env.sh" prod
fi

SURREAL_NS="${SURREAL_NS:-stg_rd}"
SURREAL_DB="${SURREAL_DB:-stg_rd}"
SURREALDB_PORT="${SURREALDB_PORT:-50001}"
ENDPOINT="http://127.0.0.1:${SURREALDB_PORT}"
ENV_PASSWORD="${SURREAL_PASSWORD:-}"

# Use a repo file so the container can read it (avoid Permission denied on temp mounts)
SQL_FILE="$ROOT/docs/surreal-select1.surql"
TMP_ERR="$(mktemp)"
trap 'rm -f "$TMP_ERR"' EXIT
if [ ! -f "$SQL_FILE" ]; then
  echo "Missing $SQL_FILE" >&2
  exit 1
fi

try_auth() {
  local user="$1"
  local pass="$2"
  docker run --rm --network host \
    -v "$ROOT/docs:/import:ro" \
    surrealdb/surrealdb:v3 \
    import \
    --endpoint "$ENDPOINT" \
    --username "$user" --password "$pass" \
    --namespace "$SURREAL_NS" --database "$SURREAL_DB" \
    /import/surreal-select1.surql >/dev/null 2>"$TMP_ERR"
}

show_fail_reason() {
  if [ -s "$TMP_ERR" ]; then
    echo "    Error output:"
    sed 's/^/      /' < "$TMP_ERR"
  fi
}

echo "SurrealDB auth check (endpoint=$ENDPOINT, ns=$SURREAL_NS, db=$SURREAL_DB)"
echo ""

if try_auth "root" "root"; then
  echo "  root/root: OK"
else
  echo "  root/root: FAIL"
  show_fail_reason
fi

if [ -n "$ENV_PASSWORD" ]; then
  if try_auth "root" "$ENV_PASSWORD"; then
    echo "  root/<.env.prod password>: OK"
  else
    echo "  root/<.env.prod password>: FAIL"
    show_fail_reason
  fi
else
  echo "  root/<.env.prod password>: (none set in .env.prod)"
fi

echo ""
echo "Compose uses SURREAL_USER/SURREAL_PASSWORD from .env.prod; only that password will work after first run."
