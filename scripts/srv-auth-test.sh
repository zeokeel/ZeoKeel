#!/usr/bin/env bash
# M2 P1 auth smoke: real PostgreSQL + real server binary + curl assertions.
set -euo pipefail
DB="zk_auth_test_$$"
ROLE="zk_auth_role_$$"
PORT="${ZEOKEEL_TEST_PORT:-8798}"
UPSTREAM_PORT="${ZEOKEEL_UPSTREAM_TEST_PORT:-8796}"
BASE="http://127.0.0.1:${PORT}"
PSQL=(sudo -u postgres psql -v ON_ERROR_STOP=1 -Atq)
cleanup() {
  if [[ -n "${SERVER_PID:-}" ]]; then kill "$SERVER_PID" >/dev/null 2>&1 || true; fi
  if [[ -n "${UPSTREAM_PID:-}" ]]; then kill "$UPSTREAM_PID" >/dev/null 2>&1 || true; fi
  sudo -u postgres dropdb --if-exists "$DB" >/dev/null 2>&1 || true
  sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true
}
trap cleanup EXIT
sudo -u postgres psql -v ON_ERROR_STOP=1 -Atqc "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';"
sudo -u postgres createdb -O "$ROLE" "$DB"
cat > /tmp/zk-fake-upstream.py <<'PYUP'
from http.server import BaseHTTPRequestHandler, HTTPServer
class H(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get("content-length", "0"))
        raw = self.rfile.read(length)
        if b'"stream":true' in raw or b'"stream": true' in raw:
            body = b'data: {"response":{"usage":{"input_tokens":5,"output_tokens":7}}}\n\ndata: [DONE]\n\n'
            content_type = "text/event-stream"
        else:
            body = b'{"id":"resp_1","output":[{"type":"message","content":"hi"}],"usage":{"input_tokens":10,"output_tokens":20}}'
            content_type = "application/json"
        self.send_response(200)
        self.send_header("content-type", content_type)
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
    def log_message(self, *args):
        pass
HTTPServer(("127.0.0.1", int(__import__("sys").argv[1])), H).serve_forever()
PYUP
python3 /tmp/zk-fake-upstream.py "$UPSTREAM_PORT" >/dev/null 2>&1 &
UPSTREAM_PID=$!
cargo build -q -p zk-server
ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" \
ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" \
ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD="admin-pass" \
ZEOKEEL_UPSTREAM_BASE_URL="http://127.0.0.1:$UPSTREAM_PORT/v1" \
ZEOKEEL_UPSTREAM_API_KEY="test-upstream-key" \
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

