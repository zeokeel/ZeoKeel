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
PGPASSWORD="$ROLE" psql "postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" -Atqc "SELECT count(*) FROM information_schema.tables WHERE table_name IN ('contents','content_releases','content_grants','content_deployments','content_installs');" | grep -qx 5
echo "store ok: content schema wired"
