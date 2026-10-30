#!/usr/bin/env bash
# M5 OEM/delivery: channel schema, OEM index/pin, compose matrix, dist verify.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -q -p zk
./target/debug/zk channel validate --index oem/fixtures --all
./target/debug/zk channel verify --channel oem-demo --dist oem/fixtures/dist/oem-demo
./target/debug/zk compose --edition enterprise --profile default --channel oem-demo >/dev/null
./target/debug/zk compose --edition enterprise --profile default --channel oem-demo --check >/dev/null
grep -q 'Acme Keel' apps/desktop/web/src/generated/brand.ts
grep -q 'oem-demo' apps/desktop/web/src/generated/brand.ts
# residue negative: a product string must fail hide verification
cp -r oem/fixtures/dist/oem-demo /tmp/zk-oem-loud
printf 'ZeoKeel\n' > /tmp/zk-oem-loud/leak.txt
printf '\000ZeoKeel\000' > /tmp/zk-oem-loud/leak.bin
if ./target/debug/zk channel verify --channel oem-demo --dist /tmp/zk-oem-loud >/dev/null 2>&1; then
  echo "residue check failed to reject" >&2; exit 1
fi
rm -rf /tmp/zk-oem-loud
# restore official generated artifacts
./target/debug/zk compose --edition enterprise --profile default --channel official >/dev/null
echo "oem e2e ok: channel/index/pin/compose/brand/residue"
