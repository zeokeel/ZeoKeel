#!/usr/bin/env bash
# M0/M1 deterministic e2e: compose the offline mock profile, run Playwright, restore baseline.
set -euo pipefail
cd "$(dirname "$0")/.."
cleanup() { bash scripts/restore-baseline.sh >/dev/null 2>&1 || true; }
trap cleanup EXIT
cargo run -q -p zk -- compose --edition personal --profile mock --channel official >/dev/null
# Mock profile links agent-loop; the real serve carrier must use the personal baseline
# (agent-rig + usage), so skip it during the mock run and run it after restoring.
ZEOKEEL_E2E_SKIP_SERVE=1 pnpm exec playwright test --grep-invert "serve carrier" "$@"
bash scripts/restore-baseline.sh >/dev/null 2>&1
pnpm exec playwright test e2e/m1-serve.spec.ts e2e/m1-serve-security.spec.ts e2e/m1-a11y.spec.ts e2e/mcp-config.spec.ts "$@"
