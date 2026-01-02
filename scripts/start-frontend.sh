#!/bin/bash

# Start frontend in development mode for hybrid development
# This runs Trunk dev server with hot reload

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FRONTEND_DIR="${PROJECT_ROOT}/frontend"

# Load environment variables
source "${SCRIPT_DIR}/load-env.sh"

echo -e "${BLUE}🚀 Starting Frontend Development Server${NC}"
echo ""

# Check if we're in the right directory structure
if [ ! -d "$FRONTEND_DIR" ]; then
  echo -e "${YELLOW}❌ Frontend directory not found: $FRONTEND_DIR${NC}"
  exit 1
fi

cd "$FRONTEND_DIR"

# Check if npm is installed
if ! command -v npm > /dev/null 2>&1; then
  echo -e "${YELLOW}❌ npm not found. Please install Node.js and npm first.${NC}"
  exit 1
fi

# Check if node_modules exists, install if not
if [ ! -d "node_modules" ] || [ ! -f "node_modules/.bin/tailwindcss" ]; then
  echo -e "${BLUE}📦 Installing npm dependencies...${NC}"
  npm install
  # Verify installation
  if [ ! -f "node_modules/.bin/tailwindcss" ]; then
    echo -e "${YELLOW}⚠️  tailwindcss not found after install, trying again...${NC}"
    npm install tailwindcss --save-dev
  fi
fi

# Check if Trunk is installed
if ! command -v trunk > /dev/null 2>&1; then
  echo -e "${YELLOW}⚠️  Trunk not found. Installing...${NC}"
  cargo install trunk
fi

# Build CSS first (one-time, if needed)
if [ ! -f "public/styles.css" ] || [ "public/styles.css" -ot "src/styles/main.css" ]; then
  echo -e "${BLUE}📦 Building CSS...${NC}"
  # Use npx to ensure npm can find the binary, or call directly
  if [ -f "node_modules/.bin/tailwindcss" ]; then
    ./node_modules/.bin/tailwindcss -i ./src/styles/main.css -o ./public/styles.css --minify
  else
    npx tailwindcss -i ./src/styles/main.css -o ./public/styles.css --minify
  fi
fi

echo -e "${GREEN}✅ Starting Trunk dev server...${NC}"
echo ""
echo -e "${BLUE}📝 Frontend will be available at: http://localhost:${FRONTEND_PORT}${NC}"
echo -e "${BLUE}📝 Backend API proxy: http://localhost:${FRONTEND_PORT}/api/ → http://localhost:${BACKEND_PORT}/api/${NC}"
echo -e "${BLUE}📝 Hot reload is enabled - changes will auto-refresh${NC}"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop${NC}"
echo ""

# Update Trunk.toml with correct backend URL from environment
# Trunk doesn't support env vars in TOML, so we need to update it dynamically
TRUNK_TOML="${FRONTEND_DIR}/Trunk.toml"
TRUNK_TOML_BACKUP="${TRUNK_TOML}.bak"

# Backup original if backup doesn't exist
if [ ! -f "$TRUNK_TOML_BACKUP" ]; then
  cp "$TRUNK_TOML" "$TRUNK_TOML_BACKUP"
fi

# Update port and backend URL in Trunk.toml (use temp file for portability)
TMP_FILE=$(mktemp)
sed "s|^port = .*|port = ${FRONTEND_PORT}|" "$TRUNK_TOML" | \
  sed "s|backend = \".*\"|backend = \"http://localhost:${BACKEND_PORT}/api/\"|" > "$TMP_FILE"
mv "$TMP_FILE" "$TRUNK_TOML"

# Restore original on exit
trap "if [ -f '$TRUNK_TOML_BACKUP' ]; then mv '$TRUNK_TOML_BACKUP' '$TRUNK_TOML' 2>/dev/null || true; fi" EXIT INT TERM

# Start Trunk dev server
# Unset NO_COLOR if set to avoid Trunk parsing issues
unset NO_COLOR

# Use the same command as other parts of the codebase
trunk serve --no-default-features --features frontend

