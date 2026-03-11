#!/bin/bash
# Run SurrealDB verification queries against http://localhost:50001 and report pass/fail.
# Usage: ./scripts/verify-surreal-local.sh
# Requires: SurrealDB at localhost:50001, jq. Uses curl (no Docker) when SurrealDB is in Docker Desktop
# and script runs from WSL so localhost:50001 is reachable. Loads config via load-env.sh dev.

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

source "$SCRIPT_DIR/load-env.sh" dev

SURREAL_NS="${SURREAL_NS:-stg_rd}"
SURREAL_DB="${SURREAL_DB:-stg_rd}"
SURREAL_USER="${SURREAL_USER:-root}"
SURREAL_PASSWORD="${SURREAL_PASSWORD:-root}"
PORT="${SURREALDB_PORT:-50001}"
# When SurrealDB runs in Docker Desktop (Windows), from WSL use localhost so port forwarding works
HTTP_URL="${SURREAL_VERIFY_URL:-http://127.0.0.1:${PORT}}"
PLAYER_KEY="${SURREAL_VERIFY_PLAYER_KEY:-2025041711441879938520500}"

# Use HTTP (curl) by default so it works from WSL when SurrealDB is in Docker Desktop.
# Set SURREAL_VERIFY_USE_DOCKER=1 to use Docker CLI instead.
USE_HTTP=1
[ -n "${SURREAL_VERIFY_USE_DOCKER}" ] && USE_HTTP=0

run_sql_docker() {
  echo "$1" | docker run -i --rm --add-host=host.docker.internal:host-gateway \
    surrealdb/surrealdb:v2 sql \
    --conn "http://host.docker.internal:${PORT}" --user "$SURREAL_USER" --pass "$SURREAL_PASSWORD" \
    --ns "$SURREAL_NS" --db "$SURREAL_DB" \
    --hide-welcome --json 2>/dev/null
}

run_sql_http() {
  curl -s --connect-timeout 5 --max-time 15 -X POST \
    -H "Accept: application/json" \
    -H "NS: $SURREAL_NS" \
    -H "DB: $SURREAL_DB" \
    -u "$SURREAL_USER:$SURREAL_PASSWORD" \
    --data "$1" \
    "$HTTP_URL/sql" 2>/dev/null
}

run_sql() {
  if [ "$USE_HTTP" = 1 ]; then
    run_sql_http "$1"
  else
    run_sql_docker "$1"
  fi
}

echo "==> Verifying SurrealDB at $HTTP_URL (ns=$SURREAL_NS db=$SURREAL_DB)"
[ "$USE_HTTP" = 1 ] && echo "    (using HTTP; set SURREAL_VERIFY_USE_DOCKER=1 to use Docker CLI)"
echo ""

PASS=0
FAIL=0

# A. Contest id string form (must NOT contain contest:contest:)
check_a() {
  local out
  out=$(run_sql "SELECT string::concat(id) AS contest_id_str FROM contest LIMIT 1")
  if echo "$out" | jq -e '.[0].contest_id_str | test("contest:`contest:")' >/dev/null 2>&1; then
    echo "FAIL A: Contest id has wrong key form (contest:\`contest:key\`). Re-import with updated arango-to-surreal."
    return 1
  fi
  if echo "$out" | jq -e '.[0].contest_id_str' >/dev/null 2>&1; then
    echo "PASS A: Contest id string form looks correct."
    return 0
  fi
  echo "FAIL A: No contest row or invalid response."
  return 1
}

# B. Edge out string form (should match contest id style)
check_b() {
  local out
  out=$(run_sql "SELECT string::concat(out) AS edge_out_str FROM resulted_in LIMIT 1")
  if echo "$out" | jq -e '.[0].edge_out_str' >/dev/null 2>&1; then
    echo "PASS B: Edge out string form present."
    return 0
  fi
  echo "FAIL B: No resulted_in row or invalid response."
  return 1
}

# C. Edge in stored as record id (in_str should be player:`key`)
check_c() {
  local out
  out=$(run_sql "SELECT string::concat(in) AS in_str FROM resulted_in LIMIT 1")
  if echo "$out" | jq -e '.[0].in_str | test("player:`")' >/dev/null 2>&1; then
    echo "PASS C: Edge in stored as record id (player:\`key\`)."
    return 0
  fi
  echo "FAIL C: Edge in string form missing or wrong."
  return 1
}

# D. Subquery returns contests for player
check_d() {
  local out
  out=$(run_sql "SELECT out AS contest_rid FROM resulted_in WHERE in = type::thing(\"player\", \"$PLAYER_KEY\")")
  local len
  len=$(echo "$out" | jq 'length' 2>/dev/null || echo "0")
  if [ "$len" -gt 0 ] 2>/dev/null; then
    echo "PASS D: Subquery for player $PLAYER_KEY returns $len contest(s)."
    return 0
  fi
  echo "FAIL D: No contests for player $PLAYER_KEY (in/type::thing may not match)."
  return 1
}

