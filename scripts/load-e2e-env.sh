#!/bin/bash

# Load environment variables for E2E tests
# This script loads config/.env.development and exports variables for Playwright
# Usage: source scripts/load-e2e-env.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Load environment variables from config/.env.development
if [ -f "${SCRIPT_DIR}/load-env.sh" ]; then
  source "${SCRIPT_DIR}/load-env.sh" development
else
  echo "Warning: load-env.sh not found" >&2
fi

# Export FRONTEND_URL for Playwright (uses FRONTEND_PORT from env file, but E2E overrides to 8080)
export FRONTEND_URL="${FRONTEND_URL:-http://localhost:8080}"

