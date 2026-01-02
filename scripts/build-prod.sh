#!/bin/bash

# Production build script
# This script builds both frontend and backend with proper version information

set -e

echo "🚀 Starting production build..."

# Get build information
echo "📋 Getting build information..."
source ./scripts/build-info.sh

# Export build info for the build process
export GIT_COMMIT
export BUILD_DATE

echo "Build Info:"
echo "  Git Commit: $GIT_COMMIT"
echo "  Build Date: $BUILD_DATE"

# Build frontend
echo "🔨 Building frontend..."
cd frontend
npm run build:css:prod
trunk build --release
cd ..

# Build backend
echo "🔨 Building backend..."
cd backend
cargo build --release
cd ..

echo "✅ Production build complete!"
echo "Frontend: dist/ directory in frontend/"
echo "Backend: target/release/backend"
echo "Version info: $GIT_COMMIT - $BUILD_DATE"
