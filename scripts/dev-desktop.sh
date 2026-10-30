#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# Headless CI/containers: use xvfb-run; normal desktops can run directly.
if [[ -n "${DISPLAY:-}" ]]; then
  exec cargo run -p zk-desktop
fi
if command -v xvfb-run >/dev/null 2>&1; then
  exec xvfb-run -a cargo run -p zk-desktop
fi
echo "no DISPLAY and no xvfb-run; run from a desktop session" >&2
exit 1
