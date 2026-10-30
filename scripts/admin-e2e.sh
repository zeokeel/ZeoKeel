#!/usr/bin/env bash
# Provision a throwaway PostgreSQL + real server, build admin SPA, run M2 admin Playwright.
set -euo pipefail
DB="zk_admin_e2e_$$"
ROLE="zk_admin_e2e_$$"
PORT="${ZEOKEEL_ADMIN_TEST_PORT:-8794}"
BASE="http://127.0.0.1:$PORT"
cleanup() {
  [[ -n "${SERVER_PID:-}" ]] && kill "$SERVER_PID" >/dev/null 2>&1 || true
  sudo -u postgres dropdb --if-exists --force "$DB" >/dev/null 2>&1 || true
  sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true
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
mkdir -p target/evidence/m2/admin target/evidence/m3
export ZEOKEEL_SKILL_ZIP=/tmp/zk-admin-skill.zip
ZEOKEEL_ADMIN_URL="$BASE/admin" ZEOKEEL_ADMIN_PASSWORD="admin-pass" \
  pnpm exec playwright test e2e/m2-admin.spec.ts e2e/m3-admin-skills.spec.ts e2e/m3-admin-connectors.spec.ts --reporter=line
echo "admin e2e ok"
