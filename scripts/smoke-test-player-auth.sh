#!/usr/bin/env bash
# Smoke-test player register → login → me against the **running** backend (real SurrealDB + Redis).
# Use this to confirm the source code works before debugging integration tests.
#
# Prereqs:
#   1. Stack up:  ./scripts/start-back.sh prod   (or deploy/stack.sh start)
#   2. Minimal schema:  source scripts/load-env.sh prod && ./scripts/apply-surreal-schema-minimal.sh
#   3. Backend running: from back/api, e.g.  source ../../scripts/load-env.sh prod && cargo run
#      (If using Docker stack, backend may already be running; then use BACKEND_BASE_URL=http://127.0.0.1:50002)
#
# Usage:  ./scripts/smoke-test-player-auth.sh [BASE_URL]
#   BASE_URL defaults to http://127.0.0.1:50002
#
# Exits 0 if register, login, and GET /api/players/me all succeed.

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

BASE_URL="${1:-http://127.0.0.1:50002}"
EMAIL="smoke-$(date +%s)@example.com"
PASSWORD="password123"
USERNAME="smokeuser"

echo "==> Smoke test: register → login → me (BASE_URL=$BASE_URL)"
echo "    Register: POST $BASE_URL/api/players/register"

REG="$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/players/register" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"$USERNAME\",\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\"}")"
REG_BODY="$(echo "$REG" | head -n -1)"
REG_CODE="$(echo "$REG" | tail -n 1)"

if [ "$REG_CODE" != "201" ]; then
  echo "FAIL: Register returned HTTP $REG_CODE (expected 201)"
  echo "$REG_BODY" | head -20
  exit 1
fi
echo "    Register: 201 OK"

echo "    Login: POST $BASE_URL/api/players/login"
LOGIN="$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/players/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\"}")"
LOGIN_BODY="$(echo "$LOGIN" | head -n -1)"
LOGIN_CODE="$(echo "$LOGIN" | tail -n 1)"

if [ "$LOGIN_CODE" != "200" ]; then
  echo "FAIL: Login returned HTTP $LOGIN_CODE (expected 200)"
  echo "$LOGIN_BODY" | head -20
  exit 1
fi
SESSION_ID="$(echo "$LOGIN_BODY" | grep -o '"session_id":"[^"]*"' | head -1 | cut -d'"' -f4)"
if [ -z "$SESSION_ID" ]; then
  echo "FAIL: Login response had no session_id"
  echo "$LOGIN_BODY" | head -5
  exit 1
fi
echo "    Login: 200 OK (session_id present)"

echo "    Me: GET $BASE_URL/api/players/me (Bearer)"
ME="$(curl -s -w "\n%{http_code}" -X GET "$BASE_URL/api/players/me" \
  -H "Authorization: Bearer $SESSION_ID")"
ME_BODY="$(echo "$ME" | head -n -1)"
ME_CODE="$(echo "$ME" | tail -n 1)"

if [ "$ME_CODE" != "200" ]; then
  echo "FAIL: GET /api/players/me returned HTTP $ME_CODE (expected 200)"
  echo "$ME_BODY" | head -20
  exit 1
fi
echo "    Me: 200 OK"

echo "==> Smoke test passed: register → login → me (source code works with real backend)."
