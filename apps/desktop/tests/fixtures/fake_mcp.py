#!/usr/bin/env python3
"""Minimal newline-delimited MCP (JSON-RPC 2.0) server for desktop tests."""
import json, sys, time

MODE = sys.argv[1] if len(sys.argv) > 1 else "basic"

for raw in sys.stdin:
    raw = raw.strip()
    if not raw:
        continue
    message = json.loads(raw)
    if "id" not in message:
        continue
    if MODE == "timeout":
        time.sleep(30)
        continue
    method = message.get("method")
    if method == "initialize":
        result = {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "serverInfo": {"name": "fake-mcp", "version": "0.0.1"}}
    elif method == "tools/list":
        result = {"tools": [{"name": "echo", "description": "echo", "inputSchema": {"type": "object"}}]}
    else:
        result = {}
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": message["id"], "result": result}) + "\n")
    sys.stdout.flush()
