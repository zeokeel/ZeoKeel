#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

cargo test --workspace
cargo run -q -p zk -- validate
cargo run -q -p zk -- compose --edition personal --profile default --channel official --check
cargo run -q -p zk -- capability --check
python3 scripts/docs-check.py
python3 scripts/spec-check.py
echo "test: ok"
