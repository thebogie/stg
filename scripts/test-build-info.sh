#!/bin/bash

# Test script to verify build info is working correctly

set -e

echo "🧪 Testing build info configuration..."

# Get build information
echo "📋 Getting build information..."
source ./scripts/build-info.sh

# Export build info for the build process
export GIT_COMMIT
export BUILD_DATE

echo "Build Info:"
echo "  Git Commit: $GIT_COMMIT"
echo "  Build Date: $BUILD_DATE"

# Test that environment variables are set
if [ -z "$GIT_COMMIT" ]; then
    echo "❌ GIT_COMMIT is not set"
    exit 1
fi

if [ -z "$BUILD_DATE" ]; then
    echo "❌ BUILD_DATE is not set"
    exit 1
fi

echo "✅ Environment variables are set correctly"

# Test frontend build with build info
echo "🔨 Testing frontend build with build info..."
cd frontend

# Run a quick build to test
echo "Running trunk build --release..."
if trunk build --release; then
    echo "✅ Frontend build successful"
else
    echo "❌ Frontend build failed"
    exit 1
fi

cd ..

echo "✅ All tests passed!"
echo "Build info should now be available in the frontend application"
