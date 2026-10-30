#!/usr/bin/env bash
# M3 content store smoke: real PostgreSQL + server migrations/routes.
set -euo pipefail
DB="zk_store_test_$$"; ROLE="zk_store_role_$$"
PORT="${ZEOKEEL_STORE_TEST_PORT:-8799}"
BASE="http://127.0.0.1:${PORT}"
cleanup() {
  [[ -n "${SERVER_PID:-}" ]] && kill "$SERVER_PID" >/dev/null 2>&1 || true
  sudo -u postgres dropdb --if-exists --force "$DB" >/dev/null 2>&1 || true
  sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true
}
trap cleanup EXIT
sudo -u postgres psql -Atq -c "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';"
sudo -u postgres createdb -O "$ROLE" "$DB"
ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" \
ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" \
ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD="admin-pass" \
./target/debug/zk-server >/tmp/zk-store-test.log 2>&1 & SERVER_PID=$!
for _ in $(seq 1 40); do curl -fsS "$BASE/healthz" >/dev/null 2>&1 && break; sleep 0.25; done
curl -fsS "$BASE/readyz" | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["status"]=="ready"'

LOGIN=$(curl -fsS -X POST "$BASE/api/client/v1/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass","device_id":"store-test"}')
ADMIN_JWT=$(printf '%s' "$LOGIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["access_token"])')
ADMIN=$(curl -fsS -c /tmp/zk-store-admin.cookies -X POST "$BASE/api/admin/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass"}')
CSRF=$(printf '%s' "$ADMIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["csrf_token"])')

CREATE=$(curl -fsS -b /tmp/zk-store-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/skills" -H 'content-type: application/json' -d '{"name":"demo-skill"}')
SKILL_ID=$(printf '%s' "$CREATE" | python3 -c 'import json,sys;print(json.load(sys.stdin)["id"])')
CODE=$(curl -sS -b /tmp/zk-store-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/skills/$SKILL_ID/approve")
[[ "$CODE" == "200" ]]
CODE=$(curl -sS -b /tmp/zk-store-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X PUT "$BASE/api/admin/skills/$SKILL_ID/grants" -H 'content-type: application/json' -d '{"grants":[{"grantee":"all","can_install":true}]}')
[[ "$CODE" == "200" ]]

python3 - <<'PY'
import zipfile
with zipfile.ZipFile('/tmp/zk-skill.zip', 'w', zipfile.ZIP_DEFLATED) as z:
    z.writestr('SKILL.md', '# demo skill\n')
    z.writestr('scripts/run.sh', 'echo ok\n')
with zipfile.ZipFile('/tmp/zk-skill-bad.zip', 'w', zipfile.ZIP_DEFLATED) as z:
    z.writestr('SKILL.md', '# bad\n')
    z.writestr('../evil.txt', 'x')
PY
UP=$(curl -fsS -b /tmp/zk-store-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/skills/$SKILL_ID/versions" -F "version=1.0.0" -F "archive=@/tmp/zk-skill.zip")
SHA=$(printf '%s' "$UP" | python3 -c 'import json,sys;print(json.load(sys.stdin)["sha256"])')
CODE=$(curl -sS -b /tmp/zk-store-admin.cookies -H "X-CSRF-Token: $CSRF" -o /tmp/zk-dup.json -w '%{http_code}' -X POST "$BASE/api/admin/skills/$SKILL_ID/versions" -F "version=1.0.0" -F "archive=@/tmp/zk-skill.zip")
[[ "$CODE" == "409" ]]
CODE=$(curl -sS -b /tmp/zk-store-admin.cookies -H "X-CSRF-Token: $CSRF" -o /tmp/zk-bad.json -w '%{http_code}' -X POST "$BASE/api/admin/skills/$SKILL_ID/versions" -F "version=1.0.1" -F "archive=@/tmp/zk-skill-bad.zip")
[[ "$CODE" == "400" ]]
curl -fsS -D /tmp/zk-archive.headers "$BASE/api/client/v1/skills/demo-skill/1.0.0/archive" -H "Authorization: Bearer $ADMIN_JWT" -o /tmp/zk-downloaded.zip
grep -qi "x-zeokeel-skill-sha256: $SHA" /tmp/zk-archive.headers
python3 -c 'import zipfile; z=zipfile.ZipFile("/tmp/zk-downloaded.zip"); assert z.read("SKILL.md").startswith(b"# demo")'

CATALOG=$(curl -fsS "$BASE/api/client/v1/skills" -H "Authorization: Bearer $ADMIN_JWT")
[[ "$(printf '%s' "$CATALOG" | python3 -c 'import json,sys;print(len(json.load(sys.stdin)["items"]))')" == "1" ]]
[[ "$(printf '%s' "$CATALOG" | python3 -c 'import json,sys;print(json.load(sys.stdin)["items"][0]["version"])')" == "1.0.0" ]]
CODE=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$BASE/api/client/v1/skills/demo-skill/1.0.0/install" -H "Authorization: Bearer $ADMIN_JWT" -H 'content-type: application/json' -d '{"tree_sha256":"'$(printf 'a%.0s' {1..64})'"}')
[[ "$CODE" == "200" ]]
CODE=$(curl -sS -b /tmp/zk-store-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X DELETE "$BASE/api/admin/skills/$SKILL_ID")
[[ "$CODE" == "200" ]]
TOMB=$(curl -fsS "$BASE/api/client/v1/skills/tombstones" -H "Authorization: Bearer $ADMIN_JWT")
[[ "$(printf '%s' "$TOMB" | python3 -c 'import json,sys;print("demo-skill" in json.load(sys.stdin)["items"])')" == "True" ]]
CATALOG=$(curl -fsS "$BASE/api/client/v1/skills" -H "Authorization: Bearer $ADMIN_JWT")
[[ "$(printf '%s' "$CATALOG" | python3 -c 'import json,sys;print(len(json.load(sys.stdin)["items"]))')" == "0" ]]
CONN=$(curl -fsS -b /tmp/zk-store-admin.cookies -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/connectors" -H 'content-type: application/json' -d '{"name":"demo-mcp","transport":"http","config":{}}')
CONN_ID=$(printf '%s' "$CONN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["id"])')
CODE=$(curl -sS -b /tmp/zk-store-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/connectors" -H 'content-type: application/json' -d '{"name":"bad-secret","transport":"http","config":{"api_key":"x"}}')
[[ "$CODE" == "400" ]]
CODE=$(curl -sS -b /tmp/zk-store-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X PUT "$BASE/api/admin/connectors/$CONN_ID/grants" -H 'content-type: application/json' -d '{"grants":[{"grantee":"all","can_enable":true}]}')
[[ "$CODE" == "200" ]]
CODE=$(curl -sS -b /tmp/zk-store-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/connectors/$CONN_ID/oauth" -H 'content-type: application/json' -d '{"client_id":"public-client","scopes":["mcp"],"redirect_uri":"http://127.0.0.1/callback"}')
[[ "$CODE" == "200" ]]
CATALOG=$(curl -fsS "$BASE/api/client/v1/connectors" -H "Authorization: Bearer $ADMIN_JWT")
[[ "$(printf '%s' "$CATALOG" | python3 -c 'import json,sys;print(json.load(sys.stdin)["items"][0]["oauth"]["client_id"])')" == "public-client" ]]
[[ "$(printf '%s' "$CATALOG" | python3 -c 'import json,sys;d=json.load(sys.stdin)["items"][0];assert "secret" not in str(d) and "token" not in str(d).lower()')" == "" ]]
CODE=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$BASE/api/client/v1/connectors/$CONN_ID/enable" -H "Authorization: Bearer $ADMIN_JWT")
[[ "$CODE" == "200" ]]
CODE=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$BASE/api/client/v1/connectors/$CONN_ID/disable" -H "Authorization: Bearer $ADMIN_JWT")
[[ "$CODE" == "200" ]]
CODE=$(curl -sS -b /tmp/zk-store-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X DELETE "$BASE/api/admin/connectors/$CONN_ID")
[[ "$CODE" == "200" ]]
PGPASSWORD="$ROLE" psql "postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" -Atqc "SELECT count(*) FROM information_schema.tables WHERE table_name IN ('contents','content_releases','content_grants','content_deployments','content_installs');" | grep -qx 5
echo "store ok: content schema + skill archive + connectors"
