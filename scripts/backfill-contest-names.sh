#!/usr/bin/env bash
# Retroactively rename contests to "{Game} — {Weekday Mon D}" (venue timezone).
#
# Usage (from repo root; loads dev env like backend-watch):
#   ./scripts/backfill-contest-names.sh              # dry-run
#   ./scripts/backfill-contest-names.sh --apply      # write to SurrealDB
#   ./scripts/backfill-contest-names.sh --limit 20   # sample first 20
#
# Production: set ENV_FILE_PATH to prod env (or source load-env.sh prod) before --apply.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

# shellcheck source=/dev/null
source "$SCRIPT_DIR/load-env.sh" dev

exec cargo run -p backend --bin backfill_contest_names -- "$@"
