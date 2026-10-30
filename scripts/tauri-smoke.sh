#!/usr/bin/env bash
# M4 Tauri runtime smoke: Xvfb + real desktop binary; asserts the main window exists.
set -euo pipefail
DISPLAY_NUM="${ZEOKEEL_SMOKE_DISPLAY:-:99}"
Xvfb "$DISPLAY_NUM" -screen 0 1280x800x24 >/tmp/zk-xvfb.log 2>&1 & XVFB_PID=$!
cleanup() { [[ -n "${APP_PID:-}" ]] && kill "$APP_PID" >/dev/null 2>&1 || true; kill "$XVFB_PID" >/dev/null 2>&1 || true; }
trap cleanup EXIT
sleep 1
DISPLAY="$DISPLAY_NUM" ZEOKEEL_HOME=/tmp/zk-tauri-smoke timeout 15 ./target/debug/zk-desktop >/tmp/zk-tauri-smoke.log 2>&1 & APP_PID=$!
mkdir -p target/evidence/m4
for _ in $(seq 1 20); do
  if DISPLAY="$DISPLAY_NUM" xdotool search --name 'ZeoKeel' >/dev/null 2>&1; then
    DISPLAY="$DISPLAY_NUM" import -window root target/evidence/m4/tauri-main.png
    echo "tauri smoke ok: main window"
    exit 0
  fi
  sleep 0.5
done
echo "tauri smoke failed: no window" >&2
cat /tmp/zk-tauri-smoke.log >&2
exit 1
