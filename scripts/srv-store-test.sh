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
CATALOG=$(curl -fsS "$BASE/api/client/v1/skills" -H "Authorization: Bearer $ADMIN_JWT")
[[ "$(printf '%s' "$CATALOG" | python3 -c 'import json,sys;print(len(json.load(sys.stdin)["items"]))')" == "1" ]]
CODE=$(curl -sS -b /tmp/zk-store-admin.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X DELETE "$BASE/api/admin/skills/$SKILL_ID")
[[ "$CODE" == "200" ]]
CATALOG=$(curl -fsS "$BASE/api/client/v1/skills" -H "Authorization: Bearer $ADMIN_JWT")
[[ "$(printf '%s' "$CATALOG" | python3 -c 'import json,sys;print(len(json.load(sys.stdin)["items"]))')" == "0" ]]
PGPASSWORD="$ROLE" psql "postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" -Atqc "SELECT count(*) FROM information_schema.tables WHERE table_name IN ('contents','content_releases','content_grants','content_deployments','content_installs');" | grep -qx 5
echo "store ok: content schema wired"
