#!/usr/bin/env bash
# M6 backup -> restore drill (docs/20 §3, docs/11 §5 evidence).
#
# What this actually does locally (process-level drill, real PostgreSQL):
#   1. start a throwaway `zk-server` (real migrations) on its own PG database;
#   2. write a real business row (admin-created user) so the dump has business data;
#   3. cold backup: `pg_dump -Fc` + the `/data` directory (ZEOKEEL_APP_DB_DIR) +
#      sanitized env + VERSION.json (git HEAD / workspace version / MASTER_KEY fingerprint,
#      never the key value);
#   4. restore into a FRESH database and a FRESH `/data` dir, start a second `zk-server`;
#   5. assert: readyz, table/row counts, admin login smoke, audit hash-chain link continuity;
#   6. negative check: a truncated dump must be rejected by `pg_restore`;
#   7. record durations (RPO/RTO not committed: D64) in JSON.
#
# Honest UNVERIFIED items (not fake-passed; exit 2 when the local drill passed):
#   - master.key file restore: the versioned AEAD key ring IS implemented
#     (crates/zk-kit-server/src/master_key.rs + scripts/master-key-rotate.sh); this drill
#     backs up/restores a master.key file if present, but an offline restore drill from a
#     real backup has NOT been executed (UNVERIFIED).
#   - WAL archiving / PITR: `archive_mode` is off in this single-node postgres; no WAL replay.
#   - backup encryption (age/KMS) + offsite: ops step, not local.
#
# Usage: bash scripts/backup-restore.sh
# Exit: 0 all checks pass; 2 local pass + >=1 UNVERIFIED; 1 failure.
set -uo pipefail
cd "$(dirname "$0")/.."

OUT="${ZEOKEEL_BACKUP_JSON:-target/evidence/m6/backup-restore.json}"
PORT="${ZEOKEEL_BACKUP_PORT:-8795}"
WORK="target/evidence/m6/backup"
SRC_DB="zk_backup_src_$$"
DST_DB="zk_backup_dst_$$"
ROLE="zk_backup_role_$$"
MASTER_KEY_VALUE="${ZEOKEEL_MASTER_KEY:-local-drill-master-key-not-implemented}"
MASTER_KEY_FP="$(printf '%s' "$MASTER_KEY_VALUE" | sha256sum | cut -d' ' -f1)"
mkdir -p "$(dirname "$OUT")" "$WORK"
rm -rf "$WORK"; mkdir -p "$WORK/data-src" "$WORK/data-dst"

SERVER_PID=""
cleanup() {
  if [[ -n "$SERVER_PID" ]]; then kill "$SERVER_PID" >/dev/null 2>&1 || true; fi
  sudo -u postgres dropdb --if-exists --force "$SRC_DB" >/dev/null 2>&1 || true
  sudo -u postgres dropdb --if-exists --force "$DST_DB" >/dev/null 2>&1 || true
  sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true
}
trap cleanup EXIT

CHECKS="$WORK/checks.tsv"; : > "$CHECKS"
record() { # id status detail
  printf '%s\t%s\t%s\n' "$1" "$2" "$(printf '%s' "$3" | tr '\t\n' '  ')" >> "$CHECKS"
  printf '  [%s] %s%s\n' "$2" "$1" "${3:+ — $3}"
}
finish() {
  python3 - "$CHECKS" "$OUT" "$WORK" <<'PY'
import json
import pathlib
import sys

checks_path, out_path, work = sys.argv[1:4]
checks = []
with open(checks_path, encoding="utf-8") as handle:
    for line in handle:
        line = line.rstrip("\n")
        if not line:
            continue
        parts = (line.split("\t") + ["", ""])[:3]
        checks.append({"id": parts[0], "status": parts[1], "detail": parts[2]})
if any(check["status"] == "fail" for check in checks):
    status = "fail"
elif any(check["status"] == "unverified" for check in checks):
    status = "unverified"
elif checks and all(check["status"] == "pass" for check in checks):
    status = "pass"
else:
    status = "fail"
payload = {
    "schema": 1,
    "status": status,
    "checks": checks,
    "unverified": [check["id"] for check in checks if check["status"] == "unverified"],
    "evidence_dir": work,
    "rpo_rto": "not committed (D64); durations recorded in checks",
}
pathlib.Path(out_path).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
print(f"backup-restore: {status} ({len(checks)} checks, unverified={len(payload['unverified'])}) -> {out_path}")
PY
}
fail() { record "$1" fail "$2"; finish; exit 1; }

