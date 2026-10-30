#!/usr/bin/env bash
# P2 原型：构建 host + 两个 guest，并跑 golden conformance。
set -euo pipefail
cd "$(dirname "$0")"

export PATH="/usr/local/go/bin:${PATH}"

echo "[1/4] host (Rust + wasmtime embedding)"
cargo build --release

echo "[2/4] guest-rust (wasm32-wasip1)"
rustup target add wasm32-wasip1 >/dev/null 2>&1 || true
(cd guest-rust && cargo build --release --target wasm32-wasip1)

echo "[3/4] guest-go (wasip1)"
(cd guest-go && GOOS=wasip1 GOARCH=wasm go build -ldflags="-s -w" -o guest-go.wasm .)

echo "[4/4] golden conformance"
python3 golden/run.py
