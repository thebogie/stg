#!/bin/bash
set -e

# Get build information
echo "Getting build information..."
source ../scripts/build-info.sh

# Export build info for the build process
export GIT_COMMIT
export BUILD_DATE

echo "Build Info:"
echo "  Git Commit: $GIT_COMMIT"
echo "  Build Date: $BUILD_DATE"

# Build with Trunk (no wasm-opt)
# Pass git commit to the build process
export GIT_COMMIT
export BUILD_DATE
trunk build --release

# Optimize the WASM
# Path to the built WASM file (adjust if your output name changes)
WASM_INPUT="dist/frontend-*_bg.wasm"
WASM_OUTPUT="dist/frontend_bg.optimized.wasm"

# Path to wasm-opt (use Trunk's cached version or your own)
WASM_OPT="$HOME/binaryen-version_123/bin/wasm-opt"

# Run wasm-opt with required features
$WASM_OPT --enable-bulk-memory --enable-nontrapping-float-to-int -Oz -o $WASM_OUTPUT $WASM_INPUT

echo "Optimized WASM written to $WASM_OUTPUT"

echo "Production build complete. Optimized WASM is in dist/"