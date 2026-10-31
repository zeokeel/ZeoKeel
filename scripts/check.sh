#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

cargo run -q -p zk -- validate
cargo run -q -p zk -- compose --edition personal --profile default --channel official --check
cargo run -q -p zk -- capability --check
cargo run -q -p zk -- lint --duplicates
cargo check --workspace
python3 scripts/agent-tool-matrix.py --check
python3 scripts/docs-command-check.py
python3 scripts/docs-check.py
python3 scripts/spec-check.py
python3 scripts/review-check.py
pnpm --dir apps/desktop/web build
pnpm --dir apps/desktop/web test
echo "check: ok"
