#!/usr/bin/env bash
# Provision a throwaway PostgreSQL + real server, build admin SPA, run M2 admin Playwright.
set -euo pipefail
CLIENT_HDR="X-ZeoKeel-Client: 0.1.0;official;linux;e2e"
DB="zk_admin_e2e_$$"
ROLE="zk_admin_e2e_$$"
PORT="${ZEOKEEL_ADMIN_TEST_PORT:-8794}"
BASE="http://127.0.0.1:$PORT"
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
  stop_server
  sudo -u postgres dropdb --if-exists --force "$DB" >/dev/null 2>&1 || true
  sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true
  bash scripts/restore-baseline.sh >/dev/null 2>&1 || true
}
trap cleanup EXIT
sudo -u postgres psql -v ON_ERROR_STOP=1 -Atqc "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';"
sudo -u postgres createdb -O "$ROLE" "$DB"
cargo run -q -p zk -- compose --edition enterprise --profile default --channel official >/dev/null
cargo build -q -p zk-server
pnpm --dir apps/admin/web build >/dev/null
ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" \
ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" \
ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD="admin-pass" \
ZEOKEEL_ADMIN_DIST="apps/admin/web/dist" \
./target/debug/zk-server >/tmp/zk-admin-e2e.log 2>&1 &
SERVER_PID=$!
for _ in $(seq 1 40); do curl -fsS "$BASE/healthz" >/dev/null 2>&1 && break; sleep 0.25; done
python3 - <<'PYZIP'
import zipfile
with zipfile.ZipFile('/tmp/zk-admin-skill.zip', 'w', zipfile.ZIP_DEFLATED) as z:
    z.writestr('SKILL.md', '# admin e2e skill\n')
PYZIP
# Seed one pending app for M4 admin page (dummy wasm; no execution in this test).
ADMIN_JWT=$(curl -fsS -H "$CLIENT_HDR" -X POST "$BASE/api/client/v1/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass","device_id":"admin-e2e"}' | python3 -c 'import json,sys;print(json.load(sys.stdin)["access_token"])')
printf '\0asm\1\0\0\0' > /tmp/zk-admin-app.wasm
printf '{"name":"admin-app","version":"1.0.0","description":"admin","icon":"","db_schema_version":0,"db_migrations":[],"ui":[]}' > /tmp/zk-admin-app.json
curl -fsS -H "$CLIENT_HDR" -X POST "$BASE/api/client/v1/apps" -H "Authorization: Bearer $ADMIN_JWT" -F "manifest=</tmp/zk-admin-app.json" -F "wasm=@/tmp/zk-admin-app.wasm" >/dev/null
mkdir -p target/evidence/m2/admin target/evidence/m3
export ZEOKEEL_SKILL_ZIP=/tmp/zk-admin-skill.zip
ZEOKEEL_ADMIN_URL="$BASE/admin" ZEOKEEL_PORTAL_URL="$BASE/portal" ZEOKEEL_ADMIN_PASSWORD="admin-pass" \
  pnpm exec playwright test e2e/m2-admin.spec.ts e2e/m3-admin-skills.spec.ts e2e/m3-admin-connectors.spec.ts e2e/m4-admin-apps.spec.ts e2e/m5-portal.spec.ts e2e/m6-a11y.spec.ts --reporter=line
echo "admin e2e ok"
