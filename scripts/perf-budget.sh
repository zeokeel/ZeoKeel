#!/usr/bin/env bash
# M6 performance budget: server ready/health, bundles, mock first token field.
set -euo pipefail
cd "$(dirname "$0")/.."
DB="zk_perf_$$"; ROLE="zk_perf_role_$$"; PORT="${ZEOKEEL_PERF_PORT:-8791}"; OUT=target/evidence/m6/perf.json; mkdir -p target/evidence/m6
cleanup() { [[ -n "${SERVER_PID:-}" ]] && kill "$SERVER_PID" >/dev/null 2>&1 || true; sudo -u postgres dropdb --if-exists --force "$DB" >/dev/null 2>&1 || true; sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true; }
trap cleanup EXIT
sudo -u postgres psql -Atq -c "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';"; sudo -u postgres createdb -O "$ROLE" "$DB"
START=$(date +%s%N)
ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD=admin-pass ./target/debug/zk-server >/tmp/zk-perf.log 2>&1 & SERVER_PID=$!
for _ in $(seq 1 80); do curl -fsS "http://127.0.0.1:$PORT/readyz" >/dev/null 2>&1 && break; sleep 0.1; done
READY_MS=$(( ($(date +%s%N) - START) / 1000000 ))
for _ in $(seq 1 20); do curl -sS -o /dev/null -w '%{time_total}\n' "http://127.0.0.1:$PORT/healthz"; done | python3 -c 'import sys;v=sorted(float(x) for x in sys.stdin);print(v[-1]*1000)' > /tmp/zk-health-p95
HEALTH=$(cat /tmp/zk-health-p95)
ADMIN_JS=$(find apps/admin/web/dist/assets -name '*.js' 2>/dev/null | sort | tail -1); DESKTOP_JS=$(find apps/desktop/web/dist/assets -name '*.js' 2>/dev/null | sort | tail -1)
ADMIN_GZ=$(gzip -c "$ADMIN_JS" 2>/dev/null | wc -c); DESKTOP_GZ=$(gzip -c "$DESKTOP_JS" 2>/dev/null | wc -c)
python3 - "$READY_MS" "$HEALTH" "${ADMIN_GZ:-0}" "${DESKTOP_GZ:-0}" "$OUT" <<'PY'
import json,sys,pathlib
ready,health,admin,desktop=map(float,sys.argv[1:5])
result={"server_ready_ms":ready,"health_p95_ms":health,"admin_js_bytes":int(admin),"desktop_js_bytes":int(desktop),"gateway_p50_ms":None,"gateway_p95_ms":None,"image_size_bytes":None,"unverified":["image_size","gateway_latency","mock_first_token"],"budget":{"server_ready_ms":2000,"health_p95_ms":50,"admin_gzip_kb":400,"desktop_gzip_kb":500,"mock_first_token_ms":300},"status":"unverified"}
pathlib.Path(sys.argv[5]).write_text(json.dumps(result,indent=2)+"\n")
print(json.dumps(result))
sys.exit(0 if result["status"]=="pass" else 1)
PY