start_server() { # url data_dir log ; sets SERVER_PID
  ZEOKEEL_DATABASE_URL="$1" \
  ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" \
  ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD=admin-pass \
  ZEOKEEL_APP_DB_DIR="$2" \
  ZEOKEEL_MASTER_KEY="$MASTER_KEY_VALUE" \
    ./target/debug/zk-server >>"$3" 2>&1 &
  SERVER_PID=$!
}

wait_ready() {
  local base="http://127.0.0.1:$PORT" attempt
  for attempt in $(seq 1 120); do
    if curl -fsS "$base/readyz" >/dev/null 2>&1; then return 0; fi
    sleep 0.25
  done
  return 1
}

sql() { PGPASSWORD="$ROLE" psql -h 127.0.0.1 -U "$ROLE" -d "$1" -Atqc "$2"; }

# ---------------------------------------------------------------- source instance
echo "backup-restore: source instance ($SRC_DB)"
sudo -u postgres psql -Atq -c "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';" || fail source-db "CREATE ROLE failed"
sudo -u postgres createdb -O "$ROLE" "$SRC_DB" || fail source-db "createdb failed"
BUILD_MS=0
if [[ ! -x target/debug/zk-server ]]; then
  START=$(date +%s%N); cargo build -q --offline -p zk-server || fail source-server "cargo build zk-server failed"; BUILD_MS=$(( ($(date +%s%N) - START) / 1000000 ))
fi
SRC_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$SRC_DB"
START=$(date +%s%N)
start_server "$SRC_URL" "$WORK/data-src" "$WORK/source-server.log"
if ! wait_ready; then fail source-ready "$(tail -n 8 "$WORK/source-server.log" | tr '\n' ' ')"; fi
READY_MS=$(( ($(date +%s%N) - START) / 1000000 ))
record source-instance pass "server ready in ${READY_MS}ms (build ${BUILD_MS}ms); migrations applied"

# minimal real business write so the backup has more than bootstrap rows.
ADMIN_JSON="$(curl -fsS -c "$WORK/admin.cookies" -X POST "http://127.0.0.1:$PORT/api/admin/auth/login" \
  -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass"}')" || fail admin-login "admin login failed"
CSRF="$(printf '%s' "$ADMIN_JSON" | python3 -c 'import json,sys;print(json.load(sys.stdin)["csrf_token"])')" || fail admin-login "csrf_token missing"
if ! curl -fsS -b "$WORK/admin.cookies" -H "X-CSRF-Token: $CSRF" -X POST "http://127.0.0.1:$PORT/api/admin/users" \
  -H 'content-type: application/json' \
  -d '{"username":"backup-drill-user","password":"drill-pass-123","role":"employee"}' > "$WORK/create-user.json"; then
  fail business-write "admin user create failed: $(cat "$WORK/create-user.json" 2>/dev/null)"
fi
record business-write pass "created user backup-drill-user with audit row"

SRC_USERS="$(sql "$SRC_DB" 'SELECT count(*) FROM users;')"
SRC_AUDIT="$(sql "$SRC_DB" 'SELECT count(*) FROM audit_log;')"
SRC_TABLES="$(sql "$SRC_DB" "SELECT count(*) FROM information_schema.tables WHERE table_schema='public';")"
SRC_LEDGER="$(sql "$SRC_DB" 'SELECT count(*) FROM wallet_ledger;' 2>/dev/null || echo 0)"
record source-counters pass "users=$SRC_USERS audit=$SRC_AUDIT ledger=$SRC_LEDGER tables=$SRC_TABLES"

