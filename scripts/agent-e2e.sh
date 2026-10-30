#!/usr/bin/env bash
# Personal rig agent e2e: composed personal build + fake Chat Completions + Playwright.
set -euo pipefail
cd "$(dirname "$0")/.."
UI_PORT=8798; UPSTREAM_PORT=8802
cleanup() {
  [[ -n "${UI_PID:-}" ]] && kill "$UI_PID" >/dev/null 2>&1 || true
  [[ -n "${UPSTREAM_PID:-}" ]] && kill "$UPSTREAM_PID" >/dev/null 2>&1 || true
  cargo run -q -p zk -- compose --edition enterprise --profile default --channel official >/dev/null 2>&1 || true
}
trap cleanup EXIT
mkdir -p target/evidence/agent-rig
cat > /tmp/zk-agent-fake.py <<'PY'
import json,sys
from http.server import BaseHTTPRequestHandler, HTTPServer
def respond(body):
    messages=body.get("messages",[])
    if any(m.get("role")=="tool" for m in messages):
        return {"id":"chatcmpl-final","object":"chat.completion","choices":[{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":"已完成工具调用并通过审批。"}}],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}
    user=" ".join(str(m.get("content") or "") for m in messages if m.get("role")=="user")
    if "ask" in user:
        call={"id":"call_ask","type":"function","function":{"name":"ask_user_question","arguments":json.dumps({"questions":[{"id":"name","question":"你的名字？"}]})}}
    else:
        call={"id":"call_write","type":"function","function":{"name":"fs.write","arguments":json.dumps({"path":"agent-e2e.txt","content":"hello from rig"})}}
    return {"id":"chatcmpl-tool","object":"chat.completion","choices":[{"index":0,"finish_reason":"tool_calls","message":{"role":"assistant","content":None,"tool_calls":[call]}}],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}
class H(BaseHTTPRequestHandler):
    def do_POST(self):
        length=int(self.headers.get("content-length","0")); body=json.loads(self.rfile.read(length) or b"{}")
        data=json.dumps(respond(body)).encode()
        self.send_response(200); self.send_header("content-type","application/json"); self.send_header("content-length",str(len(data))); self.end_headers(); self.wfile.write(data)
    def log_message(self,*args): pass
HTTPServer(("127.0.0.1",int(sys.argv[1])),H).serve_forever()
PY
python3 /tmp/zk-agent-fake.py "$UPSTREAM_PORT" >/tmp/zk-agent-fake.log 2>&1 & UPSTREAM_PID=$!
cargo run -q -p zk -- compose --edition personal --profile dev-live --channel official >/dev/null
cargo build -q -p zk-desktop
pnpm --dir apps/desktop/web build >/dev/null
rm -rf target/evidence/agent-rig/home && mkdir -p target/evidence/agent-rig/home
HOME="$PWD/target/evidence/agent-rig/home" ZEOKEEL_HOME="$PWD/target/evidence/agent-rig/home" ZEOKEEL_SERVE_ADDR="127.0.0.1:$UI_PORT" ZEOKEEL_AGENT_STREAMING=0 \
ZEOKEEL_OPENAI_BASE_URL="http://127.0.0.1:$UPSTREAM_PORT/v1" ZEOKEEL_OPENAI_API_KEY=test ZEOKEEL_OPENAI_MODEL=test-model \
./target/debug/zk-desktop --serve >/tmp/zk-agent-ui.log 2>&1 & UI_PID=$!
for _ in $(seq 1 80); do curl -fsS "http://127.0.0.1:$UI_PORT/healthz" >/dev/null 2>&1 && break; sleep 0.25; done
pnpm exec playwright test e2e/agent-rig.spec.ts --config=playwright.agent.config.ts
echo "agent e2e ok"
