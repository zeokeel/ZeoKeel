#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

cargo run -q -p zk -- validate
cargo run -q -p zk -- compose --edition personal --profile default --channel official --check
cargo run -q -p zk -- capability --check
cargo check --workspace
python3 scripts/docs-check.py
python3 scripts/spec-check.py
echo "check: ok"