# ---------------------------------------------------------------- backup
echo "backup-restore: cold backup"
START=$(date +%s%N)
if ! PGPASSWORD="$ROLE" pg_dump -Fc -h 127.0.0.1 -U "$ROLE" "$SRC_DB" > "$WORK/pg.dump"; then
  fail pg-dump "pg_dump failed"
fi
[[ -s "$WORK/pg.dump" ]] || fail pg-dump "pg_dump produced an empty dump"
tar -czf "$WORK/data.tgz" -C "$WORK/data-src" . || fail data-archive "tar of /data failed"
cat > "$WORK/env.sanitized" <<'ENV'
# sanitized drill env: secrets are NOT stored in the backup package (docs/20 §3)
ZEOKEEL_DATABASE_URL=***
ZEOKEEL_MASTER_KEY=***
ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD=***
ZEOKEEL_APP_DB_DIR=***
ENV
# master.key: if a real file exists it is backed up (0600); otherwise the check stays UNVERIFIED.
MASTER_KEY_STATE="absent"
if [[ -f "$WORK/data-src/master.key" ]]; then
  cp -p "$WORK/data-src/master.key" "$WORK/master.key"; chmod 600 "$WORK/master.key"; MASTER_KEY_STATE="present"
fi
VERSION="$(python3 -c 'import tomllib;print(tomllib.load(open("Cargo.toml","rb"))["workspace"]["package"]["version"])' 2>/dev/null || echo unknown)"
cat > "$WORK/VERSION.json" <<VER
{"version": "$VERSION", "head": "$(git rev-parse --short HEAD 2>/dev/null || echo unknown)", "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)", "master_key_fingerprint_sha256": "$MASTER_KEY_FP", "master_key_file": "$MASTER_KEY_STATE"}
VER
BACKUP_MS=$(( ($(date +%s%N) - START) / 1000000 ))
record backup-package pass "pg.dump=$(stat -c%s "$WORK/pg.dump")B data.tgz=$(stat -c%s "$WORK/data.tgz")B VERSION.json + sanitized env; backup ${BACKUP_MS}ms"
if [[ "$MASTER_KEY_STATE" == "present" ]]; then
  record master-key-backup pass "master.key file backed up separately (0600) and fingerprinted"
else
  record master-key-backup unverified "no master.key file in /data; server does not implement master.key/encryption yet (upstream key is plaintext api_key_enc, docs/07 '未决'); only MASTER_KEY fingerprint recorded"
fi
ARCHIVE_MODE="$(sudo -u postgres psql -Atqc 'SHOW archive_mode;' 2>/dev/null || echo unknown)"
if [[ "$ARCHIVE_MODE" == "on" ]]; then
  record wal-pitr pass "archive_mode=on (WAL archiving configured)"
else
  record wal-pitr unverified "archive_mode=$ARCHIVE_MODE: no WAL archive/PITR; local drill cannot replay WAL"
fi
record backup-encryption unverified "backup encryption with age/KMS + offsite copy is an ops step (docs/20 §3), not exercised locally"

kill "$SERVER_PID" >/dev/null 2>&1 || true
wait "$SERVER_PID" 2>/dev/null || true
SERVER_PID=""

# ---------------------------------------------------------------- restore into fresh instance
echo "backup-restore: restore into fresh database + fresh /data"
sudo -u postgres psql -Atq -c "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';" 2>/dev/null || true
sudo -u postgres createdb -O "$ROLE" "$DST_DB" || fail restore-db "createdb (fresh) failed"
START=$(date +%s%N)
if ! PGPASSWORD="$ROLE" pg_restore -h 127.0.0.1 -U "$ROLE" -d "$DST_DB" "$WORK/pg.dump" 2> "$WORK/pg-restore.err"; then
  fail pg-restore "pg_restore failed: $(tr '\n' ' ' < "$WORK/pg-restore.err")"
