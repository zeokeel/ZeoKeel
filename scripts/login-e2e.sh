#!/usr/bin/env bash
# Enterprise login e2e: enterprise compose + real auth server + serve UI.
set -euo pipefail
CLIENT_HDR="X-ZeoKeel-Client: 0.1.0;official;linux;e2e"
DB="zk_login_e2e_$$"; ROLE="zk_login_role_$$"
SERVER_PORT="${ZEOKEEL_LOGIN_SERVER_PORT:-8795}"; UI_PORT="${ZEOKEEL_LOGIN_UI_PORT:-8789}"; UPSTREAM_PORT="${ZEOKEEL_LOGIN_UPSTREAM_PORT:-8796}"
# SIGTERM then bounded wait, fallback SIGKILL; DB/role are dropped only after exit.
stop_pid() {
  local pid="$1"
  [[ -n "$pid" ]] || return 0
  kill "$pid" 2>/dev/null || true
  for _ in $(seq 1 40); do kill -0 "$pid" 2>/dev/null || break; sleep 0.25; done
  if kill -0 "$pid" 2>/dev/null; then kill -KILL "$pid" 2>/dev/null || true; fi
  wait "$pid" 2>/dev/null || true
}
stop_server() { stop_pid "${SERVER_PID:-}"; SERVER_PID=""; }
cleanup() {
  stop_pid "${UI_PID:-}"; stop_pid "${UPSTREAM_PID:-}"; stop_pid "${OAUTH_PID:-}"
  stop_server
  sudo -u postgres dropdb --if-exists --force "$DB" >/dev/null 2>&1 || true
  sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true
  # docs/24 §1.2: never leave a non-personal composition in the committed baseline.
  bash scripts/restore-baseline.sh >/dev/null 2>&1 || true
}
trap cleanup EXIT
cat > /tmp/zk-login-upstream.py <<'PYUP'
from http.server import BaseHTTPRequestHandler, HTTPServer
class H(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get("content-length", "0"))
        raw = self.rfile.read(length)
        if b'"stream":true' in raw or b'"stream": true' in raw:
            body = b'data: {"type":"response.output_text.delta","delta":"gateway "}\n\ndata: {"type":"response.output_text.delta","delta":"hello"}\n\ndata: {"type":"response.completed","response":{"usage":{"input_tokens":10,"output_tokens":20}}}\n\ndata: [DONE]\n\n'
            content_type = "text/event-stream"
        else:
            body = b'{"id":"resp_gateway","output":[{"content":[{"type":"output_text","text":"gateway hello"}]}],"usage":{"input_tokens":10,"output_tokens":20}}'
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
python3 /tmp/zk-login-upstream.py "$UPSTREAM_PORT" >/dev/null 2>&1 & UPSTREAM_PID=$!
cat > /tmp/zk-oauth-server.py <<'PYOAUTH'
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse, parse_qs, urlencode
class H(BaseHTTPRequestHandler):
    def cors(self):
        self.send_header("access-control-allow-origin", "http://127.0.0.1:8789")
        self.send_header("access-control-allow-headers", "content-type")
        self.send_header("access-control-allow-methods", "POST,OPTIONS")
    def do_OPTIONS(self):
        self.send_response(204); self.cors(); self.end_headers()
    def do_GET(self):
        q = parse_qs(urlparse(self.path).query)
        redirect = q.get("redirect_uri", [""])[0]
        state = q.get("state", [""])[0]
        target = redirect + "?" + urlencode({"code": "oauth-code", "state": state})
        self.send_response(302); self.send_header("location", target); self.end_headers()
    def do_POST(self):
        length = int(self.headers.get("content-length", "0"))
        self.rfile.read(length)
        body = b'{"access_token":"oauth-access","refresh_token":"oauth-refresh","expires_in":3600}'
        self.send_response(200); self.cors(); self.send_header("content-type", "application/json"); self.send_header("content-length", str(len(body))); self.end_headers(); self.wfile.write(body)
    def log_message(self, *args): pass
