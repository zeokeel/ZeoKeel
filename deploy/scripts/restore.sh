#!/usr/bin/env bash
# Restore: requires the backup directory and the offline MASTER_KEY injection.
set -euo pipefail
SRC="${1:?usage: restore.sh <backup-dir>}"
docker compose up -d postgres
docker compose exec -T postgres pg_restore -U zeokeel -d zeokeel --clean --if-exists < "$SRC/zeokeel.dump"
docker compose up -d server caddy
echo "restore complete; verify /readyz and /admin/"
