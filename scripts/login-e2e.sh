#!/usr/bin/env bash
# Enterprise login e2e: enterprise compose + real auth server + serve UI.
set -euo pipefail
DB="zk_login_e2e_$$"; ROLE="zk_login_role_$$"
SERVER_PORT="${ZEOKEEL_LOGIN_SERVER_PORT:-8795}"; UI_PORT="${ZEOKEEL_LOGIN_UI_PORT:-8789}"
cleanup() {
  [[ -n "${UI_PID:-}" ]] && kill "$UI_PID" >/dev/null 2>&1 || true
  [[ -n "${SERVER_PID:-}" ]] && kill "$SERVER_PID" >/dev/null 2>&1 || true
  sudo -u postgres dropdb --if-exists --force "$DB" >/dev/null 2>&1 || true
  sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true
  cargo run -q -p zk -- compose --edition personal --profile default --channel official >/dev/null 2>&1 || true
}
trap cleanup EXIT
cargo run -q -p zk -- compose --edition enterprise --profile default --channel official >/dev/null
cargo build -q -p zk-server -p zk-desktop
pnpm --dir apps/desktop/web build >/dev/null
sudo -u postgres psql -Atq -c "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';"
sudo -u postgres createdb -O "$ROLE" "$DB"
ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" \
ZEOKEEL_SERVER_ADDR="127.0.0.1:$SERVER_PORT" \
ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD="admin-pass" \
ZEOKEEL_DEV_CORS_ORIGIN="http://127.0.0.1:$UI_PORT" \
./target/debug/zk-server >/tmp/zk-login-server.log 2>&1 & SERVER_PID=$!
rm -rf target/evidence/m2/login-home; mkdir -p target/evidence/m2/login-home
ZEOKEEL_HOME=target/evidence/m2/login-home ZEOKEEL_SERVE_ADDR="127.0.0.1:$UI_PORT" \
./target/debug/zk-desktop --serve >/tmp/zk-login-desktop.log 2>&1 & UI_PID=$!
for _ in $(seq 1 40); do curl -fsS "http://127.0.0.1:$UI_PORT/healthz" >/dev/null 2>&1 && break; sleep 0.5; done
ZEOKEEL_LOGIN_URL="http://127.0.0.1:$UI_PORT" ZEOKEEL_LOGIN_SERVER="http://127.0.0.1:$SERVER_PORT" \
  pnpm exec playwright test e2e/m2-login.spec.ts --reporter=line
echo "login e2e ok"
