#!/bin/bash
# Create .env.dev or .env.prod from templates.
# Usage: ./config/setup-env.sh [dev|prod]

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENV="${1:-dev}"
case "$ENV" in
  dev)  TEMPLATE="$SCRIPT_DIR/env.dev.template";  OUT="$SCRIPT_DIR/.env.dev"  ;;
  prod) TEMPLATE="$SCRIPT_DIR/env.prod.template"; OUT="$SCRIPT_DIR/.env.prod" ;;
  *)
    echo "Usage: $0 [dev|prod]"
    exit 1
    ;;
esac
if [ ! -f "$TEMPLATE" ]; then
  echo "Error: Template not found: $TEMPLATE"
  exit 1
fi
if [ -f "$OUT" ]; then
  echo "Warning: $OUT already exists. Overwrite? (y/N)"
  read -r r
  [[ "$r" != [yY] ]] && exit 0
fi
cp "$TEMPLATE" "$OUT"
echo "Created $OUT from template. Edit as needed."
