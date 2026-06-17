#!/bin/bash
# Deploy STG from GHCR: ensure systemd unit exists, stop service, docker pull backend + frontend images, compose down, start full stack.
#
# Usage (from deploy/ directory, e.g. after scp deploy/ to server):
#   ./deploy_stg.sh <label>
#
# <label> = image tag to pull and run (e.g. latest, or short sha 0013844 from CI).
#   CI tags both backend and frontend with the same tag (short sha + latest).
#
# What this script does:
# 1. If systemd unit stg.service does not exist: create and install it, daemon-reload, enable.
# 2. Stop the stg service (if running).
# 3. docker pull backend, frontend, and playwright-worker images from GHCR with <label>.
# 4. docker compose down (docker-compose.full.yml in this directory).
# 5. Start SurrealDB + Redis, run DB migrations, then start backend + frontend with new images.
#
# Requires: this deploy/ directory on the host (docker-compose.full.yml, Caddyfile.frontend, config/.env.prod); Docker.
# Optional: run as root so systemd unit can be installed and /etc/stg/stg.env written.

set -euo pipefail

SERVICE_NAME="stg"
UNIT_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
STG_ENV_FILE="/etc/stg/stg.env"

# GHCR image names (no tag). CI pushes to ghcr.io/<owner>/<repo>/backend, .../frontend, .../playwright-worker
GHCR_IMAGE_BACKEND="${GHCR_IMAGE_BACKEND:-ghcr.io/thebogie/stg/backend}"
GHCR_IMAGE_FRONTEND="${GHCR_IMAGE_FRONTEND:-ghcr.io/thebogie/stg/frontend}"
GHCR_IMAGE_PLAYWRIGHT_WORKER="${GHCR_IMAGE_PLAYWRIGHT_WORKER:-ghcr.io/thebogie/stg/playwright-worker}"

# Deploy root = directory containing this script (the deploy/ folder). Scp this folder to the server.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_ROOT="${DEPLOY_ROOT:-$SCRIPT_DIR}"
COMPOSE_FILE="$DEPLOY_ROOT/docker-compose.full.yml"
ENV_FILE="$DEPLOY_ROOT/config/.env.prod"

usage() {
  echo "Usage: $0 <label>" >&2
  echo "  <label>  Image tag to pull from GHCR and run (e.g. latest, or short sha 0013844)" >&2
  echo "  Backend:  $GHCR_IMAGE_BACKEND:<label>" >&2
  echo "  Frontend: $GHCR_IMAGE_FRONTEND:<label>" >&2
  echo "  Playwright worker: $GHCR_IMAGE_PLAYWRIGHT_WORKER:<label>" >&2
  exit 1
}

# --- Parse label ---
LABEL="${1:-}"
if [ -z "$LABEL" ]; then
  usage
fi

BACKEND_IMAGE_FULL="${GHCR_IMAGE_BACKEND}:${LABEL}"
FRONTEND_IMAGE_FULL="${GHCR_IMAGE_FRONTEND}:${LABEL}"
PLAYWRIGHT_WORKER_IMAGE_FULL="${GHCR_IMAGE_PLAYWRIGHT_WORKER}:${LABEL}"

# --- Ensure systemd unit exists ---
install_unit_if_missing() {
  if [ -f "$UNIT_FILE" ]; then
    echo "==> systemd unit $UNIT_FILE already exists; skipping install."
    return 0
  fi

  if [ "$(id -u)" -ne 0 ]; then
    echo "==> Unit $UNIT_FILE not found. Install it as root, e.g.: sudo $0 $LABEL" >&2
    exit 1
  fi

  echo "==> Creating systemd unit $UNIT_FILE"
  mkdir -p "$(dirname "$UNIT_FILE")"
  # BACKEND_IMAGE and FRONTEND_IMAGE are loaded from /etc/stg/stg.env when present
  cat > "$UNIT_FILE" <<EOF
[Unit]
Description=STG full stack (SurrealDB + Redis + backend + frontend)
After=docker.service network-online.target
Wants=network-online.target

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=$DEPLOY_ROOT
Environment=COMPOSE_PROJECT_NAME=stg
EnvironmentFile=-$STG_ENV_FILE
ExecStart=/usr/bin/docker compose -f $COMPOSE_FILE --env-file $ENV_FILE up -d
ExecStop=/usr/bin/docker compose -f $COMPOSE_FILE --env-file $ENV_FILE down
TimeoutStartSec=300

[Install]
WantedBy=multi-user.target
EOF
  systemctl daemon-reload
  systemctl enable "$SERVICE_NAME"
  echo "==> Installed and enabled $SERVICE_NAME.service"
}

# --- Stop service ---
stop_service() {
  if systemctl is-active --quiet "$SERVICE_NAME" 2>/dev/null; then
    echo "==> Stopping $SERVICE_NAME.service"
    systemctl stop "$SERVICE_NAME"
  else
    echo "==> $SERVICE_NAME.service not running"
  fi
}

# --- Pull backend, frontend, and playwright-worker images from GHCR ---
pull_images() {
  echo "==> Pulling $BACKEND_IMAGE_FULL"
  docker pull "$BACKEND_IMAGE_FULL"
  echo "==> Pulling $FRONTEND_IMAGE_FULL"
  docker pull "$FRONTEND_IMAGE_FULL"
  echo "==> Pulling $PLAYWRIGHT_WORKER_IMAGE_FULL"
  docker pull "$PLAYWRIGHT_WORKER_IMAGE_FULL"
}

