#!/usr/bin/env bash
# M6 backup/restore: PG dump + /data + sanitized env; restore to temp DB and verify.
set -euo pipefail
cd "$(dirname "$0")/.."
SRC_DB="zk_backup_src_$$"; DST_DB="zk_backup_dst_$$"; ROLE="zk_backup_role_$$"; PORT="${ZEOKEEL_BACKUP_PORT:-8793}"
WORK="target/evidence/m6/backup"; rm -rf "$WORK"; mkdir -p "$WORK/data"
cleanup() { [[ -n "${SERVER_PID:-}" ]] && kill "$SERVER_PID" >/dev/null 2>&1 || true; sudo -u postgres dropdb --if-exists --force "$SRC_DB" >/dev/null 2>&1 || true; sudo -u postgres dropdb --if-exists --force "$DST_DB" >/dev/null 2>&1 || true; sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true; }
trap cleanup EXIT
sudo -u postgres psql -Atq -c "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';"; sudo -u postgres createdb -O "$ROLE" "$SRC_DB"
ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$SRC_DB" ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD=admin-pass ZEOKEEL_APP_DB_DIR="$WORK/data/app-db" ./target/debug/zk-server >/tmp/zk-backup.log 2>&1 & SERVER_PID=$!
for _ in $(seq 1 40); do curl -fsS "http://127.0.0.1:$PORT/readyz" >/dev/null 2>&1 && break; sleep 0.25; done
curl -fsS "http://127.0.0.1:$PORT/readyz" >/dev/null
kill "$SERVER_PID"; wait "$SERVER_PID" 2>/dev/null || true; unset SERVER_PID
PGPASSWORD="$ROLE" pg_dump -Fc -h 127.0.0.1 -U "$ROLE" "$SRC_DB" > "$WORK/pg.dump"
tar -czf "$WORK/data.tgz" -C "$WORK" data
printf 'MASTER_KEY=***\nDATABASE_URL=***\n' > "$WORK/env.sanitized"
sudo -u postgres createdb -O "$ROLE" "$DST_DB"
PGPASSWORD="$ROLE" pg_restore -h 127.0.0.1 -U "$ROLE" -d "$DST_DB" "$WORK/pg.dump"
TABLES=$(PGPASSWORD="$ROLE" psql -h 127.0.0.1 -U "$ROLE" -d "$DST_DB" -Atqc "SELECT count(*) FROM information_schema.tables WHERE table_schema='public';")
sudo -u postgres psql -Atq -c "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';" 2>/dev/null || true
ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DST_DB" ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD=admin-pass ./target/debug/zk-server >/tmp/zk-restore.log 2>&1 & SERVER_PID=$!
for _ in $(seq 1 40); do curl -fsS "http://127.0.0.1:$PORT/readyz" >/dev/null 2>&1 && break; sleep 0.25; done
curl -fsS "http://127.0.0.1:$PORT/readyz" >/dev/null
curl -fsS -X POST "http://127.0.0.1:$PORT/api/client/v1/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass","device_id":"restore"}' | python3 -c 'import json,sys;d=json.load(sys.stdin);assert d["ok"]'
[[ "$TABLES" -ge 10 ]]
echo "backup restore PARTIAL: $TABLES tables; /data/env bundled; PITR/WAL/master.key restore UNVERIFIED"
exit 2
