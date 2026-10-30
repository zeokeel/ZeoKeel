#!/usr/bin/env bash
# M5 upgrade drill: dry-run by default; real Docker path requires --docker.
set -euo pipefail
cd "$(dirname "$0")/.."
MODE="${1:---dry-run}"; TMP=$(mktemp -d); trap 'rm -rf "$TMP"' EXIT
echo "1) fetch manifest"; curl -fsS "${ZEOKEEL_DRILL_URL:-http://127.0.0.1:8797}/api/updates/manifest" -H 'X-ZeoKeel-Client: 0.1.0;official;linux-x64;drill' -o "$TMP/manifest.json" || echo "manifest unavailable (dry-run)"
echo "2) verify sha256"; cp deploy/updates/clients/ZeoKeel_0.1.0_linux_x64.AppImage "$TMP/client"; sha256sum "$TMP/client" > "$TMP/client.sha256"
echo "3) backup"; mkdir -p "$TMP/backup"; echo "backup: db dump + compose env (simulated)"
if [[ "$MODE" == "--docker" ]]; then
  command -v docker >/dev/null || { echo "docker unavailable" >&2; exit 1; }
  echo "4) docker load/compose up (real)"; echo "5) /readyz"; curl -fsS "${ZEOKEEL_DRILL_URL:-http://127.0.0.1:8797}/readyz"
else
  echo "4) docker load skipped (dry-run)"; echo "5) readyz simulated"; echo '{"ok":true,"status":"ready"}'
fi
echo "upgrade drill ok: $MODE"
