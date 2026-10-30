#!/usr/bin/env bash
# M6 reproducible build: two clean builds of zk must match; larger artifacts UNVERIFIED locally.
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=target/evidence/m6/reproducible.json; mkdir -p target/evidence/m6
export SOURCE_DATE_EPOCH=0
build() { cargo clean -q -p zk; cargo build -q -p zk --offline; sha256sum target/debug/zk | cut -d' ' -f1; }
A=$(build); cp target/debug/zk /tmp/zk-repro-a
B=$(build); cp target/debug/zk /tmp/zk-repro-b
STATUS=pass; [[ "$A" == "$B" ]] || STATUS=fail
[[ "$STATUS" == pass ]] && STATUS=unverified
python3 - "$A" "$B" "$STATUS" "$OUT" <<'PY'
import json,sys,pathlib
pathlib.Path(sys.argv[4]).write_text(json.dumps({"artifact":"target/debug/zk","sha_a":sys.argv[1],"sha_b":sys.argv[2],"status":sys.argv[3],"unverified":["zk-server","zk-desktop","admin dist","image tar"],"source_date_epoch":0},indent=2)+"\n")
print(f"reproducible: {sys.argv[3]} {sys.argv[1][:12]} / {sys.argv[2][:12]}")
PY

[[ "$STATUS" == fail ]] && exit 1
[[ "$STATUS" == unverified ]] && exit 2
