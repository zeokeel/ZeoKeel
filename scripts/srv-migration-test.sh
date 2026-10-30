#!/usr/bin/env bash
# M2 P1 migration smoke: apply deploy/migrations to a throwaway PostgreSQL database.
set -euo pipefail
DB="zk_migration_test_$$"
PSQL=(sudo -u postgres psql -v ON_ERROR_STOP=1)
createdb() { sudo -u postgres createdb "$DB"; }
cleanup() { sudo -u postgres dropdb --if-exists "$DB" >/dev/null 2>&1 || true; }
trap cleanup EXIT
createdb
for file in packages/server/http/migrations/*.sql; do
  "${PSQL[@]}" -d "$DB" -f "$file" >/dev/null
done
count=$("${PSQL[@]}" -d "$DB" -Atqc "select count(*) from information_schema.tables where table_schema='public'")
for table in users user_devices refresh_tokens admin_sessions wallet_accounts wallet_ledger wallet_holds usage_pending price_versions price_periods providers models gateway_usage audit_log system_settings job_runs schema_migrations; do
  "${PSQL[@]}" -d "$DB" -Atqc "select to_regclass('public.$table')" | grep -qx "$table" || { echo "missing $table"; exit 1; }
done
echo "migration ok: $count tables"
