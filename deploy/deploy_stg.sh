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
# 3. docker pull backend and frontend images from GHCR with <label>.
# 4. docker compose down (docker-compose.full.yml in this directory).
# 5. Write BACKEND_IMAGE, FRONTEND_IMAGE, IMAGE_TAG to /etc/stg/stg.env, then start the stg service (full stack).
#
# Requires: this deploy/ directory on the host (docker-compose.full.yml, Caddyfile.frontend, config/.env.prod); Docker.
# Optional: run as root so systemd unit can be installed and /etc/stg/stg.env written.

set -euo pipefail

SERVICE_NAME="stg"
UNIT_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
STG_ENV_FILE="/etc/stg/stg.env"

# GHCR image names (no tag). CI pushes to ghcr.io/<owner>/<repo>/backend and .../frontend
GHCR_IMAGE_BACKEND="${GHCR_IMAGE_BACKEND:-ghcr.io/thebogie/stg/backend}"
GHCR_IMAGE_FRONTEND="${GHCR_IMAGE_FRONTEND:-ghcr.io/thebogie/stg/frontend}"

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
  exit 1
}

# --- Parse label ---
LABEL="${1:-}"
if [ -z "$LABEL" ]; then
  usage
fi

BACKEND_IMAGE_FULL="${GHCR_IMAGE_BACKEND}:${LABEL}"
FRONTEND_IMAGE_FULL="${GHCR_IMAGE_FRONTEND}:${LABEL}"

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

# --- Pull backend and frontend images from GHCR ---
pull_images() {
  echo "==> Pulling $BACKEND_IMAGE_FULL"
  docker pull "$BACKEND_IMAGE_FULL"
  echo "==> Pulling $FRONTEND_IMAGE_FULL"
  docker pull "$FRONTEND_IMAGE_FULL"
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
  echo "IMAGE_TAG=$LABEL" >> "$STG_ENV_FILE"
  echo "==> Wrote BACKEND_IMAGE and FRONTEND_IMAGE to $STG_ENV_FILE"
}

# --- Compose down (ensure everything is down before we start again) ---
compose_down() {
  if [ ! -f "$COMPOSE_FILE" ] || [ ! -f "$ENV_FILE" ]; then
    echo "⚠ Compose or env file missing; skipping docker compose down."
    return 0
  fi
  echo "==> Bringing stack down (docker compose down)"
  cd "$DEPLOY_ROOT"
  docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down || true
}

# --- Start service ---
start_service() {
  echo "==> Starting $SERVICE_NAME.service (backend: $BACKEND_IMAGE_FULL, frontend: $FRONTEND_IMAGE_FULL)"
  if [ "$(id -u)" -eq 0 ] && systemctl is-enabled "$SERVICE_NAME" &>/dev/null; then
    write_image_env
    systemctl start "$SERVICE_NAME"
  else
    cd "$DEPLOY_ROOT"
    export COMPOSE_PROJECT_NAME=stg
    export BACKEND_IMAGE="$BACKEND_IMAGE_FULL"
    export FRONTEND_IMAGE="$FRONTEND_IMAGE_FULL"
    export IMAGE_TAG="$LABEL"
    docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d
  fi
}

# --- Main ---
install_unit_if_missing
stop_service
pull_images
compose_down
start_service
echo "✅ Deploy finished: backend $BACKEND_IMAGE_FULL, frontend $FRONTEND_IMAGE_FULL"