# E. Contest list (id IN subquery, no player filter)
check_e() {
  local out
  out=$(run_sql "SELECT * FROM contest WHERE id IN (SELECT out FROM resulted_in LIMIT 10)")
  local len
  len=$(echo "$out" | jq 'length' 2>/dev/null || echo "0")
  if [ "$len" -gt 0 ] 2>/dev/null; then
    echo "PASS E: Contest list (id IN subquery) returns $len row(s)."
    return 0
  fi
  echo "FAIL E: Contest list (id IN subquery) empty. IN (subquery) may not match."
  return 1
}

# F. Contest list for player
check_f() {
  local out
  out=$(run_sql "SELECT * FROM contest WHERE id IN (SELECT out FROM resulted_in WHERE in = type::thing(\"player\", \"$PLAYER_KEY\"))")
  local len
  len=$(echo "$out" | jq 'length' 2>/dev/null || echo "0")
  if [ "$len" -gt 0 ] 2>/dev/null; then
    echo "PASS F: Contest list for player returns $len row(s)."
    return 0
  fi
  echo "FAIL F: Contest list for player empty."
  return 1
}

# G. Counts (one query each to avoid multi-statement JSON shape issues)
check_g() {
  local out
  out=$(run_sql "SELECT count() AS players FROM player GROUP ALL")
  local players
  players=$(echo "$out" | jq -r '.[0].players // empty' 2>/dev/null)
  out=$(run_sql "SELECT count() AS contests FROM contest GROUP ALL")
  local contests
  contests=$(echo "$out" | jq -r '.[0].contests // empty' 2>/dev/null)
  out=$(run_sql "SELECT count() AS resulted_in_edges FROM resulted_in GROUP ALL")
  local edges
  edges=$(echo "$out" | jq -r '.[0].resulted_in_edges // empty' 2>/dev/null)
  if [ -n "$players" ] && [ -n "$contests" ] && [ -n "$edges" ]; then
    echo "PASS G: Counts — players=$players, contests=$contests, resulted_in_edges=$edges"
    return 0
  fi
  echo "FAIL G: Could not read counts (players=$players contests=$contests edges=$edges)."
  return 1
}

# Optional: try SELECT VALUE form if E/F failed
check_e2() {
  local out
  out=$(run_sql "SELECT * FROM contest WHERE id IN (SELECT VALUE out FROM resulted_in LIMIT 10)")
  local len
  len=$(echo "$out" | jq 'length' 2>/dev/null || echo "0")
  if [ "$len" -gt 0 ] 2>/dev/null; then
    echo "PASS E2: Contest list (SELECT VALUE out) returns $len row(s)."
    return 0
  fi
  echo "FAIL E2: Contest list (SELECT VALUE out) still empty."
  return 1
}

if ! command -v jq &>/dev/null; then
  echo "Error: jq is required. Install jq or run docs/verify-surreal-contest-list.surql manually in Surrealist." >&2
  exit 1
fi

# Check connectivity first (HTTP may return [ { "one": 1 } ] or nested array)
check_conn() {
  run_sql "SELECT 1 AS one" | jq -e '.[0].one == 1 or (.[0] | type == "array" and .[0].one == 1)' >/dev/null 2>&1
}

if ! check_conn; then
  # WSL + Docker Desktop: try hostnames/IPs that might reach the Windows host
  if [ -z "${SURREAL_VERIFY_URL}" ]; then
    for try_url in "http://host.docker.internal:${PORT}" "http://$(grep -m1 '^nameserver ' /etc/resolv.conf 2>/dev/null | awk '{print $2}'):${PORT}"; do
      case "$try_url" in
        http://:*) continue ;;
        http://127.0.0.*) continue ;;
      esac
      echo "  Trying $try_url ..." >&2
      HTTP_URL="$try_url"
      if check_conn; then
        echo "  Reached SurrealDB at $HTTP_URL" >&2
        break
      fi
    done
    if ! check_conn; then
      HTTP_URL="http://127.0.0.1:${PORT}"
    fi
  fi
fi

if ! check_conn; then
  echo "Error: Cannot reach SurrealDB at $HTTP_URL. Is it running on port $PORT?" >&2
  echo "" >&2
  echo "  WSL + Docker Desktop (SurrealDB on Windows):" >&2
  echo "    • Try: SURREAL_VERIFY_URL=http://host.docker.internal:$PORT ./scripts/verify-surreal-local.sh" >&2
  echo "    • Or run this script from Windows (PowerShell) so localhost:$PORT works." >&2
  echo "    • Or run the queries in Surrealist: paste docs/verify-surreal-contest-list.surql" >&2
  exit 1
fi

for check in check_a check_b check_c check_d check_e check_f check_g; do
  if $check; then ((PASS+=1)); else ((FAIL+=1)); fi
done

# If E or F failed, try E2
if [ $FAIL -gt 0 ]; then
  echo ""
  echo "Trying SELECT VALUE form (E2)..."
  if check_e2; then ((PASS+=1)); fi
fi

echo ""
echo "==> Result: $PASS passed, $FAIL failed"
[ $FAIL -eq 0 ] && exit 0 || exit 1
