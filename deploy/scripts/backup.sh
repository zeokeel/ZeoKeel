#!/usr/bin/env bash
# ZeoKeel backup (docs/20 §3, cold backup baseline).
#
# Contents: PG `pg_dump -Fc` + `/data` (WAL DB / config / logs, master.key EXCLUDED)
#           + redacted .env + compose/Caddy config. Encrypt/offsite is an ops step.
# UNVERIFIED (must stay explicit): WAL archive / PITR, encryption-at-rest, object storage.
#
# Usage (any cwd; the script enters deploy/):
#   bash scripts/backup.sh
#   ENV_FILE=./other.env BACKUP_DIR=/var/backups bash scripts/backup.sh
#   COMPOSE_PROJECT_NAME=zeokeel-drill-1 bash scripts/backup.sh   # used by upgrade-drill
set -euo pipefail
cd "$(dirname "$0")/.."

ENV_FILE="${ENV_FILE:-.env}"
COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.yml}"
BACKUP_DIR="${BACKUP_DIR:-./backups}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="$BACKUP_DIR/$STAMP"
mkdir -p "$OUT"

DC=(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")
if [[ -n "${COMPOSE_PROJECT_NAME:-}" ]]; then
  DC+=(--project-name "$COMPOSE_PROJECT_NAME")
fi
dc() { "${DC[@]}" "$@"; }

[[ -f "$ENV_FILE" ]] || { echo "backup: missing env file $ENV_FILE" >&2; exit 1; }

# Cold backup: make sure the containers exist; /data is read through the server container.
dc up -d postgres server >/dev/null
ready=""
for _ in $(seq 1 60); do
  if dc exec -T server curl -fsS http://127.0.0.1:8787/readyz >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done
if [[ -z "$ready" ]]; then
  echo "backup: server did not become ready; refusing to claim a backup" >&2
  exit 1
fi

dc exec -T postgres pg_dump -U zeokeel -Fc zeokeel > "$OUT/zeokeel.dump"
[[ -s "$OUT/zeokeel.dump" ]] || { echo "backup: pg_dump produced an empty dump" >&2; exit 1; }

# /data: application SQLite + config + logs. master.key is deliberately excluded
# (separate offline custody, docs/20 §3).
dc exec -T server tar -czf - -C /data --exclude=master.key . > "$OUT/data.tgz"
[[ -s "$OUT/data.tgz" ]] || { echo "backup: /data archive is empty" >&2; exit 1; }

# Redacted env + deployment configs (never the MASTER_KEY value).
grep -v '^MASTER_KEY=' "$ENV_FILE" > "$OUT/env.redacted" || true
cp "$COMPOSE_FILE" "$OUT/docker-compose.yml"
cp -r caddy "$OUT/caddy"
rm -rf "$OUT/caddy/certs" "$OUT/caddy"/*.pem "$OUT/caddy"/*.key 2>/dev/null || true

cat > "$OUT/README.txt" <<'EOF'
ZeoKeel backup
- zeokeel.dump : pg_dump -Fc
- data.tgz     : /data archive (master.key NOT included)
- env.redacted : .env without MASTER_KEY
- docker-compose.yml, caddy/ : deployment config
UNVERIFIED: WAL archive / PITR, encryption-at-rest, offsite copy, restore rehearsal.
EOF

echo "backup: $OUT ($(du -sh "$OUT" | cut -f1)); master.key excluded, keep it offline"
echo "backup: PITR/WAL/encryption UNVERIFIED (docs/20 §3)"
