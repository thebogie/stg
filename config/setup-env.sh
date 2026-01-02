#!/bin/bash

# Setup script for environment files
# Creates .env files from templates

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  echo "Usage: $0 [development|production]"
  echo ""
  echo "Creates .env files from templates in config/ directory"
  exit 0
}

ENVIRONMENT="${1:-development}"

if [ "$ENVIRONMENT" = "--help" ] || [ "$ENVIRONMENT" = "-h" ]; then
  usage
fi

if [ "$ENVIRONMENT" != "development" ] && [ "$ENVIRONMENT" != "production" ]; then
  echo "❌ Error: Environment must be 'development' or 'production'"
  exit 1
fi

TEMPLATE_FILE="$SCRIPT_DIR/env.$ENVIRONMENT.template"
ENV_FILE="$SCRIPT_DIR/.env.$ENVIRONMENT"

if [ ! -f "$TEMPLATE_FILE" ]; then
  echo "❌ Error: Template file not found: $TEMPLATE_FILE"
  exit 1
fi

if [ -f "$ENV_FILE" ]; then
  echo "⚠️  $ENV_FILE already exists."
  read -p "Overwrite? (y/N): " -n 1 -r
  echo
  if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Skipping..."
    exit 0
  fi
fi

cp "$TEMPLATE_FILE" "$ENV_FILE"

echo "✅ Created $ENV_FILE from template"
echo ""
echo "📝 Next steps:"
echo "   1. Edit $ENV_FILE and update:"
if [ "$ENVIRONMENT" = "production" ]; then
  echo "      - ARANGO_ROOT_PASSWORD (use strong password)"
  echo "      - ARANGO_PASSWORD (use strong password)"
  echo "      - VOLUME_PATH (use absolute path, e.g., /var/lib/stg_rd)"
  echo "      - GOOGLE_LOCATION_API_KEY"
  echo "      - All port configurations for your production setup"
else
  echo "      - ARANGO_ROOT_PASSWORD (change from default)"
  echo "      - ARANGO_PASSWORD (change from default)"
  echo "      - GOOGLE_LOCATION_API_KEY (if you have one)"
fi
echo "   2. Run: ./deploy/deploy.sh --env $ENVIRONMENT --build"



