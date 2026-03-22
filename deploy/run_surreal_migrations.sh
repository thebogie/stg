#!/bin/bash
# Run SurrealDB migration files in deploy/migrations in lexical order.
# Intended for production deploys after SurrealDB is up and before backend/frontend start.
#
# Usage:
#   ./run_surreal_migrations.sh
#   MIGRATIONS_DIR=/opt/stg/deploy/migrations ./run_surreal_migrations.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_ROOT="${DEPLOY_ROOT:-$SCRIPT_DIR}"
ENV_FILE="${ENV_FILE:-$DEPLOY_ROOT/config/.env.prod}"
MIGRATIONS_DIR="${MIGRATIONS_DIR:-$DEPLOY_ROOT/migrations}"

if [ -f "$ENV_FILE" ]; then
  # shellcheck disable=SC1090
  set -a; source "$ENV_FILE"; set +a
fi

SURREALDB_PORT="${SURREALDB_PORT:-50001}"
SURREAL_ENDPOINT="${SURREAL_ENDPOINT:-http://127.0.0.1:${SURREALDB_PORT}}"
SURREAL_NS="${SURREAL_NS:-stg_rd}"
SURREAL_DB="${SURREAL_DB:-stg_rd}"
SURREAL_USER="${SURREAL_USER:-root}"
SURREAL_PASSWORD="${SURREAL_PASSWORD:-root}"

wait_surrealdb() {
  echo "==> Waiting for SurrealDB at $SURREAL_ENDPOINT (up to 60s)..."
  for _ in $(seq 1 20); do
    if wget -q -O- --tries=1 "$SURREAL_ENDPOINT" >/dev/null 2>&1; then
      echo "==> SurrealDB is ready."
      return 0
    fi
    sleep 3
  done
  echo "SurrealDB is not reachable at $SURREAL_ENDPOINT" >&2
  return 1
}

if [ ! -d "$MIGRATIONS_DIR" ]; then
  echo "==> No migrations directory found at $MIGRATIONS_DIR; skipping."
  exit 0
fi

shopt -s nullglob
MIGRATION_GLOB=("$MIGRATIONS_DIR"/*.surql)
if [ "${#MIGRATION_GLOB[@]}" -eq 0 ]; then
  echo "==> No migration files (*.surql) in $MIGRATIONS_DIR; skipping."
  exit 0
fi
mapfile -t MIGRATION_FILES < <(printf '%s\n' "${MIGRATION_GLOB[@]}" | sort)
if [ "${#MIGRATION_FILES[@]}" -eq 0 ]; then
  echo "==> No migration files (*.surql) in $MIGRATIONS_DIR; skipping."
  exit 0
fi

wait_surrealdb

for migration in "${MIGRATION_FILES[@]}"; do
  migration_file="$(basename "$migration")"
  migration_dir="$(cd "$(dirname "$migration")" && pwd)"
  migration_abs="$migration_dir/$migration_file"
  migration_id="$migration_file"

  echo "==> Applying migration: $migration_file"
  docker run --rm --network host \
    -v "$migration_dir:/import:ro" \
    surrealdb/surrealdb:v3 \
    import \
    --endpoint "$SURREAL_ENDPOINT" \
    --username "$SURREAL_USER" --password "$SURREAL_PASSWORD" \
    --namespace "$SURREAL_NS" --database "$SURREAL_DB" \
    "/import/$migration_file"

  marker_file="$(mktemp)"
  cat > "$marker_file" <<EOF
OPTION IMPORT;
UPSERT type::record("schema_migrations", "${migration_id}") SET
  appliedAt = time::now(),
  name = "${migration_id}";
EOF
  # mktemp is 0600; surrealdb in Docker runs as another UID and must read this bind mount.
  chmod a+r "$marker_file"

  docker run --rm --network host \
    -v "$(dirname "$marker_file"):/import:ro" \
    surrealdb/surrealdb:v3 \
    import \
    --endpoint "$SURREAL_ENDPOINT" \
    --username "$SURREAL_USER" --password "$SURREAL_PASSWORD" \
    --namespace "$SURREAL_NS" --database "$SURREAL_DB" \
    "/import/$(basename "$marker_file")"
  rm -f "$marker_file"

  echo "==> Migration applied: $migration_abs"
done

echo "==> All migrations completed."
