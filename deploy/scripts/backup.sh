#!/usr/bin/env bash
# Backup: PG dump + portal/config; master.key and .env MASTER_KEY are excluded.
set -euo pipefail
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="${BACKUP_DIR:-./backups}/$STAMP"
mkdir -p "$OUT"
docker compose exec -T postgres pg_dump -U zeokeel -Fc zeokeel > "$OUT/zeokeel.dump"
if [[ -f .env ]]; then grep -v '^MASTER_KEY=' .env > "$OUT/env.redacted"; fi
cp deploy/docker-compose.yml deploy/caddy/Caddyfile "$OUT/" 2>/dev/null || true
echo "backup: $OUT (master key excluded; keep it offline)"
