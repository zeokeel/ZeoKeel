#!/usr/bin/env bash
# One-shot local bootstrap for the M0 scaffold demo.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build -p zk
cargo run -q -p zk -- validate
cargo run -q -p zk -- compose --edition personal --profile default --channel official
cargo run -q -p zk -- capability
cargo test --workspace

echo "bootstrap: ok"
echo "  desktop demo: ./scripts/dev-desktop.sh"
echo "  server demo : ./scripts/dev-server.sh"
