#!/bin/bash

# Build info script for version display
# This script captures git commit and build date information

set -e

# Get git commit hash (short)
GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

# Get build date
BUILD_DATE=$(date -u +"%Y-%m-%d %H:%M:%S UTC")

# Export for use in build
export GIT_COMMIT
export BUILD_DATE

echo "Build Info:"
echo "  Git Commit: $GIT_COMMIT"
echo "  Build Date: $BUILD_DATE"

# If called with --export, export the variables
if [[ "$1" == "--export" ]]; then
    echo "export GIT_COMMIT=\"$GIT_COMMIT\""
    echo "export BUILD_DATE=\"$BUILD_DATE\""
fi 