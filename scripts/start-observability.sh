#!/usr/bin/env bash
# Start Loki + Promtail + Grafana + Prometheus (requires main stack / `stg` network).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# shellcheck source=scripts/load-env.sh
source "$ROOT/scripts/load-env.sh"

# load-env.sh sets ENV_FILE to an absolute path (defaults to dev).
ENV_FILE="${ENV_FILE:-$ROOT/config/.env.prod}"

if ! docker network inspect stg >/dev/null 2>&1; then
  echo "Docker network 'stg' not found. Start the main stack first: ./scripts/start-back.sh"
  exit 1
fi

mkdir -p "$VOLUME_PATH/loki" "$VOLUME_PATH/prometheus" "$VOLUME_PATH/grafana"
if [ ! -w "$VOLUME_PATH/grafana" ]; then
  echo "Error: $VOLUME_PATH/grafana is not writable (often root-owned from an earlier Docker run)." >&2
  echo "  Fix: rm -rf \"$VOLUME_PATH/grafana\" \"$VOLUME_PATH/loki\" \"$VOLUME_PATH/prometheus\" && re-run this script" >&2
  exit 1
fi
chmod 777 "$VOLUME_PATH/loki" "$VOLUME_PATH/prometheus" "$VOLUME_PATH/grafana" 2>/dev/null || true

docker compose \
  --project-directory "$ROOT/deploy" \
  -f deploy/docker-compose.observability.yml \
  --env-file "$ENV_FILE" \
  up -d

echo "Observability stack started."
echo "  Grafana:    http://localhost:${GRAFANA_PORT:-3000}"
echo "  Prometheus: http://localhost:${PROMETHEUS_PORT:-9090}"
echo "  Loki:       http://localhost:${LOKI_PORT:-3100}"
