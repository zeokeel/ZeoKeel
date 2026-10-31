#!/usr/bin/env bash
# M6 performance budget (docs/37, hardening spec §3): measured values, no null-with-pass.
#
# Measures against real local processes/artifacts:
#   - server ready ms (process start -> /readyz ok, real PG)
#   - /healthz p95 ms (30 samples)
#   - admin / desktop initial-bundle gzip bytes (assets referenced by dist/index.html)
#   - mock first token ms: real gateway /v1/responses streaming against the local fake
#     upstream (OpenAI Responses SSE shape), time-to-first-byte via curl
#   - gateway p50/p95 ms (10 non-stream mock calls)
# Budgets: ready <=2000ms, health p95 <=50ms, admin <=400KB gzip, desktop <=500KB gzip,
#          mock first token <=300ms.
#
# Honest UNVERIFIED (recorded, never fake-passed): image size (no Docker image locally),
# 500-user capacity load test (no harness/keys on this host). Overall status is therefore
# `unverified` (exit 2) even when all local budgets pass; a budget breach is exit 1.
#
# Usage: bash scripts/perf-budget.sh [--out <json>]
# Exit: 0 all budgets pass with no UNVERIFIED; 2 budgets pass + UNVERIFIED; 1 breach.
set -uo pipefail
cd "$(dirname "$0")/.."

OUT="${ZEOKEEL_PERF_JSON:-target/evidence/m6/perf.json}"
WORK="target/evidence/m6/perf"
PORT="${ZEOKEEL_PERF_PORT:-8791}"
UPSTREAM_PORT="${ZEOKEEL_PERF_UPSTREAM_PORT:-8991}"
DB="zk_perf_$$"; ROLE="zk_perf_role_$$"
SERVER_PID=""; UPSTREAM_PID=""
mkdir -p "$(dirname "$OUT")"; rm -rf "$WORK"; mkdir -p "$WORK"
cleanup() {
  if [[ -n "$SERVER_PID" ]]; then kill "$SERVER_PID" >/dev/null 2>&1 || true; fi
  if [[ -n "$UPSTREAM_PID" ]]; then kill "$UPSTREAM_PID" >/dev/null 2>&1 || true; fi
  sudo -u postgres dropdb --if-exists --force "$DB" >/dev/null 2>&1 || true
  sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true
}
trap cleanup EXIT
BASE="http://127.0.0.1:$PORT"
BUDGET_READY=2000; BUDGET_HEALTH=50; BUDGET_ADMIN_KB=400; BUDGET_DESKTOP_KB=500; BUDGET_FIRST_TOKEN=300
FAILURES=""

fail_budget() { FAILURES="${FAILURES}${FAILURES:+, }$1"; }

# ---------------------------------------------------------------- fake upstream (Responses SSE)
cat > "$WORK/fake-upstream.py" <<'PY'
import json
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get("content-length", "0"))
        self.rfile.read(length)
        self.send_response(200)
        self.send_header("content-type", "text/event-stream")
        self.end_headers()
        chunks = [
            b'event: response.created\ndata: {"type":"response.created","response":{"id":"r1","usage":null}}\n\n',
            b'event: response.output_text.delta\ndata: {"type":"response.output_text.delta","delta":"hi"}\n\n',
            b'event: response.completed\ndata: {"type":"response.completed","response":{"usage":{"input_tokens":10,"output_tokens":20}}}\n\n',
        ]
        for chunk in chunks:
            self.wfile.write(chunk)
            self.wfile.flush()

    def log_message(self, *_args):
        pass


HTTPServer(("127.0.0.1", int(sys.argv[1])), Handler).serve_forever()
PY
python3 "$WORK/fake-upstream.py" "$UPSTREAM_PORT" >"$WORK/fake-upstream.log" 2>&1 &
UPSTREAM_PID=$!
for _ in $(seq 1 50); do curl -sS -o /dev/null "http://127.0.0.1:$UPSTREAM_PORT/" 2>/dev/null || break; sleep 0.1; done

