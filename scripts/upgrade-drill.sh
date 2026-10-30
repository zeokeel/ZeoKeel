#!/usr/bin/env bash
# M5 upgrade drill: dry-run by default; Docker path is explicitly UNVERIFIED if unavailable.
set -euo pipefail
cd "$(dirname "$0")/.."
MODE="${1:---dry-run}";
if [[ "$MODE" == "--full" ]]; then
  bash scripts/backup-restore.sh
  MODE=--dry-run
fi
TMP=$(mktemp -d); trap 'rm -rf "$TMP"' EXIT
URL="${ZEOKEEL_DRILL_URL:-}"
if [[ -n "$URL" ]] && curl -fsS "$URL/api/updates/manifest" -H 'X-ZeoKeel-Client: 0.1.0;official;linux-x64;drill' -o "$TMP/manifest.json"; then
  echo "1) manifest downloaded"
else
  cp deploy/updates/CLIENT-RELEASE.json "$TMP/manifest.json"
  echo "1) manifest local fallback (server unavailable; dry-run)"
fi
cp deploy/updates/clients/ZeoKeel_0.1.0_linux_x64.AppImage "$TMP/client"
EXPECTED=$(python3 - "$TMP/manifest.json" "$TMP/client" <<'PY'
import json,sys,hashlib
m=json.load(open(sys.argv[1])); data=hashlib.sha256(open(sys.argv[2],'rb').read()).hexdigest()
assets=m.get('assets') or []
if not assets: print("UNVERIFIED_NO_ASSETS"); raise SystemExit(0)
print(assets[0].get('sha256',''))
PY
)
ACTUAL=$(sha256sum "$TMP/client" | cut -d' ' -f1)
if [[ "$EXPECTED" == "UNVERIFIED_NO_ASSETS" ]]; then echo "2) sha UNVERIFIED (manifest assets empty)"; elif [[ "$EXPECTED" != "$ACTUAL" ]]; then echo "sha mismatch" >&2; exit 1; else echo "2) sha256 verified"; fi
mkdir -p "$TMP/backup"; echo "3) backup dir prepared (simulated)"
if [[ "$MODE" == "--docker" ]]; then
  command -v docker >/dev/null || { echo "UNVERIFIED: docker unavailable" >&2; exit 2; }
  echo "4) docker load / compose up"; echo "5) readyz"; curl -fsS "${URL:-http://127.0.0.1:8797}/readyz"
else
  echo "4) docker load skipped (dry-run)"; echo "5) readyz simulated"; echo '{"ok":true,"status":"ready"}'
fi
echo "upgrade drill UNVERIFIED (no real docker/upgrade/rollback/restore): $MODE"
exit 2