fi
if [[ -s "$WORK/master.key" ]]; then cp -p "$WORK/master.key" "$WORK/data-dst/master.key"; fi
tar -xzf "$WORK/data.tgz" -C "$WORK/data-dst" || fail data-restore "tar restore failed"
DST_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DST_DB"
start_server "$DST_URL" "$WORK/data-dst" "$WORK/restore-server.log"
if ! wait_ready; then fail restore-ready "restored server not ready: $(tail -n 8 "$WORK/restore-server.log" | tr '\n' ' ')"; fi
RESTORE_MS=$(( ($(date +%s%N) - START) / 1000000 ))
record restore-ready pass "fresh DB + fresh /data; server ready in ${RESTORE_MS}ms (migrations re-run idempotently)"

LOGIN_CODE="$(curl -sS -o "$WORK/restore-login.json" -w '%{http_code}' -X POST "http://127.0.0.1:$PORT/api/client/v1/auth/login" \
  -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass","device_id":"restore-drill"}')"
if [[ "$LOGIN_CODE" != "200" ]]; then
  fail restore-login "admin login after restore returned HTTP $LOGIN_CODE: $(cat "$WORK/restore-login.json")"
fi
record restore-login pass "admin login smoke after restore HTTP 200"

DST_USERS="$(sql "$DST_DB" 'SELECT count(*) FROM users;')"
DST_AUDIT="$(sql "$DST_DB" 'SELECT count(*) FROM audit_log;')"
DST_TABLES="$(sql "$DST_DB" "SELECT count(*) FROM information_schema.tables WHERE table_schema='public';")"
DST_LEDGER="$(sql "$DST_DB" 'SELECT count(*) FROM wallet_ledger;' 2>/dev/null || echo 0)"
if [[ "$DST_USERS" == "$SRC_USERS" && "$DST_AUDIT" == "$SRC_AUDIT" && "$DST_TABLES" == "$SRC_TABLES" && "$DST_LEDGER" == "$SRC_LEDGER" ]]; then
  record restore-rowcounts pass "users/audit/ledger/tables match source ($DST_USERS/$DST_AUDIT/$DST_LEDGER/$DST_TABLES)"
else
  fail restore-rowcounts "row counts differ: users $SRC_USERS->$DST_USERS audit $SRC_AUDIT->$DST_AUDIT ledger $SRC_LEDGER->$DST_LEDGER tables $SRC_TABLES->$DST_TABLES"
fi

CHAIN_BAD="$(sql "$DST_DB" "WITH chain AS (SELECT id, prev_hash, hash, lag(hash) OVER (ORDER BY id) AS prev FROM audit_log) SELECT count(*) FROM chain WHERE prev IS NOT NULL AND prev_hash <> prev;")"
if [[ "$CHAIN_BAD" == "0" ]]; then
  record audit-chain pass "audit_log prev_hash links intact after restore (0 broken links)"
else
  fail audit-chain "$CHAIN_BAD broken prev_hash link(s) after restore"
fi

# negative: unreadable/truncated dump must be rejected (docs/20 §3: backup unreadable => abort)
head -c 2048 "$WORK/pg.dump" > "$WORK/pg-truncated.dump"
if PGPASSWORD="$ROLE" pg_restore -h 127.0.0.1 -U "$ROLE" -d "$DST_DB" "$WORK/pg-truncated.dump" >/dev/null 2>&1; then
  record restore-rejects-corrupt fail "pg_restore accepted a 2KiB truncated dump (expected rejection)"
else
  record restore-rejects-corrupt pass "pg_restore rejects truncated/unreadable dump (exit non-zero)"
fi

finish
STATUS="$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["status"])' "$OUT")"
case "$STATUS" in
  pass) exit 0 ;;
  unverified) exit 2 ;;
  *) exit 1 ;;
esac