# ---------------------------------------------------------------- server + seed
sudo -u postgres psql -Atq -c "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';" || { echo "perf-budget: CREATE ROLE failed"; exit 1; }
sudo -u postgres createdb -O "$ROLE" "$DB" || { echo "perf-budget: createdb failed"; exit 1; }
if [[ ! -x target/debug/zk-server ]]; then cargo build -q --offline -p zk-server || { echo "perf-budget: build failed"; exit 1; }; fi
START=$(date +%s%N)
ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" \
ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" \
ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD=admin-pass \
ZEOKEEL_UPSTREAM_BASE_URL="http://127.0.0.1:$UPSTREAM_PORT/v1" \
ZEOKEEL_UPSTREAM_API_KEY=perf-upstream-key \
  ./target/debug/zk-server >"$WORK/server.log" 2>&1 &
SERVER_PID=$!
for _ in $(seq 1 200); do curl -fsS "$BASE/readyz" >/dev/null 2>&1 && break; sleep 0.05; done
READY_MS=$(( ($(date +%s%N) - START) / 1000000 ))
curl -fsS "$BASE/readyz" >/dev/null || { echo "perf-budget: server not ready: $(tail -5 "$WORK/server.log")"; exit 1; }

# admin session + user + pricing + wallet grant so the gateway can stream and settle.
ADMIN="$(curl -fsS -c "$WORK/admin.cookies" -X POST "$BASE/api/admin/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass"}')" || { echo "perf-budget: admin login failed"; exit 1; }
CSRF="$(printf '%s' "$ADMIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["csrf_token"])')"
curl -fsS -b "$WORK/admin.cookies" -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/users" -H 'content-type: application/json' -d '{"username":"perf-user","password":"perf-pass","role":"employee"}' >/dev/null || { echo "perf-budget: create user failed"; exit 1; }
curl -fsS -b "$WORK/admin.cookies" -H "X-CSRF-Token: $CSRF" -X PUT "$BASE/api/admin/pricing/deepseek-chat" -H 'content-type: application/json' -d '{"input_per_1k_micros":1000,"cached_per_1k_micros":100,"output_per_1k_micros":2000}' >/dev/null || { echo "perf-budget: pricing failed"; exit 1; }
USERS_JSON="$(curl -fsS -b "$WORK/admin.cookies" "$BASE/api/admin/users")" || { echo "perf-budget: list users failed"; exit 1; }
PERF_ID="$(printf '%s' "$USERS_JSON" | python3 -c 'import json,sys;print([u["id"] for u in json.load(sys.stdin)["users"] if u["username"]=="perf-user"][0])')"
curl -fsS -b "$WORK/admin.cookies" -H "X-CSRF-Token: $CSRF" -X POST "$BASE/api/admin/wallet/$PERF_ID/adjust" -H 'content-type: application/json' -d '{"delta_micros":1000000,"reason":"perf drill grant","idempotency_key":"perf-grant-1"}' >/dev/null || { echo "perf-budget: wallet adjust failed"; exit 1; }
TOKEN="$(curl -fsS -X POST "$BASE/api/client/v1/auth/login" -H 'content-type: application/json' -d '{"username":"perf-user","password":"perf-pass","device_id":"perf-drill"}' | python3 -c 'import json,sys;print(json.load(sys.stdin)["access_token"])')" || { echo "perf-budget: client login failed"; exit 1; }

# ---------------------------------------------------------------- healthz p95
for _ in $(seq 1 30); do
  curl -sS -o /dev/null -w '%{time_total}\n' "$BASE/healthz" >> "$WORK/health.tsv"
