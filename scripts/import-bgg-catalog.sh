#!/usr/bin/env bash
# Load the same env as backend-watch (Surreal password, URL, NS/DB), then run the BGG CSV importer.
# Optional: BGG_IMPORT_MAX_ROWS=10000 for a partial import (faster than ~175k full rows).
# Usage (from repo root; loads dev env like backend-watch):
#   ./scripts/import-bgg-catalog.sh
#   ./scripts/import-bgg-catalog.sh data/bgg/boardgames_ranks.csv
#   ./scripts/import-bgg-catalog.sh data/bgg/boardgames_ranks.csv my-batch-id

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

# shellcheck source=/dev/null
source "$SCRIPT_DIR/load-env.sh" dev

exec cargo run -p backend --bin import_bgg_catalog -- "$@"
