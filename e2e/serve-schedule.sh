#!/usr/bin/env bash
# Real schedule-executor auto-run e2e: fresh ZEOKEEL_HOME/SQLite, a due tool-created
# schedule in the workspace, deterministic fake OpenAI-compatible upstream, and the
# serve carrier with ZEOKEEL_ENABLE_SCHEDULE_AUTORUN=1. No external network.
set -euo pipefail
cd "$(dirname "$0")/.."

UI_PORT="${ZEOKEEL_SCHEDULE_UI_PORT:-8797}"
UPSTREAM_PORT="${ZEOKEEL_SCHEDULE_UPSTREAM_PORT:-8991}"
HOME_DIR="target/evidence/round11/schedule-home"
UI_URL="http://127.0.0.1:$UI_PORT"

stop_pid() {
  local pid="$1"
  [[ -n "$pid" ]] || return 0
  kill "$pid" 2>/dev/null || true
  for _ in $(seq 1 40); do kill -0 "$pid" 2>/dev/null || break; sleep 0.25; done
  if kill -0 "$pid" 2>/dev/null; then kill -KILL "$pid" 2>/dev/null || true; fi
  wait "$pid" 2>/dev/null || true
}
cleanup() {
  stop_pid "${UI_PID:-}"; stop_pid "${UPSTREAM_PID:-}"
  rm -rf "$HOME_DIR"
}
trap cleanup EXIT

mkdir -p "$HOME_DIR/.zeokeel" target/evidence/round11
# Due tool-created entry (executor shape: origin/prompt/interval>=60/enabled/next_run_ms).
python3 - "$HOME_DIR/.zeokeel/schedules.json" <<'PY'
import json, sys, time
now = int(time.time() * 1000)
entry = {
    "id": "e2e-schedule",
    "origin": "tool-schedule",
    "prompt": "schedule auto-run e2e",
    "interval_seconds": 60,
    "cron": "0 9 * * *",
    "enabled": True,
    "next_run_ms": 0,
    "created_at_ms": now,
}
with open(sys.argv[1], "w", encoding="utf-8") as handle:
    json.dump([entry], handle)
PY

cat > /tmp/zk-schedule-upstream.py <<'PY'
import json, sys
from http.server import BaseHTTPRequestHandler, HTTPServer

FINAL = "scheduled auto-run ok"

def respond(_body):
    return {
        "id": "chatcmpl-schedule",
        "object": "chat.completion",
        "choices": [{"index": 0, "finish_reason": "stop", "message": {"role": "assistant", "content": FINAL}}],
        "usage": {"prompt_tokens": 3, "completion_tokens": 4, "total_tokens": 7},
    }

class H(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get("content-length", "0"))
        self.rfile.read(length)
        data = json.dumps(respond(None)).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, *_args):
        pass

HTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
PY
python3 /tmp/zk-schedule-upstream.py "$UPSTREAM_PORT" >/tmp/zk-schedule-upstream.log 2>&1 &
UPSTREAM_PID=$!

pnpm --dir apps/desktop/web build >/dev/null
cargo build -q -p zk-desktop

HOME="$HOME_DIR" ZEOKEEL_HOME="$HOME_DIR" ZEOKEEL_SERVE_ADDR="127.0.0.1:$UI_PORT" \
ZEOKEEL_ENABLE_SCHEDULE_AUTORUN=1 ZEOKEEL_AGENT_STREAMING=0 \
ZEOKEEL_OPENAI_BASE_URL="http://127.0.0.1:$UPSTREAM_PORT/v1" ZEOKEEL_OPENAI_API_KEY=test \
ZEOKEEL_OPENAI_MODEL=test-model \
./target/debug/zk-desktop --serve >/tmp/zk-schedule-serve.log 2>&1 & UI_PID=$!
for _ in $(seq 1 160); do
  if curl -fsS "$UI_URL/healthz" >/dev/null 2>&1; then break; fi
  sleep 0.25
done
curl -fsS "$UI_URL/healthz" >/dev/null

set +e
ZEOKEEL_E2E_NO_WEBSERVER=1 ZEOKEEL_SCHEDULE_URL="$UI_URL" \
  pnpm exec playwright test e2e/schedule-autorun.spec.ts --reporter=line
STATUS=$?
set -e
echo "schedule auto-run e2e exit: $STATUS"
exit "$STATUS"
