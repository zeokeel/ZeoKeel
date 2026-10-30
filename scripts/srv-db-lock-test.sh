#!/usr/bin/env bash
# `zk db lock` append-only regression: edited/deleted history must be refused and
# must never be "laundered" by rewriting zk.lock (docs/24 §5).
set -euo pipefail
cargo build -q -p zk
ZK="$PWD/target/debug/zk"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/packages/demo/migrations"
: >"$TMP/zk.toml"
printf 'BEGIN; CREATE TABLE demo_a (id int); COMMIT;\n' >"$TMP/packages/demo/migrations/0001_demo.sql"

(cd "$TMP" && "$ZK" db lock) >/tmp/zk-db-lock-initial.log
grep -q '"version": "0001_demo"' "$TMP/zk.lock"
(cd "$TMP" && "$ZK" db lock --check) >/dev/null
cp "$TMP/zk.lock" /tmp/zk-db-lock-before.json

# 1. Editing an already-recorded migration must be refused (no laundering).
printf 'BEGIN; CREATE TABLE demo_a (id bigint); COMMIT;\n' >"$TMP/packages/demo/migrations/0001_demo.sql"
if (cd "$TMP" && "$ZK" db lock) >/tmp/zk-db-lock-edited.log 2>&1; then
  echo "FAIL: edited historical migration was accepted" >&2
  exit 1
fi
grep -q "migration history changed" /tmp/zk-db-lock-edited.log
cmp -s "$TMP/zk.lock" /tmp/zk-db-lock-before.json
# Restore and confirm --check is read-only/green again.
printf 'BEGIN; CREATE TABLE demo_a (id int); COMMIT;\n' >"$TMP/packages/demo/migrations/0001_demo.sql"
(cd "$TMP" && "$ZK" db lock --check) >/dev/null

# 2. Deleting a recorded migration must be refused.
mv "$TMP/packages/demo/migrations/0001_demo.sql" /tmp/zk-db-lock-0001.sql
if (cd "$TMP" && "$ZK" db lock) >/tmp/zk-db-lock-deleted.log 2>&1; then
  echo "FAIL: deleted historical migration was accepted" >&2
  exit 1
fi
grep -q "migration history deleted or renamed" /tmp/zk-db-lock-deleted.log
mv /tmp/zk-db-lock-0001.sql "$TMP/packages/demo/migrations/0001_demo.sql"

# 3. New migrations are appended normally.
printf 'BEGIN; CREATE TABLE demo_b (id int); COMMIT;\n' >"$TMP/packages/demo/migrations/0002_demo.sql"
(cd "$TMP" && "$ZK" db lock) >/tmp/zk-db-lock-appended.log
grep -q '"version": "0002_demo"' "$TMP/zk.lock"
(cd "$TMP" && "$ZK" db lock --check) >/dev/null

# 4. Inserting a "new" migration before the recorded maximum is refused.
printf 'BEGIN; CREATE TABLE demo_early (id int); COMMIT;\n' >"$TMP/packages/demo/migrations/0000_early.sql"
if (cd "$TMP" && "$ZK" db lock) >/tmp/zk-db-lock-early.log 2>&1; then
  echo "FAIL: out-of-order migration was accepted" >&2
  exit 1
fi
grep -q "append-only" /tmp/zk-db-lock-early.log
echo "db lock ok: edited/deleted history refused, append-only enforced, --check read-only"