# --- Write BACKEND_IMAGE and FRONTEND_IMAGE so systemd unit (or compose) uses them ---
write_image_env() {
  if [ "$(id -u)" -ne 0 ]; then
    echo "==> Not root; skipping write to $STG_ENV_FILE (compose will use env from this shell)"
    return 0
  fi
  mkdir -p "$(dirname "$STG_ENV_FILE")"
  echo "BACKEND_IMAGE=$BACKEND_IMAGE_FULL" > "$STG_ENV_FILE"
  echo "FRONTEND_IMAGE=$FRONTEND_IMAGE_FULL" >> "$STG_ENV_FILE"
  echo "PLAYWRIGHT_WORKER_IMAGE=$PLAYWRIGHT_WORKER_IMAGE_FULL" >> "$STG_ENV_FILE"
  echo "IMAGE_TAG=$LABEL" >> "$STG_ENV_FILE"
  echo "==> Wrote BACKEND_IMAGE, FRONTEND_IMAGE, and PLAYWRIGHT_WORKER_IMAGE to $STG_ENV_FILE"
}

# --- Compose down (ensure everything is down before we start again) ---
compose_down() {
  if [ ! -f "$COMPOSE_FILE" ] || [ ! -f "$ENV_FILE" ]; then
    echo "⚠ Compose or env file missing; skipping docker compose down."
    return 0
  fi
  echo "==> Bringing stack down (docker compose down)"
  cd "$DEPLOY_ROOT"
  # docker compose parses/interpolates image fields even for "down".
  # Ensure required IMAGE_* env vars are set so a fresh deploy doesn't fail interpolation.
  export BACKEND_IMAGE="$BACKEND_IMAGE_FULL"
  export FRONTEND_IMAGE="$FRONTEND_IMAGE_FULL"
  export PLAYWRIGHT_WORKER_IMAGE="$PLAYWRIGHT_WORKER_IMAGE_FULL"
  export IMAGE_TAG="$LABEL"
  docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down || true
}

# --- Start service ---
start_service() {
  echo "==> Starting $SERVICE_NAME.service (backend: $BACKEND_IMAGE_FULL, frontend: $FRONTEND_IMAGE_FULL, playwright-worker: $PLAYWRIGHT_WORKER_IMAGE_FULL)"
  if [ "$(id -u)" -eq 0 ] && systemctl is-enabled "$SERVICE_NAME" &>/dev/null; then
    write_image_env
    systemctl start "$SERVICE_NAME"
  else
    cd "$DEPLOY_ROOT"
    export COMPOSE_PROJECT_NAME=stg
    export BACKEND_IMAGE="$BACKEND_IMAGE_FULL"
    export FRONTEND_IMAGE="$FRONTEND_IMAGE_FULL"
    export PLAYWRIGHT_WORKER_IMAGE="$PLAYWRIGHT_WORKER_IMAGE_FULL"
    export IMAGE_TAG="$LABEL"
    docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d
  fi
}

start_deps_only() {
  echo "==> Starting SurrealDB + Redis only"
  cd "$DEPLOY_ROOT"
  export COMPOSE_PROJECT_NAME=stg
  export BACKEND_IMAGE="$BACKEND_IMAGE_FULL"
  export FRONTEND_IMAGE="$FRONTEND_IMAGE_FULL"
  export PLAYWRIGHT_WORKER_IMAGE="$PLAYWRIGHT_WORKER_IMAGE_FULL"
  export IMAGE_TAG="$LABEL"
  docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d surrealdb redis
}

run_db_migrations() {
  if [ ! -x "$DEPLOY_ROOT/run_surreal_migrations.sh" ]; then
    echo "==> Migration runner missing or not executable: $DEPLOY_ROOT/run_surreal_migrations.sh" >&2
    exit 1
  fi
  echo "==> Running SurrealDB migrations..."
  DEPLOY_ROOT="$DEPLOY_ROOT" ENV_FILE="$ENV_FILE" "$DEPLOY_ROOT/run_surreal_migrations.sh"
}

# Backend runs as uid 1000; host bind mount must be writable (contest thumbnails under contest-images/).
ensure_backend_data_writable() {
  if [ "$(id -u)" -ne 0 ]; then
    return 0
  fi
  if [ ! -f "$ENV_FILE" ]; then
    return 0
  fi
  # shellcheck disable=SC1090
  set -a
  # shellcheck source=/dev/null
  . "$ENV_FILE" 2>/dev/null || true
  set +a
  if [ -z "${VOLUME_PATH:-}" ]; then
    return 0
  fi
  mkdir -p "${VOLUME_PATH}/backend_data/contest-images"
  chown -R 1000:1000 "${VOLUME_PATH}/backend_data"
  echo "==> ${VOLUME_PATH}/backend_data owned by uid 1000 (contest thumbnails)"
}

# --- Main ---
install_unit_if_missing
stop_service
pull_images
compose_down
start_deps_only
run_db_migrations
ensure_backend_data_writable
start_service
echo "✅ Deploy finished: backend $BACKEND_IMAGE_FULL, frontend $FRONTEND_IMAGE_FULL, playwright-worker $PLAYWRIGHT_WORKER_IMAGE_FULL"
