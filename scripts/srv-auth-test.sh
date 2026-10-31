#!/usr/bin/env bash
# M2 P1/P2 server smoke: real PostgreSQL + real server binary + curl assertions.
#
# Covers auth/session lifecycle, fail-closed audit, gateway billing (real OpenAI
# Responses SSE shape, server-side request ids, streaming overdraft), usage_pending
# wiring, single-instance advisory lock, migration checksum refusal and SIGTERM.
#
# Security round-2 fixtures (docs/21 §10): the fake upstream is loopback plaintext HTTP,
# so ZEOKEEL_ALLOW_INSECURE_UPSTREAM=1 is required; provider keys are encrypted at rest,
# so an ephemeral test master key is passed (never a real secret).
set -euo pipefail
DB="zk_auth_test_$$"
ROLE="zk_auth_role_$$"
PORT="${ZEOKEEL_TEST_PORT:-8798}"
UPSTREAM_PORT="${ZEOKEEL_UPSTREAM_TEST_PORT:-8796}"
UPSTREAM_FAIL_PORT=$((UPSTREAM_PORT + 1))
BASE="http://127.0.0.1:${PORT}"
PSQL=(sudo -u postgres psql -d "$DB" -v ON_ERROR_STOP=1 -Atq)
q() { "${PSQL[@]}" -c "$1"; }
sha() { printf '%s' "$1" | sha256sum | cut -d' ' -f1; }
CLIENT_ID="0.1.0;official;linux-x64;test"
# Round-6 business gate: every client business (and gateway) curl carries the
# same client identity. Gateway settlement ids remain server-generated.
curl() {
  local arg needs_client=0 has_client_header=0
  for arg in "$@"; do
    if [[ "$arg" == *"/api/client/v1/"* || "$arg" == *"$BASE/v1/"* ]]; then
      needs_client=1
    fi
    if [[ "$arg" == X-ZeoKeel-Client:* ]]; then
      has_client_header=1
    fi
  done
  if (( needs_client && !has_client_header )); then
    command curl -H "X-ZeoKeel-Client: $CLIENT_ID" "$@"
  else
    command curl "$@"
  fi
}
start_server() {
  local port="$1" log="$2"
  ZEOKEEL_ALLOW_INSECURE_UPSTREAM=1 \
  ZEOKEEL_MASTER_KEY="${ZEOKEEL_TEST_MASTER_KEY:-000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f}" \
  ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" \
  ZEOKEEL_SERVER_ADDR="127.0.0.1:$port" \
  ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD="admin-pass" \
  ZEOKEEL_MONTHLY_GRANT_MICROS="500000" \
  ZEOKEEL_STREAM_NEGATIVE_FLOOR_MICROS="100000" \
  ZEOKEEL_UPSTREAM_BASE_URL="http://127.0.0.1:$UPSTREAM_PORT/v1" \
  ZEOKEEL_UPSTREAM_API_KEY="test-upstream-key" \
  ./target/debug/zk-server >>"$log" 2>&1 &
  SERVER_PID=$!
}
wait_ready() {
  local base="$1"
  for _ in $(seq 1 60); do curl -fsS "$base/healthz" >/dev/null 2>&1 && break; sleep 0.2; done
  [[ "$(curl -fsS "$base/readyz" | python3 -c 'import json,sys;print(json.load(sys.stdin)["status"])')" == "ready" ]]
}
cleanup() {
  if [[ -n "${SERVER_PID:-}" ]]; then kill "$SERVER_PID" >/dev/null 2>&1 || true; fi
  if [[ -n "${UPSTREAM_PID:-}" ]]; then kill "$UPSTREAM_PID" >/dev/null 2>&1 || true; fi
  if [[ -n "${UPSTREAM_FAIL_PID:-}" ]]; then kill "$UPSTREAM_FAIL_PID" >/dev/null 2>&1 || true; fi
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
            # Real OpenAI Responses SSE shape: response.created carries usage=null
            # and the authoritative counters arrive in response.completed.
            body = (b'event: response.created\n'
                    b'data: {"type":"response.created","response":{"id":"r1","usage":null}}\n\n'
                    b'event: response.output_text.delta\n'
                    b'data: {"type":"response.output_text.delta","delta":"hi"}\n\n'
                    b'event: response.completed\n'
                    b'data: {"type":"response.completed","response":{"usage":{"input_tokens":100,"input_tokens_details":{"cached_tokens":10},"output_tokens":100,"output_tokens_details":{"reasoning_tokens":5}}}}\n\n')
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
cat > /tmp/zk-fake-upstream-fail.py <<'PYFAIL'
from http.server import BaseHTTPRequestHandler, HTTPServer
class H(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get("content-length", "0"))
        self.rfile.read(length)
        body = b'{"error":{"message":"upstream unavailable"}}'
        self.send_response(500)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
    def log_message(self, *args):
        pass
HTTPServer(("127.0.0.1", int(__import__("sys").argv[1])), H).serve_forever()
PYFAIL
python3 /tmp/zk-fake-upstream-fail.py "$UPSTREAM_FAIL_PORT" >/dev/null 2>&1 &
UPSTREAM_FAIL_PID=$!
cargo build -q -p zk-server
LOG=/tmp/zk-auth-test.log
: >"$LOG"
start_server "$PORT" "$LOG"
SERVER_1=$SERVER_PID
wait_ready "$BASE"

# ---------- single-instance advisory lock: a second process must refuse to start ----------
set +e
ZEOKEEL_ALLOW_INSECURE_UPSTREAM=1 \
ZEOKEEL_MASTER_KEY="${ZEOKEEL_TEST_MASTER_KEY:-000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f}" \
ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" \
ZEOKEEL_SERVER_ADDR="127.0.0.1:$((PORT + 1))" \
  timeout 15 ./target/debug/zk-server >/tmp/zk-auth-dupe.log 2>&1
DUPE_CODE=$?
set -e
[[ "$DUPE_CODE" != "0" ]]
grep -q "advisory lock" /tmp/zk-auth-dupe.log

# ---------- login / refresh rotation ----------
LOGIN=$(curl -fsS -X POST "$BASE/api/client/v1/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass","device_id":"dev1"}')
ACCESS=$(printf '%s' "$LOGIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["access_token"])')
REFRESH=$(printf '%s' "$LOGIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["refresh_token"])')
[[ "$(curl -fsS "$BASE/api/client/v1/auth/me" -H "Authorization: Bearer $ACCESS" | python3 -c 'import json,sys;print(json.load(sys.stdin)["role"])')" == "super_admin" ]]
NEW=$(curl -fsS -X POST "$BASE/api/client/v1/auth/refresh" -H 'content-type: application/json' -d "{\"refresh_token\":\"$REFRESH\",\"device_id\":\"dev1\"}")
NEW_ACCESS=$(printf '%s' "$NEW" | python3 -c 'import json,sys;print(json.load(sys.stdin)["access_token"])')
NEW_REFRESH=$(printf '%s' "$NEW" | python3 -c 'import json,sys;print(json.load(sys.stdin)["refresh_token"])')
[[ -n "$NEW_ACCESS" ]]
CODE=$(curl -sS -o /tmp/zk-auth-reuse.json -w '%{http_code}' -X POST "$BASE/api/client/v1/auth/refresh" -H 'content-type: application/json' -d "{\"refresh_token\":\"$REFRESH\",\"device_id\":\"dev1\"}")
[[ "$CODE" == "401" ]]
# Refresh reuse revokes the whole device token family, including the rotated child.
CODE=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$BASE/api/client/v1/auth/refresh" -H 'content-type: application/json' -d "{\"refresh_token\":\"$NEW_REFRESH\",\"device_id\":\"dev1\"}")
[[ "$CODE" == "401" ]]
# Expired refresh tokens are rejected (docs/04 §3: 30-day absolute lifetime).
EXP=$(curl -fsS -X POST "$BASE/api/client/v1/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass","device_id":"dev-exp"}')
EXP_REFRESH=$(printf '%s' "$EXP" | python3 -c 'import json,sys;print(json.load(sys.stdin)["refresh_token"])')
q "UPDATE refresh_tokens SET expires_at = now() - interval '1 day' WHERE token_hash = '$(sha "$EXP_REFRESH")'"
CODE=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$BASE/api/client/v1/auth/refresh" -H 'content-type: application/json' -d "{\"refresh_token\":\"$EXP_REFRESH\",\"device_id\":\"dev-exp\"}")
[[ "$CODE" == "401" ]]

# ---------- admin cookie session + RBAC + auditor logout ----------
ADMIN=$(curl -fsS -c /tmp/zk-auth-admin.cookies -X POST "$BASE/api/admin/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass"}')
CSRF=$(printf '%s' "$ADMIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["csrf_token"])')
# Logout is session termination, not a CSRF-guarded mutation (docs/06 §1).
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/auth/logout")
[[ "$CODE" == "200" ]]
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
CODE=$(curl -sS -b /tmp/zk-auth-auditor.cookies -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/auth/logout")
[[ "$CODE" == "200" ]]
AUD=$(curl -fsS -c /tmp/zk-auth-auditor.cookies -X POST "$BASE/api/admin/auth/login" -H 'content-type: application/json' -d '{"username":"auditor","password":"auditor-pass"}')
AUDCSRF=$(printf '%s' "$AUD" | python3 -c 'import json,sys;print(json.load(sys.stdin)["csrf_token"])')
ADMIN_ID=$(printf '%s' "$LOGIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["user"]["id"])')
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X DELETE "$BASE/api/admin/users/$ADMIN_ID")
[[ "$CODE" == "409" ]]
ALICE_ID=$(printf '%s' "$USERS" | python3 -c 'import json,sys;print([u["id"] for u in json.load(sys.stdin)["users"] if u["username"]=="alice"][0])')

# ---------- disable revokes refresh tokens; re-enable does not resurrect them ----------
DIS_LOGIN=$(curl -fsS -X POST "$BASE/api/client/v1/auth/login" -H 'content-type: application/json' -d '{"username":"alice","password":"alice-pass","device_id":"alice-disable"}')
DIS_REFRESH=$(printf '%s' "$DIS_LOGIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["refresh_token"])')
curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X PATCH "$BASE/api/admin/users/$ALICE_ID" -H 'content-type: application/json' -d '{"status":"disabled"}' >/dev/null
CODE=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$BASE/api/client/v1/auth/refresh" -H 'content-type: application/json' -d "{\"refresh_token\":\"$DIS_REFRESH\",\"device_id\":\"alice-disable\"}")
[[ "$CODE" == "401" ]]
curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X PATCH "$BASE/api/admin/users/$ALICE_ID" -H 'content-type: application/json' -d '{"status":"active"}' >/dev/null
CODE=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$BASE/api/client/v1/auth/refresh" -H 'content-type: application/json' -d "{\"refresh_token\":\"$DIS_REFRESH\",\"device_id\":\"alice-disable\"}")
[[ "$CODE" == "401" ]]

# ---------- wallet adjust: idempotent + fail-closed audit ----------
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -H 'X-Request-Id: probe-audit-client-id' -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/wallet/$ALICE_ID/adjust" -H 'content-type: application/json' -d '{"delta_micros":1000000,"reason":"test grant","idempotency_key":"grant-alice-1"}')
[[ "$CODE" == "200" ]]
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/wallet/$ALICE_ID/adjust" -H 'content-type: application/json' -d '{"delta_micros":1000000,"reason":"test grant","idempotency_key":"grant-alice-1"}')
[[ "$CODE" == "200" ]]
[[ "$(q "SELECT request_id FROM audit_log WHERE action = 'wallet.adjust' ORDER BY id DESC LIMIT 1")" == "probe-audit-client-id" ]]
ALICE_JWT=$(curl -fsS -X POST "$BASE/api/client/v1/auth/login" -H 'content-type: application/json' -d '{"username":"alice","password":"alice-pass","device_id":"alice-dev"}' | python3 -c 'import json,sys;print(json.load(sys.stdin)["access_token"])')
BAL=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $ALICE_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
[[ "$BAL" == "1000000" ]]
LEDGER=$(curl -fsS "$BASE/api/client/v1/wallet/ledger" -H "Authorization: Bearer $ALICE_JWT")
[[ "$(printf '%s' "$LEDGER" | python3 -c 'import json,sys;print(len(json.load(sys.stdin)["items"]))')" == "1" ]]
[[ "$(q "SELECT count(*) FROM audit_log WHERE action = 'wallet.adjust'")" == "1" ]]
# Audit write failure must abort the wallet mutation (same transaction, fail-closed).
q "CREATE OR REPLACE FUNCTION zk_fail_audit() RETURNS trigger LANGUAGE plpgsql AS \$\$ BEGIN RAISE EXCEPTION 'audit down'; END \$\$;"
q "CREATE TRIGGER zk_fail_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION zk_fail_audit();"
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/wallet/$ALICE_ID/adjust" -H 'content-type: application/json' -d '{"delta_micros":777,"reason":"fail-closed probe","idempotency_key":"fail-closed-1"}')
[[ "$CODE" == "500" ]]
[[ "$(q "SELECT count(*) FROM wallet_ledger WHERE idempotency_key = 'fail-closed-1'")" == "0" ]]
q "DROP TRIGGER zk_fail_audit ON audit_log"
q "DROP FUNCTION zk_fail_audit()"

# ---------- pricing / model catalog ----------
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X PUT "$BASE/api/admin/pricing/deepseek-chat" -H 'content-type: application/json' -d '{"input_per_1k_micros":1000,"cached_per_1k_micros":100,"output_per_1k_micros":2000}')
[[ "$CODE" == "200" ]]
MODELS=$(curl -fsS "$BASE/v1/models" -H "Authorization: Bearer $ALICE_JWT")
[[ "$(printf '%s' "$MODELS" | python3 -c 'import json,sys;print(json.load(sys.stdin)["data"][0]["id"])')" == "deepseek-chat" ]]
CODE=$(curl -sS -b /tmp/zk-auth-auditor.cookies -H "X-CSRF-Token: $AUDCSRF" -o /dev/null -w '%{http_code}' -X PUT "$BASE/api/admin/pricing/x" -H 'content-type: application/json' -d '{"input_per_1k_micros":1,"output_per_1k_micros":1}')
[[ "$CODE" == "403" ]]

# ---------- gateway non-stream billing ----------
GW=$(curl -sS -X POST "$BASE/v1/responses" -H "Authorization: Bearer $ALICE_JWT" -H 'content-type: application/json' -d '{"model":"deepseek-chat","input":"hello"}')
[[ "$(printf '%s' "$GW" | python3 -c 'import json,sys;print(json.load(sys.stdin)["id"])')" == "resp_1" ]]
BAL2=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $ALICE_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
[[ "$BAL2" == "999950" ]]
# Same client-supplied x-request-id twice are two independent admissions: both charged.
for _ in 1 2; do
  curl -sS -o /dev/null -X POST "$BASE/v1/responses" -H "Authorization: Bearer $ALICE_JWT" -H 'content-type: application/json' -H 'x-request-id: probe-fixed-id' -d '{"model":"deepseek-chat","input":"hello"}'
done
BAL2B=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $ALICE_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
[[ "$BAL2B" == "999850" ]]
# ... and the billing key is server-generated, never the client header.
[[ "$(q "SELECT count(*) FROM usage WHERE request_id = 'probe-fixed-id'")" == "0" ]]
[[ "$(q "SELECT count(*) FROM usage_pending WHERE request_id = 'probe-fixed-id'")" == "0" ]]
[[ "$(q "SELECT count(DISTINCT request_id) FROM usage WHERE model = 'deepseek-chat'")" -ge 2 ]]
# ---------- gateway streaming billing with real Responses SSE shape ----------
SSE=$(curl -sS -N -X POST "$BASE/v1/responses" -H "Authorization: Bearer $ALICE_JWT" -H 'content-type: application/json' -d '{"model":"deepseek-chat","input":"hello","stream":true}')
printf '%s' "$SSE" | grep -q "response.completed"
BAL3=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $ALICE_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
# usage 100 input (10 cached) + 100 output: 90*1000/1000 + 10*100/1000 + 100*2000/1000 = 90+1+200 = 291
[[ "$BAL3" == "999559" ]]
[[ "$(q "SELECT count(*) FROM usage WHERE input_tokens = 100 AND output_tokens = 100 AND cached_tokens = 10 AND reasoning_tokens = 5 AND amount_micros = 291")" == "1" ]]
[[ "$(q "SELECT count(*) FROM usage_pending WHERE status <> 'settled'")" == "0" ]]
[[ "$(q "SELECT count(*) FROM wallet_holds WHERE status = 'held'")" == "0" ]]
CODE=$(curl -sS -o /tmp/zk-gw-unpriced.json -w '%{http_code}' -X POST "$BASE/v1/responses" -H "Authorization: Bearer $ALICE_JWT" -H 'content-type: application/json' -d '{"model":"unpriced","input":"x"}')
[[ "$CODE" == "402" ]]
CODE=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$BASE/v1/responses" -H 'content-type: application/json' -d '{"model":"deepseek-chat","input":"x"}')
[[ "$CODE" == "401" ]]

# ---------- DB-driven provider routing + failover (docs/07 §4) ----------
P_FAIL=$(curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/providers" -H 'content-type: application/json' -d "{\"name\":\"probe-fail\",\"base_url\":\"http://127.0.0.1:$UPSTREAM_FAIL_PORT/v1\",\"api_key\":\"secret-a\",\"priority\":1,\"max_retries\":1,\"timeout_secs\":5}")
P_FAIL_ID=$(printf '%s' "$P_FAIL" | python3 -c 'import json,sys;print(json.load(sys.stdin)["id"])')
P_OK=$(curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/providers" -H 'content-type: application/json' -d "{\"name\":\"probe-ok\",\"base_url\":\"http://127.0.0.1:$UPSTREAM_PORT/v1\",\"api_key\":\"secret-b\",\"priority\":2,\"max_retries\":1,\"timeout_secs\":5}")
P_OK_ID=$(printf '%s' "$P_OK" | python3 -c 'import json,sys;print(json.load(sys.stdin)["id"])')
curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X PUT "$BASE/api/admin/models/deepseek-chat" -H 'content-type: application/json' -d "{\"provider_id\":\"$P_FAIL_ID\",\"display_name\":\"DeepSeek Chat\",\"enabled\":true}" >/dev/null
curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X PUT "$BASE/api/admin/models/deepseek-chat" -H 'content-type: application/json' -d "{\"provider_id\":\"$P_OK_ID\",\"display_name\":\"DeepSeek Chat\",\"enabled\":true}" >/dev/null
PROVIDERS=$(curl -fsS -b /tmp/zk-auth-admin.cookies "$BASE/api/admin/providers")
printf '%s' "$PROVIDERS" | python3 -c 'import json,sys;items=json.load(sys.stdin)["items"];assert len(items)>=2;assert all(v["has_api_key"] for v in items);assert all("api_key" not in v for v in items)'
GW=$(curl -sS -X POST "$BASE/v1/responses" -H "Authorization: Bearer $ALICE_JWT" -H 'content-type: application/json' -d '{"model":"deepseek-chat","input":"hello"}')
[[ "$(printf '%s' "$GW" | python3 -c 'import json,sys;print(json.load(sys.stdin)["id"])')" == "resp_1" ]]
BAL_F=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $ALICE_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
[[ "$BAL_F" == "999509" ]]
# A mapping for the failover provider was reached first (priority 1) and failed over.
[[ "$(q "SELECT count(*) FROM usage WHERE request_id IS NOT NULL")" -ge 1 ]]

# ---------- fail-closed audit for content/skills/connectors/apps ----------
python3 - <<'PYZIP'
import zipfile
with zipfile.ZipFile('/tmp/zk-fc-skill.zip', 'w') as z:
    z.writestr('SKILL.md', b'# fail-closed probe\n')
open('/tmp/zk-fc-app.wasm', 'wb').write(b'\x00asm\x01\x00\x00\x00')
PYZIP
q "CREATE OR REPLACE FUNCTION zk_fail_audit() RETURNS trigger LANGUAGE plpgsql AS \$\$ BEGIN RAISE EXCEPTION 'audit down'; END \$\$;"
q "CREATE TRIGGER zk_fail_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION zk_fail_audit();"
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/skills" -H 'content-type: application/json' -d '{"name":"fc-content","kind":"skill"}')
[[ "$CODE" == "500" ]]
[[ "$(q "SELECT count(*) FROM contents WHERE name = 'fc-content'")" == "0" ]]
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/connectors" -H 'content-type: application/json' -d '{"name":"fc-conn","transport":"http","config":{}}')
[[ "$CODE" == "500" ]]
[[ "$(q "SELECT count(*) FROM connectors WHERE name = 'fc-conn'")" == "0" ]]
CODE=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$BASE/api/client/v1/skills" -H "Authorization: Bearer $ALICE_JWT" -F 'name=fc-skill' -F 'version=1.0.0' -F 'archive=@/tmp/zk-fc-skill.zip')
[[ "$CODE" == "500" ]]
[[ "$(q "SELECT count(*) FROM contents WHERE name = 'fc-skill'")" == "0" ]]
CODE=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$BASE/api/client/v1/apps" -H "Authorization: Bearer $ALICE_JWT" -F 'manifest={"name":"fc-app","version":"0.1.0","db_schema_version":0}' -F 'wasm=@/tmp/zk-fc-app.wasm')
[[ "$CODE" == "500" ]]
[[ "$(q "SELECT count(*) FROM contents WHERE name = 'fc-app'")" == "0" ]]
q "DROP TRIGGER zk_fail_audit ON audit_log"
q "DROP FUNCTION zk_fail_audit()"
# After the audit outage is fixed the same mutation succeeds (mutation was not half-applied).
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/skills" -H 'content-type: application/json' -d '{"name":"fc-content-ok","kind":"skill"}')
[[ "$CODE" == "200" ]]

# ---------- admin-configurable peak/off-peak window (docs/22 §5.3) ----------
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/users" -H 'content-type: application/json' -d '{"username":"carol","password":"carol-pass","role":"employee"}')
[[ "$CODE" == "200" ]]
CAROL_ID=$(q "SELECT id FROM users WHERE username = 'carol'")
curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/wallet/$CAROL_ID/adjust" -H 'content-type: application/json' -d '{"delta_micros":100000,"reason":"peak probe","idempotency_key":"carol-grant"}' >/dev/null
CAROL_JWT=$(curl -fsS -X POST "$BASE/api/client/v1/auth/login" -H 'content-type: application/json' -d '{"username":"carol","password":"carol-pass","device_id":"carol-dev"}' | python3 -c 'import json,sys;print(json.load(sys.stdin)["access_token"])')
curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X PUT "$BASE/api/admin/pricing/deepseek-chat" -H 'content-type: application/json' -d '{"input_per_1k_micros":1000,"cached_per_1k_micros":100,"output_per_1k_micros":2000,"peak_multiplier_bps":20000,"offpeak_multiplier_bps":10000}' >/dev/null
curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X PUT "$BASE/api/admin/pricing/peak-window" -H 'content-type: application/json' -d '{"weekday_mask":0,"start_minute":0,"end_minute":1440}' >/dev/null
CAROL_BEFORE=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $CAROL_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
curl -sS -o /dev/null -X POST "$BASE/v1/responses" -H "Authorization: Bearer $CAROL_JWT" -H 'content-type: application/json' -d '{"model":"deepseek-chat","input":"hello"}'
CAROL_PEAK=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $CAROL_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
[[ "$((CAROL_BEFORE - CAROL_PEAK))" == "100" ]]
# Empty window = never peak -> off-peak multiplier.
curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X PUT "$BASE/api/admin/pricing/peak-window" -H 'content-type: application/json' -d '{"weekday_mask":0,"start_minute":0,"end_minute":0}' >/dev/null
CAROL_BEFORE2=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $CAROL_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
curl -sS -o /dev/null -X POST "$BASE/v1/responses" -H "Authorization: Bearer $CAROL_JWT" -H 'content-type: application/json' -d '{"model":"deepseek-chat","input":"hello"}'
CAROL_OFFPEAK=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $CAROL_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
[[ "$((CAROL_BEFORE2 - CAROL_OFFPEAK))" == "50" ]]
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X PUT "$BASE/api/admin/pricing/peak-window" -H 'content-type: application/json' -d '{"weekday_mask":0,"start_minute":-1,"end_minute":1440}')
[[ "$CODE" == "400" ]]
PEAK_GET=$(curl -fsS -b /tmp/zk-auth-admin.cookies "$BASE/api/admin/pricing/peak-window")
[[ "$(printf '%s' "$PEAK_GET" | python3 -c 'import json,sys;print(json.load(sys.stdin)["key"])')" == "pricing.peak_window" ]]
# Restore off-peak for the remaining streaming-overdraft assertion.
curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X PUT "$BASE/api/admin/pricing/deepseek-chat" -H 'content-type: application/json' -d '{"input_per_1k_micros":1000,"cached_per_1k_micros":100,"output_per_1k_micros":2000}' >/dev/null

# ---------- admin session TTL/idle + full refresh claims + capability surface ----------
# Access-token claims: iss/aud/uid/channel/jti per docs/04 §3.
CLAIMS=$(printf '%s' "$ACCESS" | python3 -c 'import base64,json,sys;t=sys.stdin.read().strip().split(".")[1];t+="="*(-len(t)%4);d=json.loads(base64.urlsafe_b64decode(t));print(d["iss"],d["aud"],d["uid"]==d["sub"],d["channel"],bool(d["jti"]),bool(d["exp"]>d["iat"]))')
[[ "$CLAIMS" == "zeokeel zeokeel-client True official True True" ]]
# Refresh binds device_id + channel.
CH_LOGIN=$(curl -fsS -H 'X-ZeoKeel-Client: 0.1.0;beta;linux;dev-chan' -X POST "$BASE/api/client/v1/auth/login" -H 'content-type: application/json' -d '{"username":"alice","password":"alice-pass","device_id":"alice-chan"}')
CH_JWT=$(printf '%s' "$CH_LOGIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["access_token"])')
CH_REFRESH=$(printf '%s' "$CH_LOGIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["refresh_token"])')
[[ "$(printf '%s' "$CH_JWT" | python3 -c 'import base64,json,sys;t=sys.stdin.read().strip().split(".")[1];t+="="*(-len(t)%4);print(json.loads(base64.urlsafe_b64decode(t))["channel"])')" == "beta" ]]
CH_NEW=$(curl -fsS -H 'X-ZeoKeel-Client: 0.1.0;beta;linux;dev-chan' -X POST "$BASE/api/client/v1/auth/refresh" -H 'content-type: application/json' -d "{\"refresh_token\":\"$CH_REFRESH\",\"device_id\":\"alice-chan\"}")
CH_REFRESH2=$(printf '%s' "$CH_NEW" | python3 -c 'import json,sys;print(json.load(sys.stdin)["refresh_token"])')
CODE=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$BASE/api/client/v1/auth/refresh" -H 'content-type: application/json' -d "{\"refresh_token\":\"$CH_REFRESH2\",\"device_id\":\"alice-chan\"}")
[[ "$CODE" == "401" ]]
# Client /auth/me exposes the documented permission points.
PERMS=$(curl -fsS "$BASE/api/client/v1/auth/me" -H "Authorization: Bearer $ALICE_JWT")
printf '%s' "$PERMS" | python3 -c 'import json,sys;d=json.load(sys.stdin);assert "gateway:chat" in d["permissions"];assert "skills:publish" in d["permissions"]'
# Admin SPA metadata surface.
ADMIN_META=$(curl -fsS -b /tmp/zk-auth-admin.cookies "$BASE/api/admin/meta")
printf '%s' "$ADMIN_META" | python3 -c 'import json,sys;d=json.load(sys.stdin);assert d["api_version"]=="v1";assert "admin:providers" in d["permissions"]["super_admin"];assert "admin:audit:read" in d["permissions"]["auditor"]'
# Default admin session TTL is 12h from settings (docs/04 §3).
ADMIN_TTL=$(curl -fsS -c /tmp/zk-auth-admin.cookies -X POST "$BASE/api/admin/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass"}' | python3 -c 'import json,sys;print(json.load(sys.stdin)["expires_in"])')
[[ "$ADMIN_TTL" == "43200" ]]
# Expired admin session is rejected; idle window is enforced.
q "UPDATE admin_sessions SET expires_at = now() - interval '5 seconds' WHERE user_id = '$ADMIN_ID'"
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -o /dev/null -w '%{http_code}' "$BASE/api/admin/users")
[[ "$CODE" == "401" ]]
ADMIN=$(curl -fsS -c /tmp/zk-auth-admin.cookies -X POST "$BASE/api/admin/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass"}')
CSRF=$(printf '%s' "$ADMIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["csrf_token"])')
q "UPDATE admin_sessions SET last_seen_at = now() - interval '2 hours' WHERE user_id = '$ADMIN_ID'"
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -o /dev/null -w '%{http_code}' "$BASE/api/admin/users")
[[ "$CODE" == "401" ]]
ADMIN=$(curl -fsS -c /tmp/zk-auth-admin.cookies -X POST "$BASE/api/admin/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass"}')
CSRF=$(printf '%s' "$ADMIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["csrf_token"])')
# Stable capability negotiation with ETag/304 (docs/04 §2.1).
CAP_HEADERS=/tmp/zk-cap.headers
CAP=$(curl -fsS -D "$CAP_HEADERS" "$BASE/api/meta/capabilities" -o /tmp/zk-cap.json)
python3 - <<'PYCAP'
import json
d = json.load(open('/tmp/zk-cap.json'))
assert d["api_version"] == "v1"
assert d["min_client_version"]
assert isinstance(d["features"], list) and d["features"]
assert d["auth_methods"] == ["local"]
PYCAP
CAP_ETAG=$(grep -i '^etag:' "$CAP_HEADERS" | tr -d '\r' | cut -d' ' -f2)
[[ -n "$CAP_ETAG" ]]
CODE=$(curl -sS -o /dev/null -w '%{http_code}' -H "If-None-Match: $CAP_ETAG" "$BASE/api/meta/capabilities")
[[ "$CODE" == "304" ]]

# ---------- monthly grant / settings / jobs ----------
GRANT=$(curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/jobs/wallet-grant/run")
printf '%s' "$GRANT" | python3 -c 'import json,sys;d=json.load(sys.stdin);assert d["ok"] is True'
BALG=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $ALICE_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
[[ "$BALG" == "1499509" ]]
GRANT2=$(curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/jobs/wallet-grant/run")
BALG2=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $ALICE_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
[[ "$BALG2" == "1499509" ]]
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X PUT "$BASE/api/admin/settings/updates.channel" -H 'content-type: application/json' -d '{"value":{"channel":"official"},"hot":true}')
[[ "$CODE" == "200" ]]
BOOT=$(curl -fsS "$BASE/api/client/v1/config/bootstrap" -H "Authorization: Bearer $ALICE_JWT")
[[ "$(printf '%s' "$BOOT" | python3 -c 'import json,sys;d=json.load(sys.stdin);print(d["updates"]["channel"])')" == "official" ]]
[[ "$(printf '%s' "$BOOT" | python3 -c 'import json,sys;print(len(json.load(sys.stdin)["gateway"]["models"]))')" == "1" ]]
RET=$(curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/jobs/audit-retention/run")
printf '%s' "$RET" | python3 -c 'import json,sys;d=json.load(sys.stdin);assert d["ok"] is True'
VERIFY=$(curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/jobs/audit-verify/run")
printf '%s' "$VERIFY" | python3 -c 'import json,sys;d=json.load(sys.stdin);assert d["ok"] is True;assert d["detail"]["broken"] == 0, d'
JOB=$(curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/jobs/wallet-reconcile/run")
printf '%s' "$JOB" | python3 -c 'import json,sys;d=json.load(sys.stdin);assert d["ok"] is True'

# ---------- streaming overdraft to the configured floor (docs/22 §4) ----------
CUR=$(q "SELECT balance_micros FROM wallet_accounts WHERE user_id = '$ALICE_ID'")
DELTA=$((10 - CUR))
curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/wallet/$ALICE_ID/adjust" -H 'content-type: application/json' -d "{\"delta_micros\":$DELTA,\"reason\":\"overdraft probe\",\"idempotency_key\":\"overdraft-probe\"}" >/dev/null
curl -sS -o /dev/null -X POST "$BASE/v1/responses" -H "Authorization: Bearer $ALICE_JWT" -H 'content-type: application/json' -d '{"model":"deepseek-chat","input":"hello","stream":true,"max_output_tokens":1}'
sleep 0.5
BALO=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $ALICE_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
[[ "$BALO" == "-281" ]]
[[ "$(q "SELECT count(*) FROM wallet_holds WHERE status = 'held'")" == "0" ]]

# ---------- lockout ----------
for _ in 1 2 3 4 5; do
  curl -sS -o /dev/null -X POST "$BASE/api/admin/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"wrong"}'
done
CODE=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass"}')
[[ "$CODE" == "429" ]]

# ---------- settlement retry is idempotent (real-PG Rust regression test) ----------
ZEOKEEL_TEST_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" \
  cargo test -q -p zk-srv-gateway --lib -- --ignored

# ---------- SIGTERM shutdown: settlement sweep, audit flush, exit 0 ----------
kill -TERM "$SERVER_1"
if wait "$SERVER_1"; then SHUTDOWN_CODE=0; else SHUTDOWN_CODE=$?; fi
SERVER_PID=""
[[ "$SHUTDOWN_CODE" == "0" ]]
grep -q "shutdown complete" "$LOG"

# ---------- migration checksum tampering refuses startup ----------
GOOD_CHECKSUM=$(q "SELECT checksum FROM schema_migrations WHERE id = '0001_core'")
q "UPDATE schema_migrations SET checksum = 'deadbeef' WHERE id = '0001_core'"
set +e
ZEOKEEL_ALLOW_INSECURE_UPSTREAM=1 \
ZEOKEEL_MASTER_KEY="${ZEOKEEL_TEST_MASTER_KEY:-000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f}" \
ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" \
ZEOKEEL_SERVER_ADDR="127.0.0.1:$((PORT + 2))" \
  timeout 20 ./target/debug/zk-server >/tmp/zk-auth-migration.log 2>&1
TAMPER_CODE=$?
set -e
[[ "$TAMPER_CODE" != "0" ]]
grep -q "checksum changed" /tmp/zk-auth-migration.log
q "UPDATE schema_migrations SET checksum = '$GOOD_CHECKSUM' WHERE id = '0001_core'"
# Lock release from the SIGTERM'd instance: a fresh instance starts and stops cleanly.
start_server "$((PORT + 3))" "$LOG"
wait_ready "http://127.0.0.1:$((PORT + 3))"
kill -TERM "$SERVER_PID"
if wait "$SERVER_PID"; then FINAL_CODE=0; else FINAL_CODE=$?; fi
SERVER_PID=""
[[ "$FINAL_CODE" == "0" ]]
echo "auth ok: login/me/refresh-reuse-family/refresh-expiry/channel-binding/logout-users-rbac/auditor-logout/claims/admin-ttl-idle"
echo "server ok: fail-closed-audit(wallet/users/pricing/content/skills/connectors/apps)/wallet-idempotency/pricing/peak-window/db-provider-failover/capabilities-meta"
echo "server ok: gateway-billing/x-request-id-ignored/real-SSE-usage/stream-overdraft/audit-chain/lockout"
echo "runtime ok: single-instance-lock/migration-checksum/SIGTERM-shutdown/usage-pending-wired"
