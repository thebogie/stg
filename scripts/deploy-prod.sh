#!/bin/bash

# Production deployment script
# This script sets build information and deploys using docker-compose

set -e

echo "🚀 Starting production deployment..."

# Get build information
echo "📋 Getting build information..."
source ./scripts/build-info.sh

# Export build info for docker-compose
export GIT_COMMIT
export BUILD_DATE

echo "Build Info:"
echo "  Git Commit: $GIT_COMMIT"
echo "  Build Date: $BUILD_DATE"

# Check if .env file exists
if [ ! -f ".env" ]; then
    echo "❌ .env file not found. Please create one from .env.example"
    exit 1
fi

# Check if docker-compose is available
if ! command -v docker-compose &> /dev/null; then
    echo "❌ docker-compose not found. Please install docker-compose"
    exit 1
fi

# Build and start services
echo "🔨 Building and starting services..."
docker-compose up --build -d

echo "✅ Production deployment complete!"
echo "Services should be available at:"
echo "  Frontend: http://localhost:\${FRONTEND_PORT}"
echo "  Backend: http://localhost:\${BACKEND_PORT}"
echo "  ArangoDB: http://localhost:\${ARANGODB_PORT}"
echo "  Redis: localhost:\${REDIS_PORT}"
echo ""
echo "Build info: $GIT_COMMIT - $BUILD_DATE"
