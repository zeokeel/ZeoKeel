#!/usr/bin/env bash
# M4 apps e2e: real PG + server + Go guest upload/approve/open/request/restart/delist.
set -euo pipefail
DB="zk_apps_e2e_$$"; ROLE="zk_apps_e2e_$$"; PORT="${ZEOKEEL_APPS_PORT:-8798}"
BASE="http://127.0.0.1:$PORT"; HOME_DIR="target/evidence/m4/apps-home"
rm -rf "$HOME_DIR"; mkdir -p "$HOME_DIR/app-db"
cleanup() {
  [[ -n "${SERVER_PID:-}" ]] && kill "$SERVER_PID" >/dev/null 2>&1 || true
  sudo -u postgres dropdb --if-exists --force "$DB" >/dev/null 2>&1 || true
  sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true
}
trap cleanup EXIT
sudo -u postgres psql -Atq -c "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';"
sudo -u postgres createdb -O "$ROLE" "$DB"
start_server() {
  ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" \
  ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD=admin-pass ZEOKEEL_APP_DB_DIR="$HOME_DIR/app-db" \
  ./target/debug/zk-server >/tmp/zk-apps.log 2>&1 & SERVER_PID=$!
  for _ in $(seq 1 40); do curl -fsS "$BASE/healthz" >/dev/null 2>&1 && break; sleep 0.25; done
}
start_server
curl -fsS "$BASE/readyz" >/dev/null
ADMIN_JWT=$(curl -fsS -X POST "$BASE/api/client/v1/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass","device_id":"apps-e2e"}' | python3 -c 'import json,sys;print(json.load(sys.stdin)["access_token"])')
ADMIN=$(curl -fsS -c /tmp/zk-apps.cookies -X POST "$BASE/api/admin/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass"}')
CSRF=$(printf '%s' "$ADMIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["csrf_token"])')
GUEST=experiments/wasm-poc/guest-go/guest-go.wasm
[[ -f "$GUEST" ]] || (cd experiments/wasm-poc && ./build.sh >/dev/null)
python3 - <<'PY'
import json,zipfile
manifest={"name":"e2e-app","version":"1.0.0","description":"e2e","icon":"","db_schema_version":1,
 "db_migrations":[{"version":1,"sql":"CREATE TABLE IF NOT EXISTS items(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL);"}],"ui":["index.html"]}
open('/tmp/zk-app-manifest.json','w').write(json.dumps(manifest))
with zipfile.ZipFile('/tmp/zk-app-ui.zip','w') as z: z.writestr('index.html','<!doctype html><title>e2e</title>')
PY
UPLOAD=$(curl -fsS -X POST "$BASE/api/client/v1/apps" -H "Authorization: Bearer $ADMIN_JWT" -F "manifest=</tmp/zk-app-manifest.json" -F "wasm=@$GUEST" -F "ui=@/tmp/zk-app-ui.zip")
APP_ID=$(printf '%s' "$UPLOAD" | python3 -c 'import json,sys;print(json.load(sys.stdin)["app_id"])')
printf 'not-wasm' > /tmp/zk-app-bad.wasm
CODE=$(curl -sS -o /tmp/zk-app-bad.json -w '%{http_code}' -X POST "$BASE/api/client/v1/apps" -H "Authorization: Bearer $ADMIN_JWT" -F "manifest=</tmp/zk-app-manifest.json" -F "wasm=@/tmp/zk-app-bad.wasm"); [[ "$CODE" == "400" ]]
CODE=$(curl -sS -b /tmp/zk-apps.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/apps/$APP_ID/approve"); [[ "$CODE" == "200" ]]
CODE=$(curl -sS -b /tmp/zk-apps.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X PUT "$BASE/api/admin/apps/$APP_ID/grants" -H 'content-type: application/json' -d '{"grants":[{"grantee":"all","can_open":true}]}'); [[ "$CODE" == "200" ]]
curl -fsS "$BASE/api/client/v1/apps" -H "Authorization: Bearer $ADMIN_JWT" | python3 -c 'import json,sys;d=json.load(sys.stdin)["items"];assert len(d)==1 and d[0]["app_id"]'
CODE=$(curl -sS -o /tmp/zk-app-open.json -w '%{http_code}' -X POST "$BASE/api/client/v1/apps/$APP_ID/open" -H "Authorization: Bearer $ADMIN_JWT"); [[ "$CODE" == "200" ]]
BODY=$(printf '{"op":"add","name":"e2e"}' | base64 -w0)
REQ=$(python3 -c "import json;print(json.dumps({'abi':'zeokeel-app/1','app_id':'$APP_ID','method':'POST','path':'/items','query':'','headers':{},'body_b64':'$BODY','request_id':'r1'}))")
curl -fsS -X POST "$BASE/api/client/v1/apps/$APP_ID/request" -H "Authorization: Bearer $ADMIN_JWT" -H 'content-type: application/json' -d "$REQ" | python3 -c 'import json,sys;d=json.load(sys.stdin);assert d["status"]==200'
LIST=$(printf '{"abi":"zeokeel-app/1","app_id":"%s","method":"GET","path":"/items","query":"","headers":{},"body_b64":"","request_id":"r2"}' "$APP_ID")
curl -fsS -X POST "$BASE/api/client/v1/apps/$APP_ID/request" -H "Authorization: Bearer $ADMIN_JWT" -H 'content-type: application/json' -d "$LIST" | python3 -c 'import json,sys,base64;d=json.load(sys.stdin);body=json.loads(base64.b64decode(d["body_b64"]));assert body["rows"][0]["name"]=="e2e"'
# cross-app DB isolation: a second app on a fresh DB must see no rows from app1.
printf '{"name":"e2e-app2","version":"1.0.0","description":"e2e2","icon":"","db_schema_version":1,"db_migrations":[{"version":1,"sql":"CREATE TABLE IF NOT EXISTS items(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL);"}],"ui":[]}' > /tmp/zk-app2-manifest.json
UP2=$(curl -fsS -X POST "$BASE/api/client/v1/apps" -H "Authorization: Bearer $ADMIN_JWT" -F "manifest=</tmp/zk-app2-manifest.json" -F "wasm=@$GUEST")
APP2=$(printf '%s' "$UP2" | python3 -c 'import json,sys;print(json.load(sys.stdin)["app_id"])')
curl -fsS -b /tmp/zk-apps.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/apps/$APP2/approve" >/dev/null
curl -fsS -b /tmp/zk-apps.cookies -H "X-CSRF-Token: $CSRF" -X PUT "$BASE/api/admin/apps/$APP2/grants" -H 'content-type: application/json' -d '{"grants":[{"grantee":"all","can_open":true}]}' >/dev/null
LIST2=$(printf '{"abi":"zeokeel-app/1","app_id":"%s","method":"GET","path":"/items","query":"","headers":{},"body_b64":"","request_id":"r3"}' "$APP2")
curl -fsS -X POST "$BASE/api/client/v1/apps/$APP2/request" -H "Authorization: Bearer $ADMIN_JWT" -H 'content-type: application/json' -d "$LIST2" | python3 -c 'import json,sys,base64;d=json.load(sys.stdin);body=json.loads(base64.b64decode(d["body_b64"]));assert body["rows"]==[]'
# restart and verify persistence
kill "$SERVER_PID"; wait "$SERVER_PID" 2>/dev/null || true
start_server
ADMIN_JWT=$(curl -fsS -X POST "$BASE/api/client/v1/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass","device_id":"apps-e2e-2"}' | python3 -c 'import json,sys;print(json.load(sys.stdin)["access_token"])')
curl -fsS -X POST "$BASE/api/client/v1/apps/$APP_ID/request" -H "Authorization: Bearer $ADMIN_JWT" -H 'content-type: application/json' -d "$LIST" | python3 -c 'import json,sys,base64;d=json.load(sys.stdin);body=json.loads(base64.b64decode(d["body_b64"]));assert body["rows"][0]["name"]=="e2e"'
CODE=$(curl -sS -b /tmp/zk-apps.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/apps/$APP_ID/delist"); [[ "$CODE" == "200" ]]
CODE=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$BASE/api/client/v1/apps/$APP_ID/open" -H "Authorization: Bearer $ADMIN_JWT"); [[ "$CODE" == "403" ]]
CODE=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$BASE/api/client/v1/apps/$APP_ID/request" -H "Authorization: Bearer $ADMIN_JWT" -H 'content-type: application/json' -d "$LIST"); [[ "$CODE" == "410" ]]
echo "apps e2e ok: upload/approve/open/request/restart/delist"