HTTPServer(("127.0.0.1", 8801), H).serve_forever()
PYOAUTH
python3 /tmp/zk-oauth-server.py >/dev/null 2>&1 & OAUTH_PID=$!
cargo run -q -p zk -- compose --edition enterprise --profile default --channel official >/dev/null
cargo build -q -p zk-server -p zk-desktop
pnpm --dir apps/desktop/web build >/dev/null
pnpm --dir apps/admin/web build >/dev/null
sudo -u postgres psql -Atq -c "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';"
sudo -u postgres createdb -O "$ROLE" "$DB"
# Security round-2 fixtures (docs/21 §10): loopback plaintext fake upstream needs the
# explicit dev flag; provider keys are encrypted at rest with an ephemeral test key.
ZEOKEEL_ALLOW_INSECURE_UPSTREAM=1 \
ZEOKEEL_MASTER_KEY="${ZEOKEEL_TEST_MASTER_KEY:-000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f}" \
ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" \
ZEOKEEL_SERVER_ADDR="127.0.0.1:$SERVER_PORT" \
ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD="admin-pass" \
ZEOKEEL_MONTHLY_GRANT_MICROS="500000" \
ZEOKEEL_UPSTREAM_BASE_URL="http://127.0.0.1:$UPSTREAM_PORT/v1" \
ZEOKEEL_UPSTREAM_API_KEY="test-upstream-key" \
ZEOKEEL_DEV_CORS_ORIGIN="http://127.0.0.1:$UI_PORT" \
./target/debug/zk-server >/tmp/zk-login-server.log 2>&1 & SERVER_PID=$!
rm -rf target/evidence/m2/login-home; mkdir -p target/evidence/m2/login-home
for _ in $(seq 1 60); do curl -fsS "http://127.0.0.1:$SERVER_PORT/healthz" >/dev/null 2>&1 && break; sleep 0.5; done
ZEOKEEL_HOME=target/evidence/m2/login-home ZEOKEEL_DEV_KEYCHAIN_FALLBACK=1 ZEOKEEL_SERVE_ADDR="127.0.0.1:$UI_PORT" \
./target/debug/zk-desktop --serve >/tmp/zk-login-desktop.log 2>&1 & UI_PID=$!
for _ in $(seq 1 40); do curl -fsS "http://127.0.0.1:$UI_PORT/healthz" >/dev/null 2>&1 && break; sleep 0.5; done
curl -fsS "http://127.0.0.1:$SERVER_PORT/readyz" >/dev/null
ADMIN=$(curl -fsS -c /tmp/zk-login-admin.cookies -X POST "http://127.0.0.1:$SERVER_PORT/api/admin/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass"}')
CSRF=$(printf '%s' "$ADMIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["csrf_token"])')
curl -fsS -b /tmp/zk-login-admin.cookies -H "X-CSRF-Token: $CSRF" -X PUT "http://127.0.0.1:$SERVER_PORT/api/admin/pricing/deepseek-chat" -H 'content-type: application/json' -d '{"input_per_1k_micros":1000,"output_per_1k_micros":2000}' >/dev/null
USERS=$(curl -fsS -b /tmp/zk-login-admin.cookies "http://127.0.0.1:$SERVER_PORT/api/admin/users")
ADMIN_ID=$(printf '%s' "$USERS" | python3 -c 'import json,sys;print([u["id"] for u in json.load(sys.stdin)["users"] if u["username"]=="admin"][0])')
curl -fsS -b /tmp/zk-login-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "http://127.0.0.1:$SERVER_PORT/api/admin/wallet/$ADMIN_ID/adjust" -H 'content-type: application/json' -d '{"delta_micros":1000000,"reason":"login e2e grant","idempotency_key":"login-e2e-grant"}' >/dev/null
# Seed one approved skill + one connector for the client store e2e.
python3 - <<'PYZIP'
import zipfile
with zipfile.ZipFile('/tmp/zk-client-skill.zip', 'w', zipfile.ZIP_DEFLATED) as z:
    z.writestr('SKILL.md', '# client e2e skill\n')
PYZIP
SKILL=$(curl -fsS -b /tmp/zk-login-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "http://127.0.0.1:$SERVER_PORT/api/admin/skills" -H 'content-type: application/json' -d '{"name":"client-skill"}')
SKILL_ID=$(printf '%s' "$SKILL" | python3 -c 'import json,sys;print(json.load(sys.stdin)["id"])')
curl -fsS -b /tmp/zk-login-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "http://127.0.0.1:$SERVER_PORT/api/admin/skills/$SKILL_ID/approve" >/dev/null
curl -fsS -b /tmp/zk-login-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "http://127.0.0.1:$SERVER_PORT/api/admin/skills/$SKILL_ID/versions" -F "version=1.0.0" -F "archive=@/tmp/zk-client-skill.zip" >/dev/null
curl -fsS -b /tmp/zk-login-admin.cookies -H "X-CSRF-Token: $CSRF" -X PUT "http://127.0.0.1:$SERVER_PORT/api/admin/skills/$SKILL_ID/grants" -H 'content-type: application/json' -d '{"grants":[{"grantee":"all","can_install":true}]}' >/dev/null
CONN=$(curl -fsS -b /tmp/zk-login-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "http://127.0.0.1:$SERVER_PORT/api/admin/connectors" -H 'content-type: application/json' -d '{"name":"client-mcp","transport":"http","config":{"authorize_url":"http://127.0.0.1:8801/authorize","token_url":"http://127.0.0.1:8801/token"}}')
CONN_ID=$(printf '%s' "$CONN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["id"])')
curl -fsS -b /tmp/zk-login-admin.cookies -H "X-CSRF-Token: $CSRF" -X PUT "http://127.0.0.1:$SERVER_PORT/api/admin/connectors/$CONN_ID/grants" -H 'content-type: application/json' -d '{"grants":[{"grantee":"all","can_enable":true}]}' >/dev/null
curl -fsS -b /tmp/zk-login-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "http://127.0.0.1:$SERVER_PORT/api/admin/connectors/$CONN_ID/oauth" -H 'content-type: application/json' -d '{"client_id":"public-client","scopes":["mcp"],"redirect_uri":"http://127.0.0.1:8789/"}' >/dev/null
# Seed one approved app for the client apps panel.
ADMIN_JWT=$(curl -fsS -H "$CLIENT_HDR" -X POST "http://127.0.0.1:$SERVER_PORT/api/client/v1/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass","device_id":"seed-app"}' | python3 -c 'import json,sys;print(json.load(sys.stdin)["access_token"])')
cp experiments/wasm-poc/guest-go/guest-go.wasm /tmp/zk-client-app.wasm
python3 - <<'PYUI'
import json,zipfile
open('/tmp/zk-client-app.json','w').write(json.dumps({"name":"client-app","version":"1.0.0","description":"client e2e","icon":"","db_schema_version":1,"db_migrations":[{"version":1,"sql":"CREATE TABLE IF NOT EXISTS items(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL);"}],"ui":["index.html"]}))
html = """<!doctype html><html><body><h1>client app</h1><pre id=out>waiting</pre><script>
(async()=>{await new Promise(r=>setTimeout(r,500));try{const body=btoa(JSON.stringify({op:'add',name:'client'}));await window.zkAppRequest({method:'POST',path:'/items',body_b64:body,request_id:'a'});const list=await window.zkAppRequest({method:'GET',path:'/items',body_b64:'',request_id:'b'});document.getElementById('out').textContent='rows:'+atob(list.body_b64);}catch(e){document.getElementById('out').textContent='error:'+e;}})();
</script></body></html>"""
with zipfile.ZipFile('/tmp/zk-client-app-ui.zip','w') as z: z.writestr('index.html',html)
PYUI
APP=$(curl -fsS -H "$CLIENT_HDR" -X POST "http://127.0.0.1:$SERVER_PORT/api/client/v1/apps" -H "Authorization: Bearer $ADMIN_JWT" -F "manifest=</tmp/zk-client-app.json" -F "wasm=@/tmp/zk-client-app.wasm" -F "ui=@/tmp/zk-client-app-ui.zip")
APP_ID=$(printf '%s' "$APP" | python3 -c 'import json,sys;print(json.load(sys.stdin)["app_id"])')
curl -fsS -b /tmp/zk-login-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "http://127.0.0.1:$SERVER_PORT/api/admin/apps/$APP_ID/approve" >/dev/null
curl -fsS -b /tmp/zk-login-admin.cookies -H "X-CSRF-Token: $CSRF" -X PUT "http://127.0.0.1:$SERVER_PORT/api/admin/apps/$APP_ID/grants" -H 'content-type: application/json' -d '{"grants":[{"grantee":"all","can_open":true}]}' >/dev/null
# First-run spec must run before any logged-in spec: login persists server_url into
# the shared ZEOKEEL_HOME settings store, which intentionally suppresses first-run.
ZEOKEEL_CLIENT_SKILL_ZIP=/tmp/zk-client-skill.zip ZEOKEEL_LOGIN_URL="http://127.0.0.1:$UI_PORT" ZEOKEEL_LOGIN_SERVER="http://127.0.0.1:$SERVER_PORT" \
  pnpm exec playwright test e2e/m2-login.spec.ts --reporter=line
ZEOKEEL_CLIENT_SKILL_ZIP=/tmp/zk-client-skill.zip ZEOKEEL_LOGIN_URL="http://127.0.0.1:$UI_PORT" ZEOKEEL_LOGIN_SERVER="http://127.0.0.1:$SERVER_PORT" \
  pnpm exec playwright test e2e/m2-gateway-chat.spec.ts e2e/m3-client-content.spec.ts e2e/m4-client-apps.spec.ts --reporter=line
echo "login e2e ok"
