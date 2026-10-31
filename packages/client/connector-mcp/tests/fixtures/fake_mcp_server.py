#!/usr/bin/env python3
"""Fake MCP stdio server for zk-connector-mcp tests (no network).

Modes: basic (default) | countfile | malformed | exit | timeout
- countfile <path>: counts `initialize` requests in <path> so tests can prove the
  client reuses one session (initialize once) and re-initializes after reload/config change.
Speaks newline-delimited JSON-RPC 2.0 over stdin/stdout.
"""
import json
import sys
import time

MODE = sys.argv[1] if len(sys.argv) > 1 else "basic"
COUNTFILE = sys.argv[2] if len(sys.argv) > 2 else ""


def send(payload):
    sys.stdout.write(json.dumps(payload) + "\n")
    sys.stdout.flush()


def bump_initialize_count():
    if not COUNTFILE:
        return
    try:
        with open(COUNTFILE, "r", encoding="utf-8") as handle:
            current = int(handle.read().strip() or "0")
    except (OSError, ValueError):
        current = 0
    with open(COUNTFILE, "w", encoding="utf-8") as handle:
        handle.write(str(current + 1))


for raw in sys.stdin:
    raw = raw.strip()
    if not raw:
        continue
    try:
        message = json.loads(raw)
    except Exception:
        continue
    if "id" not in message:
        continue  # notification (e.g. notifications/initialized)
    if MODE == "malformed":
        sys.stdout.write("this is not json-rpc\n")
        sys.stdout.flush()
        continue
    if MODE == "exit":
        sys.exit(3)
    if MODE == "timeout":
        time.sleep(10)
        continue
    method = message.get("method")
    params = message.get("params") or {}
    if method == "initialize":
        bump_initialize_count()
        send({
            "jsonrpc": "2.0",
            "id": message["id"],
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "fake-mcp", "version": "0.0.1"},
            },
        })
    elif method == "tools/list":
        send({
            "jsonrpc": "2.0",
            "id": message["id"],
            "result": {
                "tools": [{
                    "name": "echo",
                    "description": "Echo the arguments back",
                    "inputSchema": {
                        "type": "object",
                        "properties": {"text": {"type": "string"}},
                        "required": ["text"],
                    },
                }],
            },
        })
    elif method == "tools/call":
        name = params.get("name")
        arguments = params.get("arguments") or {}
        send({
            "jsonrpc": "2.0",
            "id": message["id"],
            "result": {
                "content": [{
                    "type": "text",
                    "text": json.dumps({"tool": name, "args": arguments}, sort_keys=True),
                }],
                "isError": False,
            },
        })
    else:
        send({
            "jsonrpc": "2.0",
            "id": message["id"],
            "error": {"code": -32601, "message": "method not found"},
        })
