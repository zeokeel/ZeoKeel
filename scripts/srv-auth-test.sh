#!/usr/bin/env bash
# M2 P1 auth smoke: real PostgreSQL + real server binary + curl assertions.
set -euo pipefail
DB="zk_auth_test_$$"
ROLE="zk_auth_role_$$"
PORT="${ZEOKEEL_TEST_PORT:-8798}"
BASE="http://127.0.0.1:${PORT}"
PSQL=(sudo -u postgres psql -v ON_ERROR_STOP=1 -Atq)
cleanup() {
  if [[ -n "${SERVER_PID:-}" ]]; then kill "$SERVER_PID" >/dev/null 2>&1 || true; fi
  sudo -u postgres dropdb --if-exists "$DB" >/dev/null 2>&1 || true
  sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true
}
trap cleanup EXIT
sudo -u postgres psql -v ON_ERROR_STOP=1 -Atqc "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';"
sudo -u postgres createdb -O "$ROLE" "$DB"
cargo build -q -p zk-server
ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" \
ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" \
ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD="admin-pass" \
./target/debug/zk-server >/tmp/zk-auth-test.log 2>&1 &
SERVER_PID=$!
for _ in $(seq 1 30); do curl -fsS "$BASE/healthz" >/dev/null 2>&1 && break; sleep 0.2; done
[[ "$(curl -fsS "$BASE/readyz" | python3 -c 'import json,sys;print(json.load(sys.stdin)["status"])')" == "ready" ]]
LOGIN=$(curl -fsS -X POST "$BASE/api/client/v1/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass","device_id":"dev1"}')
ACCESS=$(printf '%s' "$LOGIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["access_token"])')
REFRESH=$(printf '%s' "$LOGIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["refresh_token"])')
[[ "$(curl -fsS "$BASE/api/client/v1/auth/me" -H "Authorization: Bearer $ACCESS" | python3 -c 'import json,sys;print(json.load(sys.stdin)["role"])')" == "super_admin" ]]
NEW=$(curl -fsS -X POST "$BASE/api/client/v1/auth/refresh" -H 'content-type: application/json' -d "{\"refresh_token\":\"$REFRESH\",\"device_id\":\"dev1\"}")
[[ -n "$(printf '%s' "$NEW" | python3 -c 'import json,sys;print(json.load(sys.stdin)["access_token"])')" ]]
CODE=$(curl -sS -o /tmp/zk-auth-reuse.json -w '%{http_code}' -X POST "$BASE/api/client/v1/auth/refresh" -H 'content-type: application/json' -d "{\"refresh_token\":\"$REFRESH\",\"device_id\":\"dev1\"}")
[[ "$CODE" == "401" ]]
ADMIN=$(curl -fsS -c /tmp/zk-auth-admin.cookies -X POST "$BASE/api/admin/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass"}')
CSRF=$(printf '%s' "$ADMIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["csrf_token"])')
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/auth/logout")
[[ "$CODE" == "403" ]]
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/auth/logout")
[[ "$CODE" == "200" ]]
echo "auth ok: login/me/refresh/reuse/admin-csrf"
