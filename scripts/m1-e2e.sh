#!/usr/bin/env bash
# M0/M1 deterministic e2e: compose the offline mock profile, run Playwright, restore baseline.
set -euo pipefail
cd "$(dirname "$0")/.."
cleanup() { bash scripts/restore-baseline.sh >/dev/null 2>&1 || true; }
trap cleanup EXIT
cargo run -q -p zk -- compose --edition personal --profile mock --channel official >/dev/null
pnpm exec playwright test "$@"
