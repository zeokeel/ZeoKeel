#!/usr/bin/env bash
# M2 P1/P2 server smoke: real PostgreSQL + real server binary + curl assertions.
#
# Covers auth/session lifecycle, fail-closed audit, gateway billing (real OpenAI
# Responses SSE shape, server-side request ids, streaming overdraft), usage_pending
# wiring, single-instance advisory lock, migration checksum refusal and SIGTERM.
set -euo pipefail
DB="zk_auth_test_$$"
ROLE="zk_auth_role_$$"
PORT="${ZEOKEEL_TEST_PORT:-8798}"
UPSTREAM_PORT="${ZEOKEEL_UPSTREAM_TEST_PORT:-8796}"
BASE="http://127.0.0.1:${PORT}"
PSQL=(sudo -u postgres psql -d "$DB" -v ON_ERROR_STOP=1 -Atq)
q() { "${PSQL[@]}" -c "$1"; }
sha() { printf '%s' "$1" | sha256sum | cut -d' ' -f1; }
start_server() {
  local port="$1" log="$2"
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
cargo build -q -p zk-server
LOG=/tmp/zk-auth-test.log
: >"$LOG"
start_server "$PORT" "$LOG"
SERVER_1=$SERVER_PID
wait_ready "$BASE"

# ---------- single-instance advisory lock: a second process must refuse to start ----------
set +e
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
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/wallet/$ALICE_ID/adjust" -H 'content-type: application/json' -d '{"delta_micros":1000000,"reason":"test grant","idempotency_key":"grant-alice-1"}')
[[ "$CODE" == "200" ]]
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/wallet/$ALICE_ID/adjust" -H 'content-type: application/json' -d '{"delta_micros":1000000,"reason":"test grant","idempotency_key":"grant-alice-1"}')
[[ "$CODE" == "200" ]]
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

# ---------- monthly grant / settings / jobs ----------
GRANT=$(curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/jobs/wallet-grant/run")
printf '%s' "$GRANT" | python3 -c 'import json,sys;d=json.load(sys.stdin);assert d["ok"] is True'
BALG=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $ALICE_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
[[ "$BALG" == "1499559" ]]
GRANT2=$(curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/jobs/wallet-grant/run")
BALG2=$(curl -fsS "$BASE/api/client/v1/wallet" -H "Authorization: Bearer $ALICE_JWT" | python3 -c 'import json,sys;print(json.load(sys.stdin)["balance_micros"])')
[[ "$BALG2" == "1499559" ]]
CODE=$(curl -sS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X PUT "$BASE/api/admin/settings/updates.channel" -H 'content-type: application/json' -d '{"value":{"channel":"official"},"hot":true}')
[[ "$CODE" == "200" ]]
BOOT=$(curl -fsS "$BASE/api/client/v1/config/bootstrap" -H "Authorization: Bearer $ALICE_JWT")
[[ "$(printf '%s' "$BOOT" | python3 -c 'import json,sys;d=json.load(sys.stdin);print(d["updates"]["channel"])')" == "official" ]]
[[ "$(printf '%s' "$BOOT" | python3 -c 'import json,sys;print(len(json.load(sys.stdin)["gateway"]["models"]))')" == "1" ]]
RET=$(curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/jobs/audit-retention/run")
printf '%s' "$RET" | python3 -c 'import json,sys;d=json.load(sys.stdin);assert d["ok"] is True'
VERIFY=$(curl -fsS -b /tmp/zk-auth-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/jobs/audit-verify/run")
printf '%s' "$VERIFY" | python3 -c 'import json,sys;d=json.load(sys.stdin);assert d["ok"] is True'
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
echo "auth ok: login/me/refresh-reuse-family/refresh-expiry/logout-users-rbac/auditor-logout"
echo "server ok: fail-closed-audit/wallet-idempotency/pricing/gateway-billing/x-request-id-ignored/real-SSE-usage/stream-overdraft/lockout"
echo "runtime ok: single-instance-lock/migration-checksum/SIGTERM-shutdown/usage-pending-wired"
