#!/bin/bash

# Quick production deployment - just provide the version tag
# This is a convenience wrapper around pull-and-deploy-prod.sh
#
# Usage: ./scripts/deploy-prod-version.sh <VERSION_TAG> [SERVICE_NAME]
#
# Default service: docker-compose-stg.service
# Example:
#   ./scripts/deploy-prod-version.sh v4d4b456-20260115-233953
#   ./scripts/deploy-prod-version.sh v4d4b456-20260115-233953 docker-compose-stg.service

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

VERSION_TAG="$1"
SERVICE_NAME="${2:-docker-compose-stg.service}"

if [ -z "$VERSION_TAG" ]; then
    echo "Usage: $0 <VERSION_TAG> [SERVICE_NAME]"
    echo "Example: $0 v4d4b456-20260115-233953"
    echo "         $0 v4d4b456-20260115-233953 docker-compose-stg.service"
    exit 1
fi

# Build the command
CMD="./scripts/pull-and-deploy-prod.sh --version $VERSION_TAG"

# Add Docker Hub user if set in environment
if [ -n "${DOCKER_HUB_USER:-}" ]; then
    CMD="$CMD --docker-hub-user $DOCKER_HUB_USER"
fi

# Add service restart (always add since we have a default)
CMD="$CMD --restart-service $SERVICE_NAME"

# Execute
eval "$CMD"
