#!/usr/bin/env bash
# Offline master.key key-ring rotation drill (docs/21 §3, docs/06 §5).
#
# What it does (no server, no database, offline):
#   1. generates a throwaway versioned key ring in a temp dir:
#        master.key.v1 (0600), master.key.v2 (0600);
#   2. runs the executable ring tests in `crates/zk-kit-server/tests/master_key_ring.rs`:
#      roundtrip, tamper rejection, legacy plaintext read, missing-key fail-closed,
#      version rotation (v1 ciphertext readable with v1+v2; re-encrypt -> v2;
#      v2-only ring cannot read v1), 0600 permission enforcement;
#   3. runs the env-driven drill (`env_ring_drill`) against the generated key files,
#      which proves load-from-dir + rotation end-to-end;
#   4. asserts the generated key material never appears in the evidence log.
#
# Production rotation procedure documented in docs/21 §3: add master.key.v<N+1>,
# restart, update provider keys via admin (re-encrypt with highest version), keep the
# old version until no ciphertext references it, then remove it.
#
# Usage: bash scripts/master-key-rotate.sh
# Exit: 0 drill pass; 1 failure. Evidence: target/evidence/m6/master-key-rotate.json
set -uo pipefail
cd "$(dirname "$0")/.."

ROOT="$(pwd)"
OUT="${ZEOKEEL_MASTER_KEY_DRILL_JSON:-target/evidence/m6/master-key-rotate.json}"
HEAD_SHA="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
WORK="target/evidence/m6/master-key-rotate"
LOG="$WORK/ring-tests.log"
mkdir -p "$(dirname "$OUT")"
rm -rf "$WORK"
mkdir -p "$WORK/keys-both" "$WORK/keys-v1" "$WORK/keys-bad"

gen_key() {
  python3 -c 'import base64, secrets; print(base64.b64encode(secrets.token_bytes(32)).decode())'
}

KEY_V1="$(gen_key)"
KEY_V2="$(gen_key)"
printf '%s' "$KEY_V1" > "$WORK/keys-both/master.key.v1"
printf '%s' "$KEY_V2" > "$WORK/keys-both/master.key.v2"
printf '%s' "$KEY_V1" > "$WORK/keys-v1/master.key.v1"
chmod 700 "$WORK/keys-both" "$WORK/keys-v1" "$WORK/keys-bad"
chmod 600 "$WORK/keys-both/master.key.v1" "$WORK/keys-both/master.key.v2" "$WORK/keys-v1/master.key.v1"
# Deliberately too-open key file: the ring must refuse to load it (0600 required).
printf '%s' "$KEY_V2" > "$WORK/keys-bad/master.key.v1"
chmod 644 "$WORK/keys-bad/master.key.v1"

RUN_RC=0
ZEOKEEL_MASTER_KEY_DRILL_BOTH="$ROOT/$WORK/keys-both" \
ZEOKEEL_MASTER_KEY_DRILL_V1="$ROOT/$WORK/keys-v1" \
ZEOKEEL_MASTER_KEY_DRILL_BAD="$ROOT/$WORK/keys-bad" \
  cargo test -q --offline -p zk-kit-server --test master_key_ring -- --nocapture > "$LOG" 2>&1
RUN_RC=$?

CHECKS="$WORK/checks.tsv"; : > "$CHECKS"
record() { printf '%s\t%s\t%s\n' "$1" "$2" "$(printf '%s' "$3" | tr '\t\n' '  ')" >> "$CHECKS"; }

if [[ "$RUN_RC" -eq 0 ]]; then
  record ring-tests pass "cargo test -p zk-kit-server --test master_key_ring exit 0"
else
  record ring-tests fail "ring tests exit $RUN_RC: $(tail -n 5 "$LOG" | tr '\n' ' ')"
fi
if grep -q 'master-key env ring drill: OK' "$LOG"; then
  record env-drill pass "load_from_dir + v1->v2 rotation drill executed against generated key files"
else
  record env-drill fail "env ring drill marker missing (drill did not run)"
fi
if grep -q 'test result: ok' "$LOG" && ! grep -q 'FAILED' "$LOG"; then
  record test-results pass "all ring test cases passed"
else
  record test-results fail "test failures present: $(grep -E 'FAILED|panicked' "$LOG" | head -3 | tr '\n' ' ')"
fi
if [[ "$(stat -c '%a' "$WORK/keys-both/master.key.v1" 2>/dev/null)" == "600" && "$(stat -c '%a' "$WORK/keys-both/master.key.v2" 2>/dev/null)" == "600" ]]; then
  record key-file-mode pass "master.key.v1/v2 are 0600"
else
  record key-file-mode fail "key file mode is not 0600"
fi
if grep -q 'master-key 0644 key refused: OK' "$LOG"; then
  record open-mode-refused pass "0644 key file refused (KeyError::Permissions)"
else
  record open-mode-refused fail "ring did not reject a group/world-readable key file"
fi
if grep -Fq "$KEY_V1" "$LOG" || grep -Fq "$KEY_V2" "$LOG"; then
  record no-key-material-in-log fail "generated key material leaked into the evidence log"
else
  record no-key-material-in-log pass "no key material in the drill log (only ciphertext/metadata)"
fi

STATUS="pass"
if grep -q $'\tfail\t' "$CHECKS"; then STATUS="fail"; fi
python3 - "$CHECKS" "$OUT" "$WORK" "$STATUS" "$HEAD_SHA" <<'PY'
import json
import pathlib
import sys
from datetime import datetime, timezone

checks_path, out_path, work, status, head = sys.argv[1:6]
checks = []
with open(checks_path, encoding="utf-8") as handle:
    for line in handle:
        line = line.rstrip("\n")
        if not line:
            continue
        parts = (line.split("\t") + ["", ""])[:3]
        checks.append({"id": parts[0], "status": parts[1], "detail": parts[2]})
payload = {
    "schema": 1,
    "drill": "master-key-rotate",
    "status": status,
    "head": head,
    "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "checks": checks,
    "command": "bash scripts/master-key-rotate.sh",
    "evidence": str(work),
    "production_rotation": "docs/21 §3 (add master.key.v<N+1>, restart, re-encrypt via admin update, retire old version)",
}
pathlib.Path(out_path).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
print(f"master-key-rotate: {status} ({sum(1 for c in checks if c['status']=='pass')}/{len(checks)} checks) -> {out_path}")
PY
if [[ "$STATUS" == "pass" ]]; then exit 0; else exit 1; fi