done
HEALTH_P95="$(python3 - "$WORK/health.tsv" <<'PY'
import math, sys
values = sorted(float(line) for line in open(sys.argv[1]) if line.strip())
index = max(0, min(len(values) - 1, math.ceil(0.95 * len(values)) - 1))
print(round(values[index] * 1000, 2))
PY
)"
[[ "$(python3 -c "print(1 if $READY_MS <= $BUDGET_READY else 0)")" == "1" ]] || fail_budget "server_ready_ms>${BUDGET_READY} (${READY_MS}ms)"
[[ "$(python3 -c "print(1 if $HEALTH_P95 <= $BUDGET_HEALTH else 0)")" == "1" ]] || fail_budget "health_p95_ms>${BUDGET_HEALTH} (${HEALTH_P95}ms)"

# ---------------------------------------------------------------- mock first token / gateway latency
FIRST_TOKEN_MS="$(curl -sS -N -o "$WORK/stream.sse" -w '%{time_starttransfer}' -X POST "$BASE/v1/responses" \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"model":"deepseek-chat","input":"hello","stream":true}')"
FIRST_TOKEN_MS="$(python3 -c "print(round(float('$FIRST_TOKEN_MS')*1000, 2))")"
grep -q 'response.completed' "$WORK/stream.sse" || { echo "perf-budget: mock stream did not complete"; exit 1; }
[[ "$(python3 -c "print(1 if $FIRST_TOKEN_MS <= $BUDGET_FIRST_TOKEN else 0)")" == "1" ]] || fail_budget "mock_first_token_ms>${BUDGET_FIRST_TOKEN} (${FIRST_TOKEN_MS}ms)"
for _ in $(seq 1 10); do
  curl -sS -o /dev/null -w '%{time_total}\n' -X POST "$BASE/v1/responses" \
    -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
    -d '{"model":"deepseek-chat","input":"hello"}' >> "$WORK/gateway.tsv"
done
GATEWAY="$(python3 - "$WORK/gateway.tsv" <<'PY'
import math, sys
values = sorted(float(line) for line in open(sys.argv[1]) if line.strip())
def pct(p):
    index = max(0, min(len(values) - 1, math.ceil(p * len(values)) - 1))
    return round(values[index] * 1000, 2)
print(pct(0.50), pct(0.95))
PY
)"
GATEWAY_P50="${GATEWAY% *}"; GATEWAY_P95="${GATEWAY#* }"

# ---------------------------------------------------------------- bundle sizes
measure_bundle() { # dist_dir -> "largest_total_kb initial_gzip_bytes assets"
  local dist="$1"
  python3 - "$dist" <<'PY'
import gzip
import pathlib
import re
import sys

dist = pathlib.Path(sys.argv[1])
index = dist / "index.html"
if not index.is_file():
    print("0 0 0")
    raise SystemExit(0)
html = index.read_text(encoding="utf-8", errors="ignore")
raw_refs = re.findall(r'(?:src|href)="([^"]+\.(?:js|css))"', html)
refs = []
for raw in raw_refs:
    match = re.search(r'(assets/[^"]+\.(?:js|css))', raw)
    if match:
        refs.append(match.group(1))
total = 0
largest = 0
for ref in sorted(set(refs)):
    path = dist / ref
    if not path.is_file():
        continue
    size = len(gzip.compress(path.read_bytes(), 6))
    total += size
    largest = max(largest, size)
print(f"{total} {largest} {len(set(refs))}")
PY
}
ADMIN_BUNDLE="$(measure_bundle apps/admin/web/dist)"
DESKTOP_BUNDLE="$(measure_bundle apps/desktop/web/dist)"
ADMIN_GZ="${ADMIN_BUNDLE%% *}"; ADMIN_REST="${ADMIN_BUNDLE#* }"; ADMIN_LARGEST="${ADMIN_REST%% *}"; ADMIN_ASSETS="${ADMIN_REST#* }"
DESKTOP_GZ="${DESKTOP_BUNDLE%% *}"; DESKTOP_REST="${DESKTOP_BUNDLE#* }"; DESKTOP_LARGEST="${DESKTOP_REST%% *}"; DESKTOP_ASSETS="${DESKTOP_REST#* }"
ADMIN_KB="$(python3 -c "print(round(${ADMIN_GZ:-0}/1024, 1))")"
DESKTOP_KB="$(python3 -c "print(round(${DESKTOP_GZ:-0}/1024, 1))")"
[[ "$(python3 -c "print(1 if $ADMIN_GZ > 0 and $ADMIN_GZ <= $BUDGET_ADMIN_KB*1024 else 0)")" == "1" ]] || fail_budget "admin_bundle>${BUDGET_ADMIN_KB}KB or dist missing (${ADMIN_KB}KB)"
[[ "$(python3 -c "print(1 if $DESKTOP_GZ > 0 and $DESKTOP_GZ <= $BUDGET_DESKTOP_KB*1024 else 0)")" == "1" ]] || fail_budget "desktop_bundle>${BUDGET_DESKTOP_KB}KB or dist missing (${DESKTOP_KB}KB)"

