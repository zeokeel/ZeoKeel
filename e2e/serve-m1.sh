#!/usr/bin/env bash
# Real `serve` carrier for m1 e2e: temp ZEOKEEL_HOME + SQLite, a deterministic fake
# Chat Completions upstream (rig provider), and a baseline git workspace for the
# inspector changes tab. No external network.
set -euo pipefail
cd "$(dirname "$0")/.."

SERVE_PORT="${ZEOKEEL_M1_SERVE_PORT:-8788}"
UPSTREAM_PORT="${ZEOKEEL_M1_UPSTREAM_PORT:-8998}"
HOME_DIR="$PWD/target/evidence/m1/home"

cleanup() {
  [[ -n "${SERVE_PID:-}" ]] && kill "$SERVE_PID" >/dev/null 2>&1 || true
  [[ -n "${UPSTREAM_PID:-}" ]] && kill "$UPSTREAM_PID" >/dev/null 2>&1 || true
}
trap cleanup EXIT
trap 'exit 0' INT TERM

# --- deterministic workspace baseline (git repo so the changes tab has a source) ---
rm -rf "$HOME_DIR"
mkdir -p "$HOME_DIR/demo-data"
cp demo-data/hello.txt "$HOME_DIR/demo-data/hello.txt"
printf 'delete me\n' > "$HOME_DIR/demo-data/old.txt"
cat > "$HOME_DIR/.gitignore" <<'GITIGNORE'
sessions.db*
agent-memory/
.agents/
GITIGNORE
git -C "$HOME_DIR" init -q
git -C "$HOME_DIR" config user.email "e2e@zeokeel.invalid"
git -C "$HOME_DIR" config user.name "ZeoKeel e2e"
git -C "$HOME_DIR" add -A
git -C "$HOME_DIR" commit -qm "m1-serve baseline"

# --- fake OpenAI-compatible upstream used by llm-rig (deterministic tool call + usage) ---
cat > /tmp/zk-m1-fake-upstream.py <<'PY'
import json, sys
from http.server import BaseHTTPRequestHandler, HTTPServer


def respond(body):
    messages = body.get("messages", [])
    user_indices = [i for i, m in enumerate(messages) if m.get("role") == "user"]
    last_user = user_indices[-1] if user_indices else -1
    user = str(messages[last_user].get("content") or "") if last_user >= 0 else ""
    tools_since = len([m for m in messages[last_user + 1:] if m.get("role") == "tool"])
    final_usage = {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
    zero_usage = {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
    if "读取" in user and tools_since == 0:
        call = {"id": "call_read", "type": "function", "function": {"name": "fs.read", "arguments": json.dumps({"path": "demo-data/hello.txt"})}}
        return {"id": "chatcmpl-tool", "object": "chat.completion", "choices": [{"index": 0, "finish_reason": "tool_calls", "message": {"role": "assistant", "content": None, "tool_calls": [call]}}], "usage": zero_usage}
    return {"id": "chatcmpl-final", "object": "chat.completion", "choices": [{"index": 0, "finish_reason": "stop", "message": {"role": "assistant", "content": "Hello from ZeoKeel"}}], "usage": final_usage}


class H(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get("content-length", "0"))
        body = json.loads(self.rfile.read(length) or b"{}")
        data = json.dumps(respond(body)).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, *_args):
        pass


HTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
PY
python3 /tmp/zk-m1-fake-upstream.py "$UPSTREAM_PORT" >/tmp/zk-m1-fake-upstream.log 2>&1 &
UPSTREAM_PID=$!

# --- builds + serve ---
pnpm --dir apps/desktop/web build >/dev/null
cargo build -q -p zk-desktop

HOME="$HOME_DIR" ZEOKEEL_HOME="$HOME_DIR" ZEOKEEL_SERVE_ADDR="127.0.0.1:$SERVE_PORT" \
ZEOKEEL_OPENAI_BASE_URL="http://127.0.0.1:$UPSTREAM_PORT/v1" ZEOKEEL_OPENAI_API_KEY=test \
ZEOKEEL_OPENAI_MODEL=test-model ZEOKEEL_AGENT_MAX_STEPS=3 ZEOKEEL_AGENT_STREAMING=0 \
./target/debug/zk-desktop --serve >/tmp/zk-m1-serve.log 2>&1 &
SERVE_PID=$!

for _ in $(seq 1 160); do
  if curl -fsS "http://127.0.0.1:$SERVE_PORT/healthz" >/dev/null 2>&1; then
    break
  fi
  sleep 0.25
done
wait "$SERVE_PID"
