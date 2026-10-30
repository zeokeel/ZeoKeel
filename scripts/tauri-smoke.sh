#!/usr/bin/env bash
# M4 Tauri runtime smoke: Xvfb + real desktop binary; asserts the main window exists.
set -euo pipefail
DISPLAY_NUM="${ZEOKEEL_SMOKE_DISPLAY:-:99}"
Xvfb "$DISPLAY_NUM" -screen 0 1280x800x24 >/tmp/zk-xvfb.log 2>&1 & XVFB_PID=$!
cleanup() { [[ -n "${APP_PID:-}" ]] && kill "$APP_PID" >/dev/null 2>&1 || true; kill "$XVFB_PID" >/dev/null 2>&1 || true; }
trap cleanup EXIT
sleep 1
rm -rf /tmp/zk-tauri-smoke; mkdir -p /tmp/zk-tauri-smoke
DISPLAY="$DISPLAY_NUM" WEBKIT_DISABLE_COMPOSITING_MODE=1 LIBGL_ALWAYS_SOFTWARE=1 ZEOKEEL_HOME=/tmp/zk-tauri-smoke ZEOKEEL_OPEN_APP_ID=smoke-app ZEOKEEL_APP_SELFTEST=1 timeout 15 ./target/debug/zk-desktop >/tmp/zk-tauri-smoke.log 2>&1 & APP_PID=$!
mkdir -p target/evidence/m4
for _ in $(seq 1 20); do
  DISPLAY="$DISPLAY_NUM" xdotool search --name 'ZeoKeel' getwindowname %@ 2>/dev/null | sed 's/^/window: /' || true
  WID=$(DISPLAY="$DISPLAY_NUM" xdotool search --name 'SELFTEST' 2>/dev/null | head -1 || true)
  if [[ -n "$WID" ]]; then
    TITLE=$(DISPLAY="$DISPLAY_NUM" xdotool getwindowname "$WID")
    printf '%s\n' "$TITLE" > target/evidence/m4/tauri-app-selftest.txt
    [[ "$TITLE" == *"APP_FORBIDDEN"* ]] || { echo "selftest missing ipc forbidden: $TITLE" >&2; exit 1; }
    DISPLAY="$DISPLAY_NUM" import -window "$WID" target/evidence/m4/tauri-app-window.png || true
    echo "tauri smoke ok: $TITLE"
    exit 0
  fi
  sleep 0.5
done
echo "tauri smoke failed: no window" >&2
cat /tmp/zk-tauri-smoke.log >&2
exit 1
