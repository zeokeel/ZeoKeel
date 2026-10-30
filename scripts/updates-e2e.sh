#!/usr/bin/env bash
# M5 stable update surface: real server + CLIENT-RELEASE + 1:1/channel checks.
set -euo pipefail
DB="zk_updates_e2e_$$"; ROLE="zk_updates_e2e_$$"; PORT="${ZEOKEEL_UPDATES_PORT:-8797}"
BASE="http://127.0.0.1:$PORT"
cleanup() { [[ -n "${SERVER_PID:-}" ]] && kill "$SERVER_PID" >/dev/null 2>&1 || true; sudo -u postgres dropdb --if-exists --force "$DB" >/dev/null 2>&1 || true; sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true; }
trap cleanup EXIT
sudo -u postgres psql -Atq -c "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';"; sudo -u postgres createdb -O "$ROLE" "$DB"
ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" ZEOKEEL_UPDATE_DIR=deploy/updates ./target/debug/zk-server >/tmp/zk-updates.log 2>&1 & SERVER_PID=$!
for _ in $(seq 1 40); do curl -fsS "$BASE/healthz" >/dev/null 2>&1 && break; sleep 0.25; done
curl -fsS "$BASE/api/updates/manifest" -H 'X-ZeoKeel-Client: 0.1.0;official;linux-x64;dev' | python3 -c 'import json,sys;d=json.load(sys.stdin);assert d["channel_id"]=="official" and d["edition"]=="enterprise"'
CODE=$(curl -sS -o /tmp/zk-updates-mismatch.json -w '%{http_code}' "$BASE/api/updates/manifest" -H 'X-ZeoKeel-Client: 0.1.0;oem-demo;linux-x64;dev'); [[ "$CODE" == "409" ]]
CODE=$(curl -sS -o /dev/null -w '%{http_code}' "$BASE/api/updates/manifest" -H 'X-ZeoKeel-Client: 0.0.9;official;linux-x64;dev'); [[ "$CODE" == "409" ]]
CODE=$(curl -sS -o /dev/null -w '%{http_code}' "$BASE/api/updates/client/../CLIENT-RELEASE.json"); [[ "$CODE" == "400" || "$CODE" == "404" ]]
curl -fsS "$BASE/portal" | grep -q 'ZeoKeel 客户端下载'
curl -fsS "$BASE/api/updates/client/ZeoKeel_0.1.0_linux_x64.AppImage" -o /tmp/zk-updates-client; cmp -s /tmp/zk-updates-client deploy/updates/clients/ZeoKeel_0.1.0_linux_x64.AppImage
# OEM portal must be brand-aware and hide the vendor string.
kill "$SERVER_PID"; wait "$SERVER_PID" 2>/dev/null || true
ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" ZEOKEEL_UPDATE_DIR=oem/fixtures/dist/oem-demo ./target/debug/zk-server >/tmp/zk-updates.log 2>&1 & SERVER_PID=$!
for _ in $(seq 1 40); do curl -fsS "$BASE/healthz" >/dev/null 2>&1 && break; sleep 0.25; done
PORTAL=$(curl -fsS "$BASE/portal")
printf '%s' "$PORTAL" | grep -q 'Acme Keel 客户端下载'
if printf '%s' "$PORTAL" | grep -q 'ZeoKeel'; then echo "oem portal leaked ZeoKeel" >&2; exit 1; fi
echo "updates e2e ok: manifest/channel-reject/version-reject/artifact/oem-portal"
