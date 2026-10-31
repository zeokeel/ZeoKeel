#!/usr/bin/env bash
# B14 — 500-user load smoke (env-gated; honest: no fake pass).
#
# Two modes:
#   default            run against an existing server from ZEOKEEL_LOAD_BASE_URL / --base-url.
#                      Requires ZEOKEEL_LOAD_USERNAME + ZEOKEEL_LOAD_PASSWORD. Without a
#                      target env this prints UNVERIFIED and exits 2.
#   --local            provision a throwaway real PostgreSQL + local zk-server (prebuilt
#                      target/debug/zk-server) + deterministic fake Responses upstream,
#                      seed admin/employee/pricing/wallet, then run the same mixed load.
#                      This is a *local* smoke; production 500-user capacity remains
#                      UNVERIFIED until the script runs against a real deployment.
#
# Per virtual user: client login -> GET /healthz -> one POST /v1/responses (non-stream).
# Reports p50/p95 per phase, error count and throughput; writes JSON evidence.
#
# Environment:
#   ZEOKEEL_LOAD_BASE_URL          existing server base URL (external mode)
#   ZEOKEEL_LOAD_USERNAME          employee username for external mode
#   ZEOKEEL_LOAD_PASSWORD          employee password for external mode
#   ZEOKEEL_LOAD_USERS             virtual users (default 500; target of record 500)
#   ZEOKEEL_LOAD_CONCURRENCY       max in-flight requests (default = users; 0 = users)
#   ZEOKEEL_LOAD_RAMP_MS           total ramp-up window spread over submissions (default 0)
#   ZEOKEEL_LOAD_ACCOUNTS          local-mode seeded accounts, requests distributed round-robin
#                                  (default 20; avoids one wallet row serializing all gateway calls)
#   ZEOKEEL_LOAD_MODEL             gateway model id (default deepseek-chat)
#   ZEOKEEL_LOAD_HEALTH_PATH       health endpoint path (default /healthz)
#   ZEOKEEL_LOAD_OUT               JSON evidence path (default phase-b evidence dir)
#   ZEOKEEL_LOAD_MAX_ERRORS        allowed request errors (default 0)
#   ZEOKEEL_LOAD_HEALTH_P95_MS     health p95 budget (default 500)
#   ZEOKEEL_LOAD_LOGIN_P95_MS      login p95 budget (default 5000)
#   ZEOKEEL_LOAD_GATEWAY_P95_MS    gateway p95 budget (default 5000)
#   ZEOKEEL_LOAD_PORT              local-mode server port (default 8792)
#   ZEOKEEL_LOAD_UPSTREAM_PORT     local-mode fake upstream port (default 8992)
#
# Exit: 0 pass (thresholds met); 1 errors/threshold breach; 2 UNVERIFIED (no target
# env, missing credentials/tools, or local provisioning impossible).
set -uo pipefail
cd "$(dirname "$0")/.."

MODE="external"
BASE_URL="${ZEOKEEL_LOAD_BASE_URL:-}"
USERS="${ZEOKEEL_LOAD_USERS:-500}"
ACCOUNTS="${ZEOKEEL_LOAD_ACCOUNTS:-20}"
CONCURRENCY="${ZEOKEEL_LOAD_CONCURRENCY:-0}"
RAMP_MS="${ZEOKEEL_LOAD_RAMP_MS:-0}"
MODEL="${ZEOKEEL_LOAD_MODEL:-deepseek-chat}"
HEALTH_PATH="${ZEOKEEL_LOAD_HEALTH_PATH:-/healthz}"
OUT="${ZEOKEEL_LOAD_OUT:-target/evidence/release-1.0-rc1/phase-b/b12b14/load-smoke.json}"
MAX_ERRORS="${ZEOKEEL_LOAD_MAX_ERRORS:-0}"
HEALTH_P95_BUDGET="${ZEOKEEL_LOAD_HEALTH_P95_MS:-500}"
LOGIN_P95_BUDGET="${ZEOKEEL_LOAD_LOGIN_P95_MS:-5000}"
GATEWAY_P95_BUDGET="${ZEOKEEL_LOAD_GATEWAY_P95_MS:-5000}"
PORT="${ZEOKEEL_LOAD_PORT:-8792}"
UPSTREAM_PORT="${ZEOKEEL_LOAD_UPSTREAM_PORT:-8992}"
WORK_DIR="target/evidence/release-1.0-rc1/phase-b/b12b14/load-work"

usage() {
  sed -n '2,/^set -uo pipefail/p' "$0" | sed 's/^# \{0,1\}//; /^set -uo pipefail/d'
}