# ---------------------------------------------------------------- JSON
python3 - "$OUT" "$WORK" "$READY_MS" "$HEALTH_P95" "$ADMIN_GZ" "$ADMIN_LARGEST" "$ADMIN_ASSETS" \
  "$DESKTOP_GZ" "$DESKTOP_LARGEST" "$DESKTOP_ASSETS" "$FIRST_TOKEN_MS" "$GATEWAY_P50" "$GATEWAY_P95" \
  "$FAILURES" "$BUDGET_READY" "$BUDGET_HEALTH" "$BUDGET_ADMIN_KB" "$BUDGET_DESKTOP_KB" "$BUDGET_FIRST_TOKEN" \
  "$(git rev-parse --short HEAD 2>/dev/null || echo unknown)" <<'PY'
import json
import pathlib
import sys

(out_path, work, ready, health, admin, admin_largest, admin_assets, desktop, desktop_largest,
 desktop_assets, first_token, gateway_p50, gateway_p95, failures, b_ready, b_health, b_admin,
 b_desktop, b_first_token, head) = sys.argv[1:21]
unverified = ["image_size", "capacity_500_users"]
payload = {
    "schema": 1,
    "head": head,
    "status": "fail" if failures else "unverified",
    "budget": {
        "server_ready_ms": int(b_ready),
        "health_p95_ms": int(b_health),
        "admin_gzip_kb": int(b_admin),
        "desktop_gzip_kb": int(b_desktop),
        "mock_first_token_ms": int(b_first_token),
    },
    "measurements": {
        "server_ready_ms": float(ready),
        "health_p95_ms": float(health),
        "admin_initial_gzip_bytes": int(admin),
        "admin_largest_asset_gzip_bytes": int(admin_largest),
        "admin_initial_assets": int(admin_assets),
        "desktop_initial_gzip_bytes": int(desktop),
        "desktop_largest_asset_gzip_bytes": int(desktop_largest),
        "desktop_initial_assets": int(desktop_assets),
        "mock_first_token_ms": float(first_token),
        "gateway_p50_ms": float(gateway_p50),
        "gateway_p95_ms": float(gateway_p95),
        "image_size_bytes": None,
        "capacity_500_users": None,
    },
    "failures": [item for item in failures.split(", ") if item],
    "unverified": unverified,
    "note": "image_size and 500-user capacity have no local harness => UNVERIFIED; null is never interpreted as pass",
}
pathlib.Path(out_path).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
print(json.dumps(payload["measurements"], indent=2))
print(f"perf-budget: {payload['status']} (failures={len(payload['failures'])}, unverified={len(unverified)}) -> {out_path}")
PY
STATUS="$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["status"])' "$OUT")"
case "$STATUS" in
  pass) exit 0 ;;
  unverified) exit 2 ;;
  *) exit 1 ;;
esac
