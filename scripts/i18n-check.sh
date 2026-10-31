#!/usr/bin/env bash
# B13 i18n gate: fail on hardcoded Han copy outside the i18n dictionaries.
#
# Scope: client + admin UI sources. Both dictionaries are typed
# (`Record<MessageKey, string>`), so a missing/extra key already fails tsc; this
# gate covers what types cannot: literal Chinese copy pasted into components.
#
# Allowed (documented) exceptions:
# - `**/i18n.ts` / `**/i18n.tsx` — the dictionaries themselves;
# - comment lines (docs cross-references such as `docs/18 §5` in code comments);
# - `apps/desktop/web/src/transport/mock.ts` — dev-only `?mock=1` fixture copy
#   (no backend, never shipped as product UI text; e2e asserts its fixtures);
# also checks that both portals expose zh + en.
set -euo pipefail
cd "$(dirname "$0")/.."

fail=0

scan() {
  local label="$1"; shift
  local raw filtered
  raw="$(rg -n '[一-龥]' -g '*.ts' -g '*.tsx' -g '!**/node_modules/**' -g '!**/*.d.ts' \
    -g '!**/i18n.ts' -g '!**/i18n.tsx' "$@" || true)"
  filtered="$(printf '%s\n' "$raw" \
    | rg -v ':[0-9]+:[[:space:]]*(//|/\*|\*)' \
    | rg -v 'apps/desktop/web/src/transport/mock\.ts:' || true)"
  filtered="${filtered//$'\n'/}"
  if [[ -n "$filtered" ]]; then
    echo "i18n-check: hardcoded Han copy in $label (move it into the i18n dictionary):"
    printf '%s\n' "$raw" | rg -v ':[0-9]+:[[:space:]]*(//|/\*|\*)' | rg -v 'apps/desktop/web/src/transport/mock\.ts:'
    fail=1
  fi
}

scan "client UI packages" packages/*/*/web/src apps/*/web/src
scan "admin UI packages" packages/admin/*/web/src apps/admin/web/src
scan "server admin SPA" apps/server/web/src 2>/dev/null || true

# Dictionary parity is type-enforced; state it so the gate is self-documenting.
if ! rg -q 'Record<MessageKey, string>' apps/desktop/web/src/i18n.tsx; then
  echo "i18n-check: en-US dictionary is no longer typed as Record<MessageKey, string>"
  fail=1
fi
if ! rg -q 'Record<AdminMessageKey, string>' packages/admin/ui-kit/web/src/i18n.ts; then
  echo "i18n-check: admin en-US dictionary is no longer typed as Record<AdminMessageKey, string>"
  fail=1
fi

# Portal must offer zh + en (R9.4: portal at least zh+en).
if ! rg -q 'lang="en"' deploy/portal/index.html || ! rg -q 'lang="zh-CN"' deploy/portal/index.html; then
  echo "i18n-check: deploy/portal/index.html must contain both zh-CN and en sections"
  fail=1
fi
if ! rg -q '"/portal/en"' packages/server/updates/src/lib.rs; then
  echo "i18n-check: runtime portal is missing the additive /portal/en route"
  fail=1
fi

if [[ "$fail" -ne 0 ]]; then
  echo "i18n-check: FAILED"
  exit 1
fi
echo "i18n-check ok"
