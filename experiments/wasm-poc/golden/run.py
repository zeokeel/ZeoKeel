#!/usr/bin/env python3
"""P2 golden conformance: Rust/Go guests must produce identical frame semantics."""
import json, os, subprocess, sys, tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HOST = os.path.join(ROOT, "target/release/zk-wasm-host")
GUESTS = {
    "rust": os.path.join(ROOT, "guest-rust/target/wasm32-wasip1/release/zk-guest-rust.wasm"),
    "go": os.path.join(ROOT, "guest-go/guest-go.wasm"),
}

def run(guest, req, db, fuel=None):
    cmd = [HOST, "--wasm", GUESTS[guest], "--db", db, "--request", json.dumps(req)]
    if fuel:
        cmd += ["--fuel", str(fuel)]
    p = subprocess.run(cmd, capture_output=True, text=True)
    out = p.stdout.strip()
    try:
        body = json.loads(out) if out else {}
    except json.JSONDecodeError:
        body = {"host_error": "non-json", "raw": out}
    return p.returncode, body

def main():
    tmp = tempfile.mkdtemp(prefix="zk-wasm-")
    db = os.path.join(tmp, "app.db")
    failures = []
    for name in GUESTS:
        rc, body = run(name, {"body": {"op": "add", "name": name}}, db)
        if rc != 0 or body.get("status") != 200:
            failures.append(f"{name} add failed: rc={rc} body={body}")
    rc, body = run("go", {"body": {"op": "list"}}, db)
    rows = body.get("body", {}).get("rows", [])
    names = [r.get("name") for r in rows]
    if names != ["rust", "go"]:
        failures.append(f"list mismatch: {names}")
    rc, body = run("go", {"body": {"op": "list"}}, db, fuel=1000)
    if rc == 0:
        failures.append("fuel limit did not trap")
    print(json.dumps({"ok": not failures, "failures": failures, "rows": rows}, ensure_ascii=False))
    sys.exit(0 if not failures else 1)

if __name__ == "__main__":
    main()
