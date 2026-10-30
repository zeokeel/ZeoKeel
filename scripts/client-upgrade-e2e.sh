#!/usr/bin/env bash
# Real forced-upgrade e2e: enterprise server with ZEOKEEL_MIN_CLIENT_VERSION=9.9.9
# + enterprise desktop serve. Login is exempt from the business 426 gate; the next
# business call returns 426 and the client must show the non-dismissable UpgradeGate.
set -euo pipefail
cd "$(dirname "$0")/.."

DB="zk_upgrade_e2e_$$"
ROLE="zk_upgrade_role_$$"
SERVER_PORT="${ZEOKEEL_UPGRADE_SERVER_PORT:-8798}"
UI_PORT="${ZEOKEEL_UPGRADE_UI_PORT:-8799}"
EVIDENCE="target/evidence/client-upgrade"

stop_pid() {
  local pid="$1"
  [[ -n "$pid" ]] || return 0
  kill "$pid" 2>/dev/null || true
  for _ in $(seq 1 40); do kill -0 "$pid" 2>/dev/null || break; sleep 0.25; done
  if kill -0 "$pid" 2>/dev/null; then kill -KILL "$pid" 2>/dev/null || true; fi
  wait "$pid" 2>/dev/null || true
}
cleanup() {
  stop_pid "${UI_PID:-}"; stop_pid "${SERVER_PID:-}"
  sudo -u postgres dropdb --if-exists --force "$DB" >/dev/null 2>&1 || true
  sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true
  # docs/24 §1.2: never leave the enterprise composition in the committed baseline.
  bash scripts/restore-baseline.sh >/dev/null 2>&1 || true
}
trap cleanup EXIT

mkdir -p "$EVIDENCE"
sudo -u postgres psql -v ON_ERROR_STOP=1 -Atqc "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';"
sudo -u postgres createdb -O "$ROLE" "$DB"

cargo run -q -p zk -- compose --edition enterprise --profile default --channel official >/dev/null
cargo build -q -p zk-server -p zk-desktop
pnpm --dir apps/desktop/web build >/dev/null
pnpm --dir apps/admin/web build >/dev/null

rm -rf target/evidence/round6/upgrade-home
mkdir -p target/evidence/round6/upgrade-home

ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" \
ZEOKEEL_SERVER_ADDR="127.0.0.1:$SERVER_PORT" \
ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD="admin-pass" \
ZEOKEEL_MIN_CLIENT_VERSION="9.9.9" \
ZEOKEEL_ADMIN_DIST="apps/admin/web/dist" \
ZEOKEEL_DEV_CORS_ORIGIN="http://127.0.0.1:$UI_PORT" \
./target/debug/zk-server >/tmp/zk-upgrade-server.log 2>&1 & SERVER_PID=$!
for _ in $(seq 1 80); do curl -fsS "http://127.0.0.1:$SERVER_PORT/healthz" >/dev/null 2>&1 && break; sleep 0.25; done
curl -fsS "http://127.0.0.1:$SERVER_PORT/readyz" >/dev/null

ZEOKEEL_HOME="target/evidence/round6/upgrade-home" ZEOKEEL_DEV_KEYCHAIN_FALLBACK=1 ZEOKEEL_SERVE_ADDR="127.0.0.1:$UI_PORT" \
./target/debug/zk-desktop --serve >/tmp/zk-upgrade-desktop.log 2>&1 & UI_PID=$!
for _ in $(seq 1 80); do curl -fsS "http://127.0.0.1:$UI_PORT/healthz" >/dev/null 2>&1 && break; sleep 0.25; done

set +e
ZEOKEEL_E2E_NO_WEBSERVER=1 \
ZEOKEEL_UPGRADE_URL="http://127.0.0.1:$UI_PORT" \
ZEOKEEL_UPGRADE_SERVER="http://127.0.0.1:$SERVER_PORT" \
  pnpm exec playwright test e2e/client-upgrade.spec.ts --reporter=line
STATUS=$?
set -e
echo "client-upgrade e2e exit: $STATUS"
exit "$STATUS"
