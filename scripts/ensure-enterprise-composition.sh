#!/usr/bin/env bash
# Ensure the worktree is composed for the ENTERPRISE server before building/running
# `zk-server` / admin assets (A18 target trimming).
#
# A18 makes `edition = personal` (the committed baseline, `targets = ["desktop"]`)
# generate a stub `apps/server/src/main.rs` (prints "server target is not part of the
# personal composition" and exits 2) with zero srv-* dependencies. Any script or CI job
# that builds or runs the real server must therefore:
#   1. compose enterprise×default×official first (this script), and
#   2. restore the personal baseline afterwards when it mutated generated files
#      (`scripts/restore-baseline.sh`) — CI jobs are ephemeral and may skip step 2.
#
# Prints one token on stdout so callers know whether they must restore:
#   already-enterprise   no change (e.g. CI job already composed enterprise)
#   composed-enterprise  caller composed enterprise and must restore the baseline
# Exit 2 when the composition cannot be produced.
set -euo pipefail
cd "$(dirname "$0")/.."

edition="$(
  sed -n 's/^pub const EDITION: &str = "\([a-z]*\)";/\1/p' crates/zk-composed/src/lib.rs 2>/dev/null | head -1
)"
if [[ "$edition" == "enterprise" ]]; then
  echo "already-enterprise"
  exit 0
fi

cargo run -q -p zk -- compose --edition enterprise --profile default --channel official >/dev/null
echo "composed-enterprise"
