#!/usr/bin/env bash
# M5 stable update surface: real server + CLIENT-RELEASE + 1:1/channel checks.
set -euo pipefail
DB="zk_updates_e2e_$$"; ROLE="zk_updates_e2e_$$"; PORT="${ZEOKEEL_UPDATES_PORT:-8797}"
BASE="http://127.0.0.1:$PORT"
# Stop a PID with SIGTERM, then force-kill if graceful shutdown does not finish.
# zk-server shutdown can exceed its documented total deadline when the DB/role it is
# using disappears mid-shutdown (see target/evidence/audit/round4-delivery.md), so
# always wait (bounded) BEFORE dropping the DB; never orphan the server.
stop_pid() {
  local pid="$1"
  [[ -n "$pid" ]] || return 0
  kill "$pid" 2>/dev/null || true
  for _ in $(seq 1 40); do kill -0 "$pid" 2>/dev/null || break; sleep 0.25; done
  if kill -0 "$pid" 2>/dev/null; then kill -KILL "$pid" 2>/dev/null || true; fi
  wait "$pid" 2>/dev/null || true
}
stop_server() { stop_pid "${SERVER_PID:-}"; SERVER_PID=""; }
cleanup() {
  stop_server
  sudo -u postgres dropdb --if-exists --force "$DB" >/dev/null 2>&1 || true
  sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true
  # A18: if this script composed enterprise, restore the committed personal baseline.
  if [[ "${COMPOSITION:-}" == "composed-enterprise" ]]; then
    bash scripts/restore-baseline.sh >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT
sudo -u postgres psql -Atq -c "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';"; sudo -u postgres createdb -O "$ROLE" "$DB"
# A18 target trimming: the committed personal baseline generates a stub zk-server
# (exit 2, zero srv deps). Compose enterprise unless the caller already did (CI builds
# the enterprise binary before invoking this script).
COMPOSITION="$(bash scripts/ensure-enterprise-composition.sh)" \
  || { echo "updates-e2e: cannot compose the enterprise server (A18)" >&2; exit 2; }
echo "updates-e2e: composition=$COMPOSITION"
if [[ "$COMPOSITION" == "composed-enterprise" || ! -x target/debug/zk-server ]]; then
  cargo build -q -p zk-server
fi
echo "updates-e2e: starting enterprise server"
ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" ZEOKEEL_UPDATE_DIR=deploy/updates ./target/debug/zk-server >/tmp/zk-updates.log 2>&1 & SERVER_PID=$!
ready=0
for _ in $(seq 1 80); do if curl -fsS "$BASE/healthz" >/dev/null 2>&1; then ready=1; break; fi; sleep 0.25; done
if [[ "$ready" != "1" ]]; then echo "updates-e2e: server readiness timeout; log:" >&2; cat /tmp/zk-updates.log >&2; exit 1; fi
curl -fsS "$BASE/api/updates/manifest" -H 'X-ZeoKeel-Client: 0.1.0;official;linux-x64;dev' | python3 -c 'import json,sys;d=json.load(sys.stdin);assert d["channel_id"]=="official" and d["edition"]=="enterprise"'
CODE=$(curl -sS -o /tmp/zk-updates-mismatch.json -w '%{http_code}' "$BASE/api/updates/manifest" -H 'X-ZeoKeel-Client: 0.1.0;oem-demo;linux-x64;dev'); [[ "$CODE" == "409" ]]
# Below min_supported -> 426 CLIENT_TOO_OLD (docs/04 §2.7 / docs/09 §7.3).
CODE=$(curl -sS -o /tmp/zk-updates-too-old.json -w '%{http_code}' "$BASE/api/updates/manifest" -H 'X-ZeoKeel-Client: 0.0.9;official;linux-x64;dev'); [[ "$CODE" == "426" ]]
python3 -c 'import json;d=json.load(open("/tmp/zk-updates-too-old.json"));assert d["error"]["code"]=="CLIENT_TOO_OLD" and d["error"]["request_id"], d' 
CODE=$(curl -sS -o /dev/null -w '%{http_code}' "$BASE/api/updates/client/../CLIENT-RELEASE.json"); [[ "$CODE" == "400" || "$CODE" == "404" ]]
curl -fsS "$BASE/portal" | grep -q 'ZeoKeel 客户端下载'
curl -fsS "$BASE/api/updates/client/ZeoKeel_0.1.0_linux_x64.AppImage" -o /tmp/zk-updates-client; cmp -s /tmp/zk-updates-client deploy/updates/clients/ZeoKeel_0.1.0_linux_x64.AppImage
# OEM portal must be brand-aware and hide the vendor string.
stop_server
ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" ZEOKEEL_UPDATE_DIR=oem/fixtures/dist/oem-demo ./target/debug/zk-server >/tmp/zk-updates.log 2>&1 & SERVER_PID=$!
for _ in $(seq 1 40); do curl -fsS "$BASE/healthz" >/dev/null 2>&1 && break; sleep 0.25; done
PORTAL=$(curl -fsS "$BASE/portal")
printf '%s' "$PORTAL" | grep -q 'Acme Keel 客户端下载'
if printf '%s' "$PORTAL" | grep -q 'ZeoKeel'; then echo "oem portal leaked ZeoKeel" >&2; exit 1; fi
echo "updates e2e ok: manifest/channel-reject/version-reject/artifact/oem-portal"