while (($# > 0)); do
  case "$1" in
    --local) MODE="local"; shift ;;
    --base-url) BASE_URL="${2:-}"; shift 2 ;;
    --base-url=*) BASE_URL="${1#*=}"; shift ;;
    --users) USERS="${2:-}"; shift 2 ;;
    --users=*) USERS="${1#*=}"; shift ;;
    --concurrency) CONCURRENCY="${2:-}"; shift 2 ;;
    --concurrency=*) CONCURRENCY="${1#*=}"; shift ;;
    --ramp-ms) RAMP_MS="${2:-}"; shift 2 ;;
    --ramp-ms=*) RAMP_MS="${1#*=}"; shift ;;
    --help|-h) usage; exit 0 ;;
    *) echo "load-smoke: unknown argument: $1 (try --help)" >&2; exit 2 ;;
  esac
done

[[ "$USERS" =~ ^[0-9]+$ ]] || { echo "load-smoke: --users must be a positive integer"; exit 2; }
[[ "$CONCURRENCY" =~ ^[0-9]+$ ]] || { echo "load-smoke: --concurrency must be a non-negative integer"; exit 2; }
[[ "$RAMP_MS" =~ ^[0-9]+$ ]] || { echo "load-smoke: --ramp-ms must be a non-negative integer"; exit 2; }
[[ "$CONCURRENCY" = "0" ]] && CONCURRENCY="$USERS"
mkdir -p "$(dirname "$OUT")"

