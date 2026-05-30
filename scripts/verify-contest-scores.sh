#!/usr/bin/env bash
# Verify contest scores against prod-copy (or prod) SurrealDB.
#
# Prereqs: Surreal up with prod seed (see below).
#
# Fresh prod seed + deps:
#   SURREAL_SEED_FORCE=1 ./scripts/start-deps.sh
#
# Usage:
#   ./scripts/verify-contest-scores.sh
#   CONTEST_KEY=<uuid> ./scripts/verify-contest-scores.sh
#
# Optional: simulate legacy prod function (no score in fn::contest_with_edges) before verify:
#   SIMULATE_LEGACY_FN=1 ./scripts/verify-contest-scores.sh

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

# shellcheck source=/dev/null
source "$SCRIPT_DIR/load-env.sh" dev

CONTEST_KEY="${CONTEST_KEY:-9e230f40-18e5-439f-82d2-50dea1860e5d}"
SURREAL_ENDPOINT="http://127.0.0.1:${SURREALDB_PORT:-50001}"

echo "==> Checking resulted_in scores in SurrealDB for contest $CONTEST_KEY"
echo "SELECT place, result, score FROM resulted_in WHERE \`in\` = type::record('contest', '$CONTEST_KEY') ORDER BY place;" \
  | docker run -i --rm --network host surrealdb/surrealdb:v3 sql \
    --endpoint "$SURREAL_ENDPOINT" \
    --username "$SURREAL_USER" --password "$SURREAL_PASSWORD" \
    --namespace "$SURREAL_NS" --database "$SURREAL_DB" \
    --hide-welcome 2>/dev/null | tail -n +2

if [ "${SIMULATE_LEGACY_FN:-0}" = "1" ]; then
  echo "==> Applying legacy fn::contest_with_edges (no score field — simulates prod before fix)"
  docker run -i --rm --network host surrealdb/surrealdb:v3 import \
    --endpoint "$SURREAL_ENDPOINT" \
    --username "$SURREAL_USER" --password "$SURREAL_PASSWORD" \
    --namespace "$SURREAL_NS" --database "$SURREAL_DB" /dev/stdin <<'EOF'
OPTION IMPORT;
REMOVE FUNCTION IF EXISTS fn::contest_with_edges;
DEFINE FUNCTION fn::contest_with_edges($key: string) {
	LET $contest = (SELECT * FROM contest WHERE id = type::record('contest', $key) LIMIT 1)[0];
	IF $contest == NONE { RETURN NONE; };
	LET $cid = $contest.id;
	RETURN {
		contest: $contest,
		venue_id: (SELECT `out` FROM played_at WHERE `in` = $cid LIMIT 1)[0].out,
		game_ids: (SELECT VALUE `out` FROM played_with WHERE `in` = $cid),
		outcomes: (SELECT `out` AS player_id, place, result FROM resulted_in WHERE `in` = $cid ORDER BY place ASC)
	};
};
EOF
fi

echo "==> Backend repository path (find_details_by_id — includes score reload fallback)"
exec env CONTEST_KEY="$CONTEST_KEY" cargo run -p backend --bin verify_contest_scores --quiet
