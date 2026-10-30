#!/usr/bin/env bash
# Restore the committed personal×default×official generated baseline.
#
# Enterprise/OEM e2e runs compose a different edition into the worktree; docs/24 §1.2
# requires the committed baseline to stay personal. Call this from an EXIT trap so a
# failed e2e never leaves the repository on the enterprise composition.
set -euo pipefail
cd "$(dirname "$0")/.."

git checkout -- \
  Cargo.lock \
  crates/zk-composed/Cargo.toml crates/zk-composed/src/lib.rs \
  apps/desktop/Cargo.toml apps/server/Cargo.toml \
  apps/desktop/src/generated apps/server/src/generated \
  apps/desktop/web/src/generated apps/admin/web/src/generated \
  apps/desktop/web/public/brand \
  shared/contracts/brand.json shared/contracts/packages.json \
  shared/contracts/roles.json shared/contracts/ui-contributions.json \
  docs/generated/PACKAGE-INDEX.md

cargo run -q -p zk -- compose --edition personal --profile default --channel official --check >/dev/null
echo "baseline restored: personal×default×official"