# users/roles + permission matrix (fresh admin session)
ADMIN=$(curl -fsS -c /tmp/zk-auth-admin.cookies -X POST "$BASE/api/admin/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass"}')
CSRF=$(printf '%s' "$ADMIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["csrf_token"])')
curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/users" -H 'content-type: application/json' -d '{"username":"alice","password":"alice-pass","role":"employee"}' >/dev/null
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/users" -H 'content-type: application/json' -d '{"username":"bob","password":"bob-pass","role":"employee"}')
[[ "$CODE" == "403" ]]
USERS=$(curl -fsS -b /tmp/zk-auth-admin.cookies "$BASE/api/admin/users")
[[ "$(printf '%s' "$USERS" | python3 -c 'import json,sys;print(len(json.load(sys.stdin)["users"]))')" == "2" ]]
CODE=$(curl -sS -o /dev/null -w '%{http_code}' "$BASE/api/admin/users")
[[ "$CODE" == "401" ]]
curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/users" -H 'content-type: application/json' -d '{"username":"auditor","password":"auditor-pass","role":"auditor"}' >/dev/null
AUD=$(curl -fsS -c /tmp/zk-auth-auditor.cookies -X POST "$BASE/api/admin/auth/login" -H 'content-type: application/json' -d '{"username":"auditor","password":"auditor-pass"}')
AUDCSRF=$(printf '%s' "$AUD" | python3 -c 'import json,sys;print(json.load(sys.stdin)["csrf_token"])')
CODE=$(curl -sS -b /tmp/zk-auth-auditor.cookies -o /dev/null -w '%{http_code}' "$BASE/api/admin/users")
[[ "$CODE" == "200" ]]
CODE=$(curl -sS -b /tmp/zk-auth-auditor.cookies -H "X-CSRF-Token: $AUDCSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/users" -H 'content-type: application/json' -d '{"username":"carol","password":"carol-pass","role":"employee"}')
[[ "$CODE" == "403" ]]
ADMIN_ID=$(printf '%s' "$LOGIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["user"]["id"])')
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X DELETE "$BASE/api/admin/users/$ADMIN_ID")
[[ "$CODE" == "409" ]]
ALICE_ID=$(printf '%s' "$USERS" | python3 -c 'import json,sys;print([u["id"] for u in json.load(sys.stdin)["users"] if u["username"]=="alice"][0])')
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/wallet/$ALICE_ID/adjust" -H 'content-type: application/json' -d '{"delta_micros":1000000,"reason":"test grant","idempotency_key":"grant-alice-1"}')
[[ "$CODE" == "200" ]]
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/wallet/$ALICE_ID/adjust" -H 'content-type: application/json' -d '{"delta_micros":1000000,"reason":"test grant","idempotency_key":"grant-alice-1"}')
[[ "$CODE" == "200" ]]
ALICE_JWT=$(curl -fsS -X POST "$BASE/api/client/v1/auth/login" -H 'content-type: application/json' -d '{"username":"alice","password":"alice-pass","device_id":"alice-dev"}' | python3 -c 'import json,sys;print(json.load(sys.stdin)["access_token"])')
BAL=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $ALICE_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
[[ "$BAL" == "1000000" ]]
LEDGER=$(curl -fsS "$BASE/api/client/v1/wallet/ledger" -H "Authorization: Bearer $ALICE_JWT")
[[ "$(printf '%s' "$LEDGER" | python3 -c 'import json,sys;print(len(json.load(sys.stdin)["items"]))')" == "1" ]]
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X PUT "$BASE/api/admin/pricing/deepseek-chat" -H 'content-type: application/json' -d '{"input_per_1k_micros":1000,"output_per_1k_micros":2000}')
[[ "$CODE" == "200" ]]
MODELS=$(curl -fsS "$BASE/v1/models" -H "Authorization: Bearer $ALICE_JWT")
[[ "$(printf '%s' "$MODELS" | python3 -c 'import json,sys;print(json.load(sys.stdin)["data"][0]["id"])')" == "deepseek-chat" ]]
CODE=$(curl -sS -b /tmp/zk-auth-auditor.cookies -H "X-CSRF-Token: $AUDCSRF" -o /dev/null -w '%{http_code}' -X PUT "$BASE/api/admin/pricing/x" -H 'content-type: application/json' -d '{"input_per_1k_micros":1,"output_per_1k_micros":1}')
[[ "$CODE" == "403" ]]
GW=$(curl -sS -X POST "$BASE/v1/responses" -H "Authorization: Bearer $ALICE_JWT" -H 'content-type: application/json' -d '{"model":"deepseek-chat","input":"hello"}')
[[ "$(printf '%s' "$GW" | python3 -c 'import json,sys;print(json.load(sys.stdin)["id"])')" == "resp_1" ]]
BAL2=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $ALICE_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
[[ "$BAL2" == "999950" ]]
SSE=$(curl -sS -N -X POST "$BASE/v1/responses" -H "Authorization: Bearer $ALICE_JWT" -H 'content-type: application/json' -d '{"model":"deepseek-chat","input":"hello","stream":true}')
printf '%s' "$SSE" | grep -q "\[DONE\]"
sleep 0.5
BAL3=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $ALICE_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
[[ "$BAL3" == "999931" ]]
CODE=$(curl -sS -o /tmp/zk-gw-unpriced.json -w '%{http_code}' -X POST "$BASE/v1/responses" -H "Authorization: Bearer $ALICE_JWT" -H 'content-type: application/json' -d '{"model":"unpriced","input":"x"}')
[[ "$CODE" == "402" ]]
CODE=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$BASE/v1/responses" -H 'content-type: application/json' -d '{"model":"deepseek-chat","input":"x"}')
[[ "$CODE" == "401" ]]
echo "auth ok: login/me/refresh/reuse/admin-csrf/users-auditor/wallet-ledger/pricing/gateway"
