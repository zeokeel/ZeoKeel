#!/usr/bin/env bash
# M6 reproducible-build drill (hardening spec §4, docs/37).
#
# Compares two builds of the same artifact from the same tree (incremental-cold rebuild
# of the target crate: `cargo clean -p <crate>` between builds) for:
#   - zk         (tools/zk)      -> target/debug/zk
#   - zk-server  (apps/server)   -> target/debug/zk-server
#   - zk-desktop (apps/desktop)  -> target/debug/zk-desktop
#   - admin dist (apps/admin/web) -> build twice, compare sorted asset hashes
#
# Honest UNVERIFIED items (recorded, never fake-passed; exit 2):
#   - server image tar reproducibility (no Docker/image build on this host)
#   - the strict flag set from hardening spec §4 is NOT applied in this local run
#     (--remap-path-prefix / -C build-id=none / SOURCE_DATE_EPOCH would force a full
#     dependency rebuild on a ~13G disk); CI cold-build path owns that check.
# Per-artifact match is still `pass`/`fail`; overall status is `unverified` locally.
#
# Usage: bash scripts/reproducible-build.sh [--out <json>]
# Exit: 0 all comparable artifacts match and nothing unverified; 2 comparable match + UNVERIFIED; 1 mismatch/build failure.
set -uo pipefail
cd "$(dirname "$0")/.."

OUT="${ZEOKEEL_REPRO_JSON:-target/evidence/m6/reproducible.json}"
WORK="target/evidence/m6/reproducible"
mkdir -p "$(dirname "$OUT")"; rm -rf "$WORK"; mkdir -p "$WORK"
export SOURCE_DATE_EPOCH=0
CHECKS="$WORK/checks.tsv"; : > "$CHECKS"
RESULT="pass"

record() { # artifact status detail
  printf '%s\t%s\t%s\n' "$1" "$2" "$(printf '%s' "$3" | tr '\t\n' '  ')" >> "$CHECKS"
  printf '  [%s] %s%s\n' "$2" "$1" "${3:+ — $3}"
  [[ "$2" == "fail" ]] && RESULT="fail"
}

sha256_file() { sha256sum "$1" | cut -d' ' -f1; }

repro_bin() { # package bin_name
  local pkg="$1" bin="$2" path="target/debug/$2" a b rc
  echo "reproducible-build: $bin ($pkg)"
  if [[ ! -x "$path" ]]; then cargo build -q --offline -p "$pkg" || { record "$bin" fail "initial build failed"; return 1; }; fi
  cargo clean -q -p "$pkg" 2>/dev/null || true
  if ! cargo build -q --offline -p "$pkg" > "$WORK/$bin-a.log" 2>&1; then record "$bin" fail "build A failed: $(tail -3 "$WORK/$bin-a.log" | tr '\n' ' ')"; return 1; fi
  [[ -x "$path" ]] || { record "$bin" fail "missing $path after build A"; return 1; }
  a="$(sha256_file "$path")"; cp "$path" "$WORK/$bin-a"
  cargo clean -q -p "$pkg" 2>/dev/null || true
  if ! cargo build -q --offline -p "$pkg" > "$WORK/$bin-b.log" 2>&1; then record "$bin" fail "build B failed: $(tail -3 "$WORK/$bin-b.log" | tr '\n' ' ')"; return 1; fi
  b="$(sha256_file "$path")"; cp "$path" "$WORK/$bin-b"
  if [[ "$a" == "$b" ]]; then
    record "$bin" pass "sha256 ${a:0:16}… identical across two clean builds"
  else
    record "$bin" fail "sha mismatch: ${a:0:16}… != ${b:0:16}…"
  fi
}

repro_bin zk zk
repro_bin zk-server zk-server
repro_bin zk-desktop zk-desktop

# ---------------------------------------------------------------- admin dist
repro_dist() {
  local dir="apps/admin/web" snapshot_a="$WORK/admin-dist-a.txt" snapshot_b="$WORK/admin-dist-b.txt"
  echo "reproducible-build: admin dist"
  if ! pnpm --dir "$dir" build > "$WORK/admin-build-a.log" 2>&1; then
    record "admin-dist" unverified "admin dist build failed (pnpm/toolchain): $(tail -3 "$WORK/admin-build-a.log" | tr '\n' ' ')"; return
  fi
  ( cd "$dir/dist" && find . -type f | sort | while IFS= read -r f; do printf '%s %s\n' "$(sha256sum "$f" | cut -d' ' -f1)" "$f"; done ) > "$snapshot_a"
  rm -rf "$dir/dist"
  if ! pnpm --dir "$dir" build > "$WORK/admin-build-b.log" 2>&1; then
    record "admin-dist" unverified "second admin dist build failed: $(tail -3 "$WORK/admin-build-b.log" | tr '\n' ' ')"; return
  fi
  ( cd "$dir/dist" && find . -type f | sort | while IFS= read -r f; do printf '%s %s\n' "$(sha256sum "$f" | cut -d' ' -f1)" "$f"; done ) > "$snapshot_b"
  if cmp -s "$snapshot_a" "$snapshot_b"; then
    record "admin-dist" pass "$(wc -l < "$snapshot_a") built files byte-identical across two builds"
  else
    record "admin-dist" fail "dist file hashes differ between builds ($(diff "$snapshot_a" "$snapshot_b" | head -5 | tr '\n' ' '))"
  fi
}
repro_dist

# ---------------------------------------------------------------- JSON + exit
UNVERIFIED="image-tar,strict-flag-set"
python3 - "$CHECKS" "$OUT" "$WORK" <<'PY'
import json
import pathlib
import sys

checks_path, out_path, work = sys.argv[1:4]
checks = []
with open(checks_path, encoding="utf-8") as handle:
    for line in handle:
        line = line.rstrip("\n")
        if not line:
            continue
        parts = (line.split("\t") + ["", ""])[:3]
        checks.append({"artifact": parts[0], "status": parts[1], "detail": parts[2]})
# unverified items are not artifacts: add them explicitly so status can never be "pass".
checks.append({"artifact": "server-image-tar", "status": "unverified", "detail": "no Docker/image build on this host; image tar reproducibility not compared"})
checks.append({"artifact": "strict-flag-set", "status": "unverified", "detail": "local run does not apply --remap-path-prefix / -C build-id=none / SOURCE_DATE_EPOCH (would force full dependency rebuild); CI cold-build path owns it"})
if any(check["status"] == "fail" for check in checks):
    status = "fail"
elif any(check["status"] == "unverified" for check in checks):
    status = "unverified"
elif all(check["status"] == "pass" for check in checks):
    status = "pass"
else:
    status = "unverified"
payload = {"schema": 1, "status": status, "checks": checks,
           "unverified": [check["artifact"] for check in checks if check["status"] == "unverified"],
           "evidence_dir": str(work), "source_date_epoch": 0}
pathlib.Path(out_path).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
print(f"reproducible-build: {status} ({sum(1 for c in checks if c['status']=='pass')} pass, "
      f"{sum(1 for c in checks if c['status']=='unverified')} unverified) -> {out_path}")
PY
STATUS="$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["status"])' "$OUT")"
case "$STATUS" in
  pass) exit 0 ;;
  unverified) exit 2 ;;
  *) exit 1 ;;
esac
