#!/usr/bin/env bash
# M6 fault injection: DB down, audit fail-closed, kill -9 restart, no data loss assertions.
set -euo pipefail
cd "$(dirname "$0")/.."
DB="zk_fault_$$"; ROLE="zk_fault_role_$$"; PORT="${ZEOKEEL_FAULT_PORT:-8792}"
BASE="http://127.0.0.1:$PORT"; OUT=target/evidence/m6/fault-injection.json; mkdir -p target/evidence/m6
cleanup() { [[ -n "${SERVER_PID:-}" ]] && kill "$SERVER_PID" >/dev/null 2>&1 || true; sudo -u postgres dropdb --if-exists --force "$DB" >/dev/null 2>&1 || true; sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true; }
trap cleanup EXIT
sudo -u postgres psql -Atq -c "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';"; sudo -u postgres createdb -O "$ROLE" "$DB"
start() { ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD=admin-pass ./target/debug/zk-server >/tmp/zk-fault.log 2>&1 & SERVER_PID=$!; for _ in $(seq 1 40); do curl -fsS "$BASE/readyz" >/dev/null 2>&1 && return; sleep 0.25; done; return 1; }
start
# 1. DB down: revoke connections / terminate backends, readyz must fail.
sudo -u postgres psql -Atq -c "ALTER DATABASE $DB ALLOW_CONNECTIONS false;" >/dev/null
sudo -u postgres psql -Atq -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname='$DB';" >/dev/null
sleep 0.5
CODE=$(curl -sS -o /dev/null -w '%{http_code}' "$BASE/readyz" || true); [[ "$CODE" != "200" ]]
sudo -u postgres psql -Atq -c "ALTER DATABASE $DB ALLOW_CONNECTIONS true;" >/dev/null
sleep 0.5; curl -fsS "$BASE/readyz" >/dev/null
# 2. audit fail-closed: rename audit_log; user creation must not succeed (audit writes fail).
ADMIN=$(curl -fsS -c /tmp/zk-fault.cookies -X POST "$BASE/api/admin/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass"}')
CSRF=$(printf '%s' "$ADMIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["csrf_token"])')
sudo -u postgres psql -d "$DB" -Atqc "ALTER TABLE audit_log RENAME TO audit_log_bak;" >/dev/null
CODE=$(curl -sS -b /tmp/zk-fault.cookies -H "X-CSRF-Token: $CSRF" -o /dev/null -w '%{http_code}' -X POST "$BASE/api/admin/users" -H 'content-type: application/json' -d '{"username":"faultuser","password":"fault-pass","role":"employee"}'); [[ "$CODE" != "200" ]]
sudo -u postgres psql -d "$DB" -Atqc "ALTER TABLE audit_log_bak RENAME TO audit_log;" >/dev/null
# 3. kill -9 server, restart, data intact.
ROWS_BEFORE=$(sudo -u postgres psql -d "$DB" -Atqc "SELECT count(*) FROM users;")
kill -9 "$SERVER_PID"; wait "$SERVER_PID" 2>/dev/null || true
start
ROWS_AFTER=$(sudo -u postgres psql -d "$DB" -Atqc "SELECT count(*) FROM users;")
[[ "$ROWS_BEFORE" == "$ROWS_AFTER" ]]
python3 - "$OUT" "$CODE" "$ROWS_BEFORE" <<'PY'
import json,sys,pathlib
pathlib.Path(sys.argv[1]).write_text(json.dumps({"db_down":"pass","audit_fail_closed":sys.argv[2],"kill9_rows_before":sys.argv[3],"kill9_rows_after":sys.argv[3],"status":"pass"},indent=2)+"\n")
PY
echo "fault injection ok: db-down/audit-fail-closed/kill9; rows=$ROWS_AFTER"
