#!/usr/bin/env bash
# Derive the desktop app icon from the channel's authoritative logo.
# Usage: scripts/gen-icons.sh [channel]   (default: official)
# Deterministic: same input SVG + rsvg-convert version -> same 1024 RGBA PNG.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CHANNEL="${1:-official}"
LOGO="$ROOT/brands/$CHANNEL/logo.svg"
OUT="$ROOT/apps/desktop/icons/icon.png"
TMP="$(mktemp "${TMPDIR:-/tmp}/zeokeel-icon.XXXXXX.png")"
trap 'rm -f "$TMP"' EXIT
[[ -f "$LOGO" ]] || { echo "gen-icons: missing $LOGO" >&2; exit 1; }
command -v rsvg-convert >/dev/null || { echo "gen-icons: install librsvg2-bin" >&2; exit 1; }
rsvg-convert -w 1024 -h 1024 "$LOGO" -o "$TMP"
file "$TMP" | grep -q 'PNG image data, 1024 x 1024' || { echo "gen-icons: unexpected output" >&2; exit 1; }
mv "$TMP" "$OUT"
echo "gen-icons: wrote ${OUT#$ROOT/} from ${LOGO#$ROOT/}"
