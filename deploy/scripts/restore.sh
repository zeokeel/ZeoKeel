#!/usr/bin/env bash
# ZeoKeel restore (docs/20 §3/§4). Requires the offline MASTER_KEY in the env file.
#
# Steps: start PG -> stop server -> pg_restore --clean -> extract /data -> start server
#        -> wait /readyz. The MASTER_KEY itself is NOT in the backup; it must be injected
#        into the env file before the server is started (docs/20 §3).
#
# Usage (any cwd; the script enters deploy/):
#   bash scripts/restore.sh ./backups/<stamp>
#   ENV_FILE=./other.env COMPOSE_PROJECT_NAME=zeokeel-drill-1 bash scripts/restore.sh ./backups/<stamp>
set -euo pipefail
cd "$(dirname "$0")/.."

SRC="${1:?usage: restore.sh <backup-dir>}"
ENV_FILE="${ENV_FILE:-.env}"
COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.yml}"

DC=(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")
if [[ -n "${COMPOSE_PROJECT_NAME:-}" ]]; then
  DC+=(--project-name "$COMPOSE_PROJECT_NAME")
fi
dc() { "${DC[@]}" "$@"; }

[[ -f "$ENV_FILE" ]] || { echo "restore: missing env file $ENV_FILE" >&2; exit 1; }
[[ -s "$SRC/zeokeel.dump" ]] || { echo "restore: missing $SRC/zeokeel.dump" >&2; exit 1; }
grep -q '^MASTER_KEY=.\+' "$ENV_FILE" || { echo "restore: MASTER_KEY must be injected into $ENV_FILE before restore (docs/20 §3)" >&2; exit 1; }

dc up -d postgres >/dev/null
ready=""
for _ in $(seq 1 60); do
  if dc exec -T postgres pg_isready -U zeokeel -d zeokeel >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done
[[ -n "$ready" ]] || { echo "restore: postgres did not become ready" >&2; exit 1; }

dc stop server >/dev/null 2>&1 || true
dc exec -T postgres pg_restore -U zeokeel -d zeokeel --clean --if-exists < "$SRC/zeokeel.dump"

if [[ -s "$SRC/data.tgz" ]]; then
  dc run --rm --no-deps --entrypoint sh server -c 'mkdir -p /data && tar -xzf - -C /data' < "$SRC/data.tgz"
else
  echo "restore: no data.tgz in $SRC (journal/log data not restored)" >&2
fi

dc up -d server >/dev/null
ready=""
for _ in $(seq 1 120); do
  if dc exec -T server curl -fsS http://127.0.0.1:8787/readyz >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done
if [[ -z "$ready" ]]; then
  echo "restore: server did not become ready after restore" >&2
  exit 1
fi
echo "restore: complete; /readyz ok. Verify portal/admin and audit chain (docs/20 §3)."
