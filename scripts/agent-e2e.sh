#!/usr/bin/env bash
# Personal rig agent e2e: composed personal build + fake Chat Completions + Playwright.
set -euo pipefail
cd "$(dirname "$0")/.."
UI_PORT="${ZEOKEEL_AGENT_UI_PORT:-$((8860 + RANDOM % 100))}"; UPSTREAM_PORT="${ZEOKEEL_AGENT_UPSTREAM_PORT:-$((8960 + RANDOM % 100))}"; UPSTREAM2_PORT=$((UPSTREAM_PORT + 1))
cleanup() {
  [[ -n "${UI_PID:-}" ]] && kill "$UI_PID" >/dev/null 2>&1 || true
  [[ -n "${UPSTREAM_PID:-}" ]] && kill "$UPSTREAM_PID" >/dev/null 2>&1 || true
  [[ -n "${UPSTREAM2_PID:-}" ]] && kill "$UPSTREAM2_PID" >/dev/null 2>&1 || true
  bash scripts/restore-baseline.sh >/dev/null 2>&1 || true
}
trap cleanup EXIT
mkdir -p target/evidence/agent-rig
cat > /tmp/zk-agent-fake.py <<'PY'
import json,sys
from http.server import BaseHTTPRequestHandler, HTTPServer
UNKNOWN_SEEN = False
def respond(body):
    global UNKNOWN_SEEN
    global UNKNOWN_SEEN
    messages=body.get("messages",[])
    user_indices=[i for i,m in enumerate(messages) if m.get("role")=="user"]
    last_user_index=user_indices[-1] if user_indices else -1
    user=str(messages[last_user_index].get("content") or "") if last_user_index >= 0 else ""
    # Only tools executed in the current turn count; earlier turns are history.
    tool_count=len([m for m in messages[last_user_index+1:] if m.get("role")=="tool"])
    if "unknown" in user:
        if not UNKNOWN_SEEN:
            UNKNOWN_SEEN = True
            call={"id":"call_unknown","type":"function","function":{"name":"unknown_tool","arguments":"{}"}}
            return {"id":"chatcmpl-unknown","object":"chat.completion","choices":[{"index":0,"finish_reason":"tool_calls","message":{"role":"assistant","content":None,"tool_calls":[call]}}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}
        return {"id":"chatcmpl-final","object":"chat.completion","choices":[{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":"unknown handled"}}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}
    if "loop" in user:
        call={"id":"call_loop","type":"function","function":{"name":"fs.read","arguments":json.dumps({"path":"agent-e2e.txt"})}}
        return {"id":"chatcmpl-loop","object":"chat.completion","choices":[{"index":0,"finish_reason":"tool_calls","message":{"role":"assistant","content":None,"tool_calls":[call]}}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}
    if "present" in user:
        if tool_count==0:
            call={"id":"call_pwrite","type":"function","function":{"name":"fs.write","arguments":json.dumps({"path":"present.txt","content":"deliverable"})}}
        elif tool_count==1:
            call={"id":"call_present","type":"function","function":{"name":"present","arguments":json.dumps({"path":"present.txt","description":"result"})}}
        else:
            return {"id":"chatcmpl-final","object":"chat.completion","choices":[{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":"已完成工具调用并通过审批。"}}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}
    elif "todo" in user and tool_count==0:
        call={"id":"call_todo","type":"function","function":{"name":"todo","arguments":json.dumps({"action":"add","text":"ship"})}}
    elif "goal" in user and tool_count==0:
        call={"id":"call_goal","type":"function","function":{"name":"goal","arguments":json.dumps({"action":"create","objective":"ship"})}}
    elif "schedule" in user and tool_count==0:
        call={"id":"call_schedule","type":"function","function":{"name":"schedule","arguments":json.dumps({"action":"create","cron":"0 9 * * *","prompt":"standup"})}}
    elif "skill" in user and tool_count==0:
        call={"id":"call_skill","type":"function","function":{"name":"skill","arguments":json.dumps({"action":"list"})}}
    elif "slow" in user:
        import time; time.sleep(5)
        return {"id":"chatcmpl-slow","object":"chat.completion","choices":[{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":"slow done"}}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}
    if tool_count>0 and "multi" not in user:
        return {"id":"chatcmpl-final","object":"chat.completion","choices":[{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":"已完成工具调用并通过审批。"}}],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}
    if "multi" in user and tool_count>=2:
        return {"id":"chatcmpl-final","object":"chat.completion","choices":[{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":"已完成工具调用并通过审批。"}}],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}
    if "bash" in user:
        call={"id":"call_bash","type":"function","function":{"name":"bash","arguments":json.dumps({"command":"printf hi"})}}
    elif "ask" in user:
        call={"id":"call_ask","type":"function","function":{"name":"ask_user_question","arguments":json.dumps({"questions":[{"id":"name","question":"你的名字？"}]})}}
    elif "multi" in user and tool_count==0:
        call={"id":"call_read","type":"function","function":{"name":"fs.read","arguments":json.dumps({"path":"agent-e2e.txt"})}}
    elif "multi" in user and tool_count==1:
        call={"id":"call_read2","type":"function","function":{"name":"fs.read","arguments":json.dumps({"path":"agent-e2e.txt"})}}
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
cat > /tmp/zk-agent-second.py <<'PY2'
import json,sys
from http.server import BaseHTTPRequestHandler, HTTPServer
class H(BaseHTTPRequestHandler):
    def do_POST(self):
        length=int(self.headers.get("content-length","0")); self.rfile.read(length)
        data=json.dumps({"id":"chatcmpl-second","object":"chat.completion","choices":[{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":"second provider"}}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}).encode()
        self.send_response(200); self.send_header("content-type","application/json"); self.send_header("content-length",str(len(data))); self.end_headers(); self.wfile.write(data)
    def log_message(self,*args): pass
HTTPServer(("127.0.0.1",int(sys.argv[1])),H).serve_forever()
PY2
python3 /tmp/zk-agent-second.py "$UPSTREAM2_PORT" >/dev/null 2>&1 & UPSTREAM2_PID=$!
cargo run -q -p zk -- compose --edition personal --profile dev-live --channel official >/dev/null
cargo build -q -p zk-desktop
pnpm --dir apps/desktop/web build >/dev/null
rm -rf target/evidence/agent-rig/home && mkdir -p target/evidence/agent-rig/home/.agents/skills/demo && printf '# demo\n' > target/evidence/agent-rig/home/.agents/skills/demo/SKILL.md
HOME="$PWD/target/evidence/agent-rig/home" ZEOKEEL_HOME="$PWD/target/evidence/agent-rig/home" ZEOKEEL_SERVE_ADDR="127.0.0.1:$UI_PORT" ZEOKEEL_AGENT_STREAMING=0 \
ZEOKEEL_OPENAI_BASE_URL="http://127.0.0.1:$UPSTREAM_PORT/v1" ZEOKEEL_OPENAI_API_KEY=test ZEOKEEL_OPENAI_MODEL=test-model ZEOKEEL_AGENT_MAX_STEPS=3 \
./target/debug/zk-desktop --serve >/tmp/zk-agent-ui.log 2>&1 & UI_PID=$!
for _ in $(seq 1 80); do curl -fsS "http://127.0.0.1:$UI_PORT/healthz" >/dev/null 2>&1 && break; sleep 0.25; done
ZEOKEEL_AGENT_UI_URL="http://127.0.0.1:$UI_PORT" ZEOKEEL_AGENT_UPSTREAM2="http://127.0.0.1:$UPSTREAM2_PORT/v1" pnpm exec playwright test e2e/agent-rig.spec.ts --config=playwright.agent.config.ts ${ZEOKEEL_AGENT_GREP:+--grep "$ZEOKEEL_AGENT_GREP"}
echo "agent e2e ok"