SERVER_PID=""; UPSTREAM_PID=""; DB=""; ROLE=""
cleanup() {
  [[ -n "$SERVER_PID" ]] && kill "$SERVER_PID" >/dev/null 2>&1 || true
  [[ -n "$UPSTREAM_PID" ]] && kill "$UPSTREAM_PID" >/dev/null 2>&1 || true
  if [[ -n "$DB" ]]; then
    sudo -u postgres dropdb --if-exists --force "$DB" >/dev/null 2>&1 || true
    sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true
  fi
  # A18: if --local composed the enterprise server, restore the committed personal baseline.
  if [[ "${COMPOSITION:-}" == "composed-enterprise" ]]; then
    bash scripts/restore-baseline.sh >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

unverified() { echo "load-smoke: UNVERIFIED — $1"; exit 2; }

if [[ "$MODE" = "external" && -z "$BASE_URL" ]]; then
  unverified "no target env: set ZEOKEEL_LOAD_BASE_URL (and ZEOKEEL_LOAD_USERNAME/PASSWORD), or use --local for a local smoke"
fi
command -v python3 >/dev/null 2>&1 || unverified "python3 not available"
command -v curl >/dev/null 2>&1 || unverified "curl not available"

# ---------------------------------------------------------------- local provisioning
if [[ "$MODE" = "local" ]]; then
  command -v psql >/dev/null 2>&1 || unverified "psql not available for local provisioning"
  BASE_URL="http://127.0.0.1:$PORT"
  DB="zk_load_$$"; ROLE="zk_load_$$"
  LOCAL_USER="load-user-00"; LOCAL_PASS="load-pass-123"
  rm -rf "$WORK_DIR"; mkdir -p "$WORK_DIR"
  cat > "$WORK_DIR/fake-upstream.py" <<'PY'
import asyncio
import json
import sys

RESPONSE = json.dumps({
    "id": "resp-load-smoke",
    "object": "response",
    "status": "completed",
    "output": [{"type": "message", "content": [{"type": "output_text", "text": "pong"}]}],
    "usage": {"input_tokens": 10, "output_tokens": 5, "total_tokens": 15},
}).encode()
HEAD = b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: " + str(len(RESPONSE)).encode() + b"\r\nconnection: keep-alive\r\n\r\n"


async def handle(reader, writer):
    try:
        while True:
            request_line = await reader.readline()
            if not request_line:
                break
            parts = request_line.decode("latin-1").strip().split(" ")
            if len(parts) < 2:
                break
            headers = {}
            while True:
                line = await reader.readline()
                if line in (b"\r\n", b"\n", b""):
                    break
                key, _, value = line.decode("latin-1").partition(":")
                headers[key.strip().lower()] = value.strip()
            length = int(headers.get("content-length", "0") or 0)
            if length:
                await reader.readexactly(length)
            writer.write(HEAD + RESPONSE)
            await writer.drain()
    except (asyncio.IncompleteReadError, ConnectionError, ValueError):
        pass
    finally:
        writer.close()


async def main(port):
    server = await asyncio.start_server(handle, "127.0.0.1", port, backlog=1024)
    async with server:
        await server.serve_forever()


asyncio.run(main(int(sys.argv[1])))
PY
  python3 "$WORK_DIR/fake-upstream.py" "$UPSTREAM_PORT" >"$WORK_DIR/fake-upstream.log" 2>&1 &
  UPSTREAM_PID=$!
  sudo -u postgres psql -Atq -c "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';" >/dev/null 2>&1 \
    || unverified "cannot create throwaway PostgreSQL role (is local postgres available?)"
  sudo -u postgres createdb -O "$ROLE" "$DB" >/dev/null 2>&1 || unverified "cannot create throwaway PostgreSQL database"
  # Local-only acceleration: this host's durable commit is ~45ms, which would make the
  # smoke measure disk fsync latency rather than service plumbing. Durability does not
  # matter for a throwaway DB; production capacity stays UNVERIFIED (external mode).
  sudo -u postgres psql -Atq -d "$DB" -c "ALTER DATABASE \"$DB\" SET synchronous_commit = off;" >/dev/null 2>&1 || true
  # A18 target trimming: the committed personal baseline generates a stub zk-server
  # (exit 2, zero srv deps). --local always composes enterprise first and restores the
  # personal baseline in cleanup (see scripts/ensure-enterprise-composition.sh).
  COMPOSITION="$(bash scripts/ensure-enterprise-composition.sh)" \
    || unverified "cannot compose the enterprise server (A18)"
  if [[ "$COMPOSITION" == "composed-enterprise" || ! -x target/debug/zk-server ]]; then
    echo "load-smoke: building zk-server (enterprise composition)"
    cargo build -q -p zk-server >"$WORK_DIR/build.log" 2>&1 || unverified "cargo build -p zk-server failed"
  fi
  ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" \
  ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" \
  ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD="admin-pass" \
  ZEOKEEL_UPSTREAM_BASE_URL="http://127.0.0.1:$UPSTREAM_PORT/v1" \
  ZEOKEEL_UPSTREAM_API_KEY="load-upstream-key" \
  ZEOKEEL_ALLOW_INSECURE_UPSTREAM=1 \
    ./target/debug/zk-server >"$WORK_DIR/server.log" 2>&1 &
  SERVER_PID=$!
  ready=0
  for _ in $(seq 1 200); do
    if curl -fsS "$BASE_URL/healthz" >/dev/null 2>&1; then ready=1; break; fi
    sleep 0.05
  done
  [[ "$ready" = "1" ]] || unverified "local server did not become ready: $(tail -3 "$WORK_DIR/server.log" 2>/dev/null)"
  ADMIN_JSON="$(curl -fsS -c "$WORK_DIR/admin.cookies" -X POST "$BASE_URL/api/admin/auth/login" \
    -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass"}')" \
    || unverified "local admin login failed"
  CSRF="$(printf '%s' "$ADMIN_JSON" | python3 -c 'import json,sys;print(json.load(sys.stdin)["csrf_token"])')"
  LOCAL_NAMES=""
  for index in $(seq 0 $((ACCOUNTS - 1))); do
    name="$(printf 'load-user-%02d' "$index")"
    curl -fsS -b "$WORK_DIR/admin.cookies" -H "X-CSRF-Token: $CSRF" -X POST "$BASE_URL/api/admin/users" \
      -H 'content-type: application/json' -d "{\"username\":\"$name\",\"password\":\"$LOCAL_PASS\",\"role\":\"employee\"}" >/dev/null \
      || unverified "local user seed failed ($name)"
    LOCAL_NAMES="${LOCAL_NAMES}${LOCAL_NAMES:+,}${name}"
  done
  curl -fsS -b "$WORK_DIR/admin.cookies" -H "X-CSRF-Token: $CSRF" -X PUT "$BASE_URL/api/admin/pricing/$MODEL" \
    -H 'content-type: application/json' -d '{"input_per_1k_micros":1000,"cached_per_1k_micros":100,"output_per_1k_micros":2000}' >/dev/null \
    || unverified "local pricing seed failed"
  USERS_JSON="$(curl -fsS -b "$WORK_DIR/admin.cookies" "$BASE_URL/api/admin/users")" || unverified "local user list failed"
  for name in ${LOCAL_NAMES//,/ }; do
    user_id="$(printf '%s' "$USERS_JSON" | python3 -c "import json,sys;print([u['id'] for u in json.load(sys.stdin)['users'] if u['username']=='$name'][0])")"
    curl -fsS -b "$WORK_DIR/admin.cookies" -H "X-CSRF-Token: $CSRF" -X POST "$BASE_URL/api/admin/wallet/$user_id/adjust" \
      -H 'content-type: application/json' \
      -d "{\"delta_micros\":1000000000,\"reason\":\"load smoke grant\",\"idempotency_key\":\"load-grant-$name-$$\"}" >/dev/null \
      || unverified "local wallet grant failed ($name)"
  done
  export ZEOKEEL_LOAD_USERNAME="$LOCAL_USER" ZEOKEEL_LOAD_PASSWORD="$LOCAL_PASS" ZEOKEEL_LOAD_USERNAMES="$LOCAL_NAMES"
fi

[[ -n "${ZEOKEEL_LOAD_USERNAME:-}" && -n "${ZEOKEEL_LOAD_PASSWORD:-}" ]] \
  || unverified "missing credentials: set ZEOKEEL_LOAD_USERNAME and ZEOKEEL_LOAD_PASSWORD"

# ---------------------------------------------------------------- mixed load harness
python3 - "$BASE_URL" "$USERS" "$MODEL" "$HEALTH_PATH" "$OUT" "$MODE" "$MAX_ERRORS" \
  "$HEALTH_P95_BUDGET" "$LOGIN_P95_BUDGET" "$GATEWAY_P95_BUDGET" "$CONCURRENCY" "$RAMP_MS" <<'PY'
import json
import math
import os
import sys
import threading
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor

(base, users, model, health_path, out, mode, max_errors, health_budget, login_budget, gateway_budget,
 concurrency, ramp_ms) = sys.argv[1:13]
users = int(users)
max_errors = int(max_errors)
concurrency = max(1, min(int(concurrency), users))
ramp_ms = int(ramp_ms)
username = os.environ["ZEOKEEL_LOAD_USERNAME"]
password = os.environ["ZEOKEEL_LOAD_PASSWORD"]
usernames = [name for name in os.environ.get("ZEOKEEL_LOAD_USERNAMES", username).split(",") if name]
health_budget = float(health_budget)
login_budget = float(login_budget)
gateway_budget = float(gateway_budget)
# A login stampede is startup cost, not the steady-state target; in --local mode login
# p95 is informational unless explicitly budgeted.
login_budget_applied = (mode != "local") or ("ZEOKEEL_LOAD_LOGIN_P95_MS" in os.environ)


def request(method, path, body=None, token=None, timeout=90):
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(base + path, data=data, method=method)
    if data is not None:
        req.add_header("content-type", "application/json")
    if token is not None:
        req.add_header("authorization", "Bearer " + token)
    started = time.perf_counter()
    try:
        with urllib.request.urlopen(req, timeout=timeout) as response:
            payload = response.read()
            return (response.status, payload, (time.perf_counter() - started) * 1000.0, None)
    except urllib.error.HTTPError as error:
        body_bytes = error.read()
        return (error.code, body_bytes, (time.perf_counter() - started) * 1000.0,
                "http %s: %s" % (error.code, body_bytes[:160].decode("utf-8", "replace")))
    except Exception as error:  # transport/timeout
        return (0, b"", (time.perf_counter() - started) * 1000.0, "%s: %s" % (type(error).__name__, error))


lock = threading.Lock()
errors = []
phases = {"login_ms": [], "health_ms": [], "gateway_ms": []}


def record(phase, status, elapsed, detail=None):
    with lock:
        phases[phase].append(elapsed)
        if detail is not None or not (200 <= status < 300):
            errors.append({"phase": phase, "status": status, "detail": detail})


def stats(values):
    if not values:
        return {"count": 0, "p50_ms": None, "p95_ms": None, "max_ms": None, "mean_ms": None}
    ordered = sorted(values)

    def pct(p):
        index = max(0, min(len(ordered) - 1, math.ceil(p * len(ordered)) - 1))
        return round(ordered[index], 2)

    return {"count": len(ordered), "p50_ms": pct(0.50), "p95_ms": pct(0.95), "max_ms": round(ordered[-1], 2),
            "mean_ms": round(sum(ordered) / len(ordered), 2)}


def run_phase(count, worker):
    """Run count tasks with bounded concurrency and an optional total ramp-up window."""
    started = time.perf_counter()
    delay = (ramp_ms / 1000.0) / count if count > 0 and ramp_ms > 0 else 0.0
    with ThreadPoolExecutor(max_workers=concurrency) as pool:
        futures = []
        for index in range(count):
            futures.append(pool.submit(worker, index))
            if delay > 0:
                time.sleep(delay)
        for future in futures:
            future.result()
    return time.perf_counter() - started


tokens = [None] * users


def login(index):
    status, payload, elapsed, detail = request(
        "POST", "/api/client/v1/auth/login",
        {"username": usernames[index % len(usernames)], "password": password, "device_id": "load-%04d" % index},
    )
    record("login_ms", status, elapsed, detail)
    if status != 200:
        return
    try:
        tokens[index] = json.loads(payload)["access_token"]
    except Exception as error:
        with lock:
            errors.append({"phase": "login_ms", "status": 0, "detail": "bad login payload: %s" % error})


started_all = time.perf_counter()
login_wall = run_phase(users, login)


def health(_index):
    status, _, elapsed, detail = request("GET", health_path)
    record("health_ms", status, elapsed, detail)


health_wall = run_phase(users, health)


def gateway(index):
    token = tokens[index]
    if token is None:
        return
    status, payload, elapsed, detail = request(
        "POST", "/v1/responses",
        {"model": model, "input": "load smoke ping", "stream": False},
        token=token,
    )
    if status == 200:
        try:
            decoded = json.loads(payload)
            if "usage" not in decoded and "response" not in decoded:
                detail = "gateway response missing usage/response"
        except Exception as error:
            detail = "gateway response not JSON: %s" % error
    record("gateway_ms", status, elapsed, detail)


gateway_wall = run_phase(users, gateway)
wall = time.perf_counter() - started_all

summary = {phase: stats(values) for phase, values in phases.items()}
throughput = {
    "login_per_second": round(users / login_wall, 2) if login_wall > 0 else None,
    "health_per_second": round(users / health_wall, 2) if health_wall > 0 else None,
    "gateway_per_second": round(users / gateway_wall, 2) if gateway_wall > 0 else None,
}
errors_by_phase_status = {}
for error in errors:
    key = "%s:%s" % (error["phase"], error["status"])
    errors_by_phase_status[key] = errors_by_phase_status.get(key, 0) + 1

failures = []
if len(errors) > max_errors:
    failures.append("errors=%d > allowed=%d" % (len(errors), max_errors))
for phase, budget in (("health_ms", health_budget), ("gateway_ms", gateway_budget)):
    value = summary[phase]["p95_ms"]
    if value is None:
        failures.append("%s produced no successful samples" % phase)
    elif value > budget:
        failures.append("%s p95=%.2fms > budget=%.2fms" % (phase, value, budget))
if login_budget_applied:
    value = summary["login_ms"]["p95_ms"]
    if value is not None and value > login_budget:
        failures.append("login_ms p95=%.2fms > budget=%.2fms" % (value, login_budget))

payload = {
    "schema": 3,
    "mode": mode,
    "users": users,
    "concurrency": concurrency,
    "ramp_ms": ramp_ms,
    "accounts": len(usernames),
    "model": model,
    "base_url": base,
    "status": "fail" if failures else "pass",
    "wall_seconds": round(wall, 3),
    "phase_wall_seconds": {"login": round(login_wall, 3), "health": round(health_wall, 3), "gateway": round(gateway_wall, 3)},
    "throughput_per_second": throughput,
    "phases": summary,
    "errors": errors[:50],
    "error_count": len(errors),
    "errors_by_phase_status": errors_by_phase_status,
    "budget": {
        "max_errors": int(max_errors),
        "health_p95_ms": health_budget,
        "gateway_p95_ms": gateway_budget,
        "login_p95_ms": login_budget,
        "login_p95_applied": login_budget_applied,
    },
    "failures": failures,
    "note": "local --local run verifies harness plumbing and error-free behaviour on this host; "
            "login p95 is a local Argon2 stampede (informational unless ZEOKEEL_LOAD_LOGIN_P95_MS is set); "
            "the throwaway local DB runs with synchronous_commit=off; production 500-user capacity remains "
            "UNVERIFIED until run against a real deployment",
}
with open(out, "w", encoding="utf-8") as handle:
    handle.write(json.dumps(payload, indent=2) + "\n")
print(json.dumps({"status": payload["status"], "mode": mode, "users": users, "concurrency": concurrency,
                  "ramp_ms": ramp_ms, "wall_seconds": payload["wall_seconds"], "phases": summary,
                  "throughput": throughput, "error_count": len(errors),
                  "errors_by_phase_status": errors_by_phase_status, "failures": failures}, indent=2))
sys.exit(1 if failures else 0)
PY
HARNESS_EXIT=$?

STATUS="$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["status"])' "$OUT" 2>/dev/null || echo fail)"
if [[ "$HARNESS_EXIT" = "0" && "$STATUS" = "pass" ]]; then
  echo "load-smoke: pass (mode=$MODE users=$USERS) -> $OUT"
  exit 0
fi
echo "load-smoke: fail (mode=$MODE users=$USERS) -> $OUT"
exit 1
