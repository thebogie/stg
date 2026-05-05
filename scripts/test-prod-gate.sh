#!/usr/bin/env bash
# Full deploy gate: unit + integration (full testing crate) + Playwright E2E against prod-built Docker images.
# Same artifacts CI should validate before release. Run from repo root.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$SCRIPT_DIR/full-prod-test.sh" "$@"
