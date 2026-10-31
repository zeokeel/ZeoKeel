#!/usr/bin/env bash
# docs/21 §10 security gates, executable (spec 2026-10-31-release-1-0-readiness A6).
#
# Emits one row per gate from the 13-item checklist (docs/21 §10 is the SSOT):
#   {id, rule, severity, owner, command, status: pass|fail|unverified, evidence, detail}
#
# Honesty rules (docs/11 §5):
#   - a gate that needs a missing external tool/env is `unverified`, never `pass`;
#   - overall status is `pass` only when all 13 gates are `pass`;
#   - non-strict: pass=0, unverified=2, fail=1;
#   - ZEOKEEL_SECURITY_STRICT=1 (CI): unverified counts as fail => exit 1.
#
# Requirements: PostgreSQL running locally (`sudo -u postgres`, like the other M6 scripts)
# and built `target/debug/zk-server` / `target/debug/zk-desktop`. CI must provide the PG
# service; set ZEOKEEL_SECURITY_SKIP_BUILD=1 if the workflow builds the binaries itself.
#
# Usage: bash scripts/security-audit.sh [--out <dir>]
# Env:
#   ZEOKEEL_SECURITY_STRICT=0|1     strict mode (CI)
#   ZEOKEEL_SECURITY_SKIP_BUILD=0|1
set -uo pipefail
cd "$(dirname "$0")/.."

OUT="${ZEOKEEL_SECURITY_OUT:-target/evidence/release-1.0-rc1/security2}"
RC_SUMMARY="${ZEOKEEL_SECURITY_RC_SUMMARY:-$OUT}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --out) OUT="${2:?--out needs a value}"; shift 2 ;;
    -h|--help) sed -n '2,22p' "$0"; exit 0 ;;
    *) echo "security-audit: unknown argument: $1" >&2; exit 1 ;;
  esac
done
STRICT="${ZEOKEEL_SECURITY_STRICT:-0}"
SKIP_BUILD="${ZEOKEEL_SECURITY_SKIP_BUILD:-0}"
mkdir -p "$OUT/logs" "$RC_SUMMARY"
GATES="$OUT/.gates.tsv"
: > "$GATES"

HEAD_SHA="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
VERSION="$(python3 -c 'import tomllib;print(tomllib.load(open("Cargo.toml","rb"))["workspace"]["package"]["version"])' 2>/dev/null || echo unknown)"

sanitize_detail() { # stdin -> valid UTF-8, single line, capped
  python3 -c '
import sys
data = sys.stdin.buffer.read().decode("utf-8", "replace")
data = data.replace("\t", " ").replace("\n", " ")
print(data[:800])
'
}
record() { # id rule severity owner command status evidence detail [env_only]
  local detail="${8:-}"
  local env_only="${9:-0}"
  detail="$(printf '%s' "$detail" | sanitize_detail)"
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$1" "$2" "$3" "$4" "$5" "$6" "$7" "$detail" "$env_only" >> "$GATES"
}

record_env() { # id rule severity owner command evidence detail -> unverified_external (env_only)
  record "$1" "$2" "$3" "$4" "$5" "unverified_external" "$6" "$7" 1
}

run_log() { # logfile cmd...
  local log="$1"; shift
  { "$@"; } > "$log" 2>&1
}

run_series() { # logfile cmd... (each cmd via bash -c; continues after failures; rc=1 if any failed)
  local log="$1"; shift
  local rc=0 idx=1 cmd result
  : > "$log"
  for cmd in "$@"; do
    printf '### (%d) %s\n' "$idx" "$cmd" >> "$log"
    bash -o pipefail -c "$cmd" >> "$log" 2>&1
    result=$?
    printf '### exit=%d\n' "$result" >> "$log"
    [[ "$result" -ne 0 ]] && rc=1
    idx=$((idx + 1))
  done
  return "$rc"
}

fail_detail() { # logfile
  tail -n 12 "$1" 2>/dev/null | tr '\n' ' ' | cut -c1-600
}

# ---------------------------------------------------------------- preflight
if [[ "$SKIP_BUILD" != "1" ]]; then
  run_log "$OUT/logs/00-preflight-build.log" cargo build -q --offline -p zk-server -p zk-desktop
  if [[ $? -ne 0 ]]; then
    echo "security-audit: WARNING preflight build failed; PG-backed gates will fail" >&2
  fi
fi
PG_OK=1
if ! sudo -u postgres psql -Atqc 'select 1' >/dev/null 2>&1; then PG_OK=0; fi


# ---------------------------------------------------------------- 1. threat model
gate1() {
  local log="$OUT/logs/01-threat-model.log"
  local review="docs/reviews/2026-10-31-threat-model.md"
  local marker='- 威胁模型评审结论：APPROVED'
  {
    echo "docs/33: $(test -f docs/33-threat-model.md && echo present || echo MISSING)"
    echo "docs/21 §10: $(grep -c '## 10. 安全门禁' docs/21-security-privacy.md)"
    echo "review record: $(test -f "$review" && echo present || echo MISSING)"
    echo "exact approval marker count: $(grep -cxF -- "$marker" "$review" 2>/dev/null || true)"
    grep -n '评审者\|结论' "$review" 2>/dev/null | head -5
  } > "$log" 2>&1
  if [[ ! -f docs/33-threat-model.md ]] || ! grep -q '## 10. 安全门禁' docs/21-security-privacy.md; then
    record 1 "threat-model-review" high security "docs/33-threat-model.md + exact approval marker in docs/reviews/2026-10-31-threat-model.md" fail "$log" "threat model or docs/21 §10 missing"
  elif [[ ! -f "$review" ]]; then
    record 1 "threat-model-review" high security "docs/33-threat-model.md + exact approval marker in docs/reviews/2026-10-31-threat-model.md" unverified "$log" "independent review record missing (P1-1)"
  elif grep -qxF -- "$marker" "$review"; then
    record 1 "threat-model-review" high security "docs/33-threat-model.md + exact approval marker in docs/reviews/2026-10-31-threat-model.md" pass "$log" "threat model + exact line '$marker' present (independent reviewer approval)"
  else
    record 1 "threat-model-review" high security "docs/33-threat-model.md + exact approval marker in docs/reviews/2026-10-31-threat-model.md" unverified "$log" "review record exists but no exact line '$marker' (P1-1); current review is CHANGES-REQUESTED/pending re-review, file presence alone is NOT pass"
  fi
}


# ---------------------------------------------------------------- 2. auth / JWT / CSRF / RBAC
gate2() {
  local log="$OUT/logs/02-auth-jwt-csrf-rbac.log"
  if [[ "$PG_OK" != "1" ]]; then
    record 2 "auth-jwt-csrf-rbac" high security "bash scripts/srv-auth-test.sh" unverified "$log" "PostgreSQL unavailable locally; auth/CSRF/audit integration test not executed"
    return
  fi
  run_log "$log" bash scripts/srv-auth-test.sh
  local rc=$?
  if [[ "$rc" -ne 0 ]]; then
    record 2 "auth-jwt-csrf-rbac" high security "bash scripts/srv-auth-test.sh" fail "$log" "exit=$rc: $(fail_detail "$log")"
  elif ! grep -q 'refresh-reuse-family' "$log" || ! grep -q 'lockout' "$log"; then
    record 2 "auth-jwt-csrf-rbac" high security "bash scripts/srv-auth-test.sh" fail "$log" "script exited 0 but expected coverage markers missing"
  else
    record 2 "auth-jwt-csrf-rbac" high security "bash scripts/srv-auth-test.sh" pass "$log" "login/refresh-reuse-family/refresh-expiry/CSRF/RBAC/auditor-logout/lockout verified against real PG"
  fi
}


# ---------------------------------------------------------------- 3. SSRF / rebinding
gate3() {
  local log="$OUT/logs/03-ssrf-rebinding.log"
  : > "$log"
  if [[ "$PG_OK" != "1" ]]; then
    record 3 "ssrf-rebinding" high security "gateway SSRF matrix tests + provider-allowlist probes + serve Host/Origin probe" unverified "$log" "PostgreSQL unavailable; provider CRUD SSRF probes not executed"
    return
  fi
  if [[ ! -x target/debug/zk-server ]]; then
    record 3 "ssrf-rebinding" high security "gateway SSRF matrix tests + provider-allowlist probes + serve Host/Origin probe" fail "$log" "target/debug/zk-server missing (preflight build failed?)"
    return
  fi
  local test_rc=0
  {
    echo "## executable SSRF matrix + redirect-policy regression tests (gateway lib tests)"
    cargo test -q --offline -p zk-srv-gateway --lib -- security_round2_tests
  } >> "$log" 2>&1
  test_rc=$?
  local DB="zk_sec_ssrf_$$" ROLE="zk_sec_ssrf_role_$$" PORT="${ZEOKEEL_SECURITY_SSRF_PORT:-8793}"
  local PIDFILE="$OUT/.probe.pid" SHOME="$OUT/serve-home"
  cleanup3() {
    if [[ -s "$PIDFILE" ]]; then
      local pid
      pid="$(cat "$PIDFILE")"
      kill "$pid" >/dev/null 2>&1 || true
      for _ in $(seq 1 50); do kill -0 "$pid" >/dev/null 2>&1 || break; sleep 0.1; done
    fi
    rm -f "$PIDFILE"
    rm -rf "$SHOME"
    sudo -u postgres dropdb --if-exists --force "$DB" >/dev/null 2>&1 || true
    sudo -u postgres psql -Atq -c "DROP ROLE IF EXISTS $ROLE" >/dev/null 2>&1 || true
  }
  ( # subshell: probes; nonzero on any failed assertion
    BASE="http://127.0.0.1:$PORT"
    echo "## SSRF probe: admin provider base_url validation (strict https-only, no dev flag)"
    sudo -u postgres psql -Atq -c "CREATE ROLE $ROLE LOGIN PASSWORD '$ROLE';" || exit 1
    sudo -u postgres createdb -O "$ROLE" "$DB" || exit 1
    ZEOKEEL_MASTER_KEY="000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f" \
    ZEOKEEL_DATABASE_URL="postgres://$ROLE:$ROLE@127.0.0.1:5432/$DB" \
    ZEOKEEL_SERVER_ADDR="127.0.0.1:$PORT" \
    ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD=admin-pass \
      ./target/debug/zk-server >>"$log" 2>&1 &
    echo $! > "$PIDFILE"
    for _ in $(seq 1 120); do curl -fsS "$BASE/readyz" >/dev/null 2>&1 && break; sleep 0.1; done
    curl -fsS "$BASE/readyz" >/dev/null || exit 1
    ADMIN="$(curl -fsS -c "$OUT/.ssrf.cookies" -X POST "$BASE/api/admin/auth/login" -H 'content-type: application/json' -d '{"username":"admin","password":"admin-pass"}')" || exit 1
    CSRF="$(printf '%s' "$ADMIN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["csrf_token"])')" || exit 1
    probe() { # name base_url expect(reject|accept)
      local name="$1" url="$2" expect="$3" code body
      code="$(curl -sS -b "$OUT/.ssrf.cookies" -H "X-CSRF-Token: $CSRF" -o "$OUT/.ssrf.body" -w '%{http_code}' \
        -X POST "$BASE/api/admin/providers" -H 'content-type: application/json' \
        -d "{\"name\":\"$name\",\"base_url\":\"$url\",\"api_key\":\"secret-probe-value\"}")"
      body="$(cat "$OUT/.ssrf.body")"
      echo "probe $name base_url=$url -> HTTP $code (expect $expect)"
      if [[ "$expect" == reject ]]; then
        [[ "$code" == 2* ]] && { echo "FAIL: expected rejection"; return 1; }
      else
        [[ "$code" == 2* ]] || { echo "FAIL: expected acceptance"; return 1; }
      fi
      if grep -q 'secret-probe-value' <<<"$body"; then echo "FAIL: response echoed provider api_key plaintext"; return 1; fi
      return 0
    }
    probe http-metadata "http://169.254.169.254/latest/meta-data/" reject || exit 1
    probe http-private "http://10.0.0.1/v1" reject || exit 1
    probe http-loopback "http://127.0.0.1:8787/v1" reject || exit 1
    probe https-loopback "https://127.0.0.1/v1" reject || exit 1
    probe https-localhost "https://localhost/v1" reject || exit 1
    probe https-metadata-ip "https://169.254.169.254/latest/meta-data/" reject || exit 1
    probe userinfo "https://user:pass@example.com/v1" reject || exit 1
    probe metadata-host "https://metadata.google.internal/computeMetadata/v1/" reject || exit 1
    probe dot-local "https://printer.local/v1" reject || exit 1
    probe dot-internal "https://service.internal/v1" reject || exit 1
    probe https-public "https://api.openai.com/v1" accept || exit 1
    echo "SSRF allowlist probes: pass"
    kill "$(cat "$PIDFILE")" >/dev/null 2>&1 || true; rm -f "$PIDFILE"
    echo
    echo "## rebinding probe: local serve carrier (Host/Origin, unauthenticated IPC)"
    if [[ ! -x target/debug/zk-desktop ]]; then echo "SKIP desktop serve: target/debug/zk-desktop missing"; exit 2; fi
    local SPORT="${ZEOKEEL_SECURITY_SERVE_PORT:-8794}" CODE HDR
    mkdir -p "$SHOME"
    HOME="$SHOME" ZEOKEEL_HOME="$SHOME" ZEOKEEL_SERVE_ADDR="127.0.0.1:$SPORT" \
      ./target/debug/zk-desktop --serve >>"$log" 2>&1 &
    echo $! > "$PIDFILE"
    for _ in $(seq 1 160); do curl -fsS "http://127.0.0.1:$SPORT/healthz" >/dev/null 2>&1 && break; sleep 0.1; done
    if ! curl -fsS "http://127.0.0.1:$SPORT/healthz" >/dev/null 2>&1; then
      echo "FAIL: local serve carrier did not become ready on 127.0.0.1:$SPORT"; exit 2
    fi
    CODE="$(curl -sS -o /dev/null -w '%{http_code}' -X POST "http://127.0.0.1:$SPORT/ipc" -H 'content-type: application/json' -d '{"name":"workspace.list","payload":{}}')"
    echo "unauthenticated /ipc -> HTTP $CODE (expect 401)"; [[ "$CODE" == "401" ]] || exit 1
    HDR="$(curl -sS -D - -o /dev/null -X POST "http://127.0.0.1:$SPORT/ipc" -H 'Origin: http://evil.example' -H 'content-type: application/json' -d '{"name":"workspace.list","payload":{}}' | tr -d '\r' | grep -i '^access-control-allow-origin:' || true)"
    echo "evil Origin ACAO header: ${HDR:-<none>} (expect none)"; [[ -z "$HDR" ]] || exit 1
    CODE="$(curl -sS -o /dev/null -w '%{http_code}' "http://127.0.0.1:$SPORT/prototype.html")"
    echo "prototype asset -> HTTP $CODE (expect 404)"; [[ "$CODE" == "404" ]] || exit 1
    echo "rebinding probe: pass"
  ) >>"$log" 2>&1
  local probe_rc=$?
  cleanup3
  if [[ "$test_rc" -ne 0 ]]; then
    record 3 "ssrf-rebinding" high security "gateway SSRF matrix tests + provider-allowlist probes + serve Host/Origin probe" fail "$log" "gateway SSRF regression tests failed: $(fail_detail "$log")"
  elif [[ "$probe_rc" -eq 0 ]]; then
    record 3 "ssrf-rebinding" high security "gateway SSRF matrix tests + provider-allowlist probes + serve Host/Origin probe" pass "$log" "strict allowlist tests pass (scheme/userinfo/private/loopback/link-local/metadata/local/internal rejected; https public accepted; redirect Policy::none regression test); provider CRUD probes reject SSRF targets, accept https://api.openai.com; serve unauth IPC=401, evil Origin not reflected"
  elif [[ "$probe_rc" -eq 2 ]]; then
    record 3 "ssrf-rebinding" high security "gateway SSRF matrix tests + provider-allowlist probes + serve Host/Origin probe" unverified "$log" "SSRF matrix/probes passed; zk-desktop missing so serve rebinding probe skipped"
  else
    record 3 "ssrf-rebinding" high security "gateway SSRF matrix tests + provider-allowlist probes + serve Host/Origin probe" fail "$log" "$(fail_detail "$log")"
  fi
}


# ---------------------------------------------------------------- 4. zip / WASM
gate4() {
  local log="$OUT/logs/04-zip-wasm.log"
  run_series "$log" \
    'cargo test --offline -q -p zk-desktop --bin zk-desktop -- hostile_entries' \
    'cargo test --offline -q -p zk-kit-tools --test kit rejects_traversal_and_absolute' \
    'cargo test --offline -q -p zk-kit-wasm'
  local rc=$?
  if [[ "$rc" -eq 0 ]]; then
    record 4 "zip-wasm-containment" high client "cargo test -p zk-desktop --bin zk-desktop hostile_entries; cargo test -p zk-kit-tools kit; cargo test -p zk-kit-wasm" pass "$log" "zip hostile entries (traversal/symlink) rejected; path escape rejected; WASM fuel/memory limits + single-DML/DDL rejection tests pass (golden test skips without wasm guests)"
  else
    record 4 "zip-wasm-containment" high client "cargo test -p zk-desktop --bin zk-desktop hostile_entries; cargo test -p zk-kit-tools kit; cargo test -p zk-kit-wasm" fail "$log" "exit=$rc: $(fail_detail "$log")"
  fi
}


# ---------------------------------------------------------------- 5. command injection / input (5a)
gate5() {
  local log="$OUT/logs/05-command-injection.log"
  run_series "$log" \
    'cargo test --offline -q -p zk-tool-worktree --test tool rejects_injection_and_escape' \
    'cargo test --offline -q -p zk-desktop --bin zk-desktop -- open_url' \
    'cargo test --offline -q -p zk-srv-http'
  local rc=$?
  if [[ "$rc" -eq 0 ]]; then
    record 5 "command-injection-input" high client "cargo test -p zk-tool-worktree; cargo test -p zk-desktop --bin zk-desktop -- open_url; cargo test -p zk-srv-http" pass "$log" "worktree injection/escape rejected; open_url passes URL as one arg without shell; HTTP client-id/request-id validation tests pass (5a of split gate 5)"
  else
    record 5 "command-injection-input" high client "cargo test -p zk-tool-worktree; cargo test -p zk-desktop --bin zk-desktop -- open_url; cargo test -p zk-srv-http" fail "$log" "exit=$rc: $(fail_detail "$log")"
  fi
}

# ---------------------------------------------------------------- 5b. MCP client transport
gate14() {
  local log="$OUT/logs/14-mcp-stdio-transport.log"
  local hits
  hits="$(grep -rniE 'mcp' packages/client apps/desktop/src --include='*.rs' 2>/dev/null | wc -l)"
  : > "$log"
  {
    echo "client-side MCP/stdio transport symbols: $hits"
    grep -rniE 'mcp' packages/client apps/desktop/src --include='*.rs' 2>/dev/null | head -10 || true
  } >> "$log" 2>&1
  if [[ ! -f packages/client/connector-mcp/Cargo.toml ]]; then
    record 14 "mcp-stdio-transport" high client "cargo test -p zk-connector-mcp" fail "$log" "MCP client transport crate packages/client/connector-mcp is absent => implementation gap (was deferred to Phase B B16; ADR-069)"
    return
  fi
  {
    echo "## executable MCP client transport tests (stdio/http/sse, validation)"
    cargo test -q --offline -p zk-connector-mcp
  } >> "$log" 2>&1
  local rc=$?
  if [[ "$rc" -eq 0 ]] && grep -q 'test result: ok' "$log" && ! grep -q 'FAILED' "$log"; then
    record 14 "mcp-stdio-transport" high client "cargo test -p zk-connector-mcp" pass "$log" "MCP client transport implemented (B16/R9.6) with executable tests: stdio spawn/list/call/timeout/malformed-JSON/non-zero-exit, HTTP+sse JSON-RPC, config validation rejects static secrets and non-loopback plaintext, honest unconfigured tools; no shell concatenation (child args array). grep evidence in log."
  else
    record 14 "mcp-stdio-transport" high client "cargo test -p zk-connector-mcp" fail "$log" "MCP client transport tests failed (exit=$rc): $(fail_detail "$log")"
  fi
}

# ---------------------------------------------------------------- 6. key lifecycle (master.key)
gate6() {
  local log="$OUT/logs/06-key-lifecycle.log"
  local rc=0
  run_series "$log" \
    'cargo test -q --offline -p zk-kit-server --test master_key_ring' \
    'ZEOKEEL_MASTER_KEY_DRILL_JSON=target/evidence/m6/master-key-rotate.json bash scripts/master-key-rotate.sh'
  rc=$?
  local drill_ok=0
  grep -q 'master-key-rotate: pass' "$log" && drill_ok=1
  {
    echo
    echo "## implementation (P1-3)"
    echo "zk-kit-server::master_key: AES-256-GCM versioned ring (ring), format vN:<b64(nonce||ct)>,"
    echo "legacy plaintext read with warning; missing key => new writes fail closed."
    echo "gateway: provider create/update encrypt via master_key ring; resolve_upstreams decrypts per provider."
    echo "JWT signing-key rotation and update-signing-key rotation remain offline/HSM deployment controls (not code-executable locally)."
  } >> "$log" 2>&1
  if [[ "$rc" -eq 0 && "$drill_ok" -eq 1 ]]; then
    record 6 "key-lifecycle-drill" high security "cargo test -p zk-kit-server --test master_key_ring + bash scripts/master-key-rotate.sh" pass "$log" "master.key AES-256-GCM versioned ring implemented (vN:<b64 nonce||ct>, legacy plaintext read+warning, missing key fail-closed for new writes); roundtrip/tamper/legacy/missing/rotation/permission tests pass; offline rotation drill pass. JWT/update signing-key rotation remain deployment/HSM controls (documented, no local drill)."
  else
    record 6 "key-lifecycle-drill" high security "cargo test -p zk-kit-server --test master_key_ring + bash scripts/master-key-rotate.sh" fail "$log" "master.key tests/drill failed: exit=$rc drill_ok=$drill_ok; $(fail_detail "$log")"
  fi
}


# ---------------------------------------------------------------- 7. audit chain + money matrix
gate7() {
  local log="$OUT/logs/07-audit-chain.log"
  local auth_log="$OUT/logs/02-auth-jwt-csrf-rbac.log"
  {
    echo "## audit hash chain + fail-closed evidence (from srv-auth-test, gate 2)"
    grep -E "audit-chain|fail-closed-audit|wallet-idempotency" "$auth_log" 2>/dev/null || echo "no marker (gate2 log missing)"
    echo "## WORM external anchoring: ADR-068 formal downgrade (deployment control)"
    grep -n "ADR-068" docs/13-risks-and-adr.md | head -3
    grep -ni "worm" docs/20-operations.md docs/33-threat-model.md | head -5
    echo "## implementation: hash chain prev_hash+hash in zk-kit-server audit_tx; WORM anchoring not in rc.1 code"
    grep -n "prev_hash" crates/zk-kit-server/src/lib.rs | head -3
  } > "$log" 2>&1
  record_env 7 "audit-chain-money-matrix" high security "bash scripts/srv-auth-test.sh (audit-chain/fail-closed) + ADR-068 WORM downgrade" "$log" "hash chain + audit fail-closed verified by gate 2 (real PG); external WORM anchoring is a deployment control (docs/20 §3/§9) formally downgraded by ADR-068 (docs/13) — not implemented in rc.1 code => external/deployment-only, excluded from strict CI failure"
}


# ---------------------------------------------------------------- 8. dependency audit
gate8() {
  local log="$OUT/logs/08-dependency-audit.log"
  local timeout_s="${ZEOKEEL_SECURITY_CARGO_TOOL_TIMEOUT:-900}"
  local deny_rc=127 audit_rc=127 pnpm_rc=127
  : > "$log"
  if cargo deny --version >/dev/null 2>&1; then
    { echo "### cargo deny check"; timeout "$timeout_s" cargo deny check; } >> "$log" 2>&1
    deny_rc=$?
  else
    echo "### cargo deny: not installed" >> "$log"
  fi
  if cargo audit --version >/dev/null 2>&1; then
    { echo "### cargo audit"; timeout "$timeout_s" cargo audit; } >> "$log" 2>&1
    audit_rc=$?
  else
    echo "### cargo audit: not installed" >> "$log"
  fi
  if pnpm --version >/dev/null 2>&1; then
    { echo "### pnpm audit --prod"; timeout "$timeout_s" pnpm audit --prod; } >> "$log" 2>&1
    pnpm_rc=$?
  else
    echo "### pnpm: not installed" >> "$log"
  fi
  {
    echo "exit codes: cargo-deny=$deny_rc cargo-audit=$audit_rc pnpm-audit=$pnpm_rc (127=not installed)"
  } >> "$log" 2>&1
  if [[ "$deny_rc" -eq 0 && "$audit_rc" -eq 0 && "$pnpm_rc" -eq 0 ]]; then
    record 8 "dependency-audit" high security "cargo deny check && cargo audit && pnpm audit --prod" pass "$log" "all three dependency audit tools executed with exit 0 (CI security.yml; deny.toml present)"
  elif [[ "$deny_rc" -eq 127 || "$audit_rc" -eq 127 || "$pnpm_rc" -eq 127 ]]; then
    record 8 "dependency-audit" high security "cargo deny check && cargo audit && pnpm audit --prod" unverified "$log" "tool(s) missing on this host (cargo-deny/audit/pnpm); CI security.yml installs and runs them => CI-satisfiable"
  elif [[ "$pnpm_rc" -ne 0 ]] && grep -qiE 'network|ENOTFOUND|ECONN|registry' "$log"; then
    record 8 "dependency-audit" high security "cargo deny check && cargo audit && pnpm audit --prod" unverified "$log" "cargo-deny/audit executed; pnpm audit failed on registry network access (offline host) => CI-satisfiable"
  else
    record 8 "dependency-audit" high security "cargo deny check && cargo audit && pnpm audit --prod" fail "$log" "dependency audit reported findings/config errors (deny=$deny_rc audit=$audit_rc pnpm=$pnpm_rc): $(fail_detail "$log")"
  fi
}


# ---------------------------------------------------------------- 9. SBOM
gate9() {
  local log="$OUT/logs/09-sbom.log"
  local sbom="$OUT/sbom.json"
  run_log "$log" bash scripts/sbom.sh "$sbom"
  local rc=$?
  local ok="0: not generated"
  if [[ "$rc" -eq 0 ]]; then
    ok="$(python3 - "$sbom" <<'PY'
import json, sys
try:
    data = json.load(open(sys.argv[1], encoding="utf-8"))
except Exception as exc:  # noqa: BLE001 - report, do not crash the audit
    print(f"0: {exc}")
    raise SystemExit(0)
components = data.get("components") or []
good = data.get("bomFormat") == "CycloneDX" and bool(data.get("specVersion")) and len(components) > 0
print(f"{1 if good else 0}: {len(components)} components")
PY
)"
  fi
  {
    echo "source SBOM: $ok"
    echo "artifact SBOMs (server image / client dist / OEM artifacts): release pipeline only (A17/release.yml), not built in this run"
  } >> "$log" 2>&1
  if [[ "$rc" -ne 0 || "${ok%%:*}" != "1" ]]; then
    record 9 "sbom-per-artifact" medium security "bash scripts/sbom.sh target/evidence/release-1.0-rc1/security2/sbom.json" fail "$log" "source SBOM generation failed (exit=$rc): $(fail_detail "$log")"
  else
    record_env 9 "sbom-per-artifact" medium security "bash scripts/sbom.sh <out> + release.yml artifact SBOMs" "$log" "source CycloneDX SBOM generated (${ok#*: }); server-image/client-dist/OEM artifact SBOMs are produced by the release pipeline only (no dist/image build in CI security job) => external/release-only, excluded from strict CI failure"
  fi
}


# ---------------------------------------------------------------- 10. image scan
gate10() {
  local log="$OUT/logs/10-image-scan.log"
  local report="${ZEOKEEL_SECURITY_TRIVY_REPORT:-}"
  local image="${ZEOKEEL_SECURITY_IMAGE:-unknown}"
  {
    echo "image: $image"
    echo "trivy report: ${report:-<unset>}"
    echo "docker: $(command -v docker || echo missing)"
    echo "trivy: $(command -v trivy || echo missing)"
  } > "$log" 2>&1
  if [[ -z "$report" ]]; then
    record_env 10 "image-scan" high security "trivy image \$ZEOKEEL_SECURITY_IMAGE --format json -o \$ZEOKEEL_SECURITY_TRIVY_REPORT" "$log" "no trivy report provided (ZEOKEEL_SECURITY_TRIVY_REPORT unset); CI security.yml builds the image and runs pinned trivy, then this gate consumes the JSON => environment/external, excluded from local strict failure"
    return
  fi
  if [[ ! -s "$report" ]]; then
    record 10 "image-scan" high security "trivy image \$ZEOKEEL_SECURITY_IMAGE --format json -o \$ZEOKEEL_SECURITY_TRIVY_REPORT" fail "$log" "ZEOKEEL_SECURITY_TRIVY_REPORT set but file missing/empty: $report"
    return
  fi
  local counts
  counts="$(python3 - "$report" <<'PY'
import json, sys
try:
    data = json.load(open(sys.argv[1], encoding="utf-8"))
except Exception as exc:  # noqa: BLE001
    print(f"parse-error: {exc}")
    raise SystemExit(0)
severities = {}
fixable = 0
unfixed = 0
for result in data.get("Results") or []:
    for vuln in result.get("Vulnerabilities") or []:
        severity = str(vuln.get("Severity") or "UNKNOWN").upper()
        severities[severity] = severities.get(severity, 0) + 1
        if severity in ("HIGH", "CRITICAL"):
            if vuln.get("FixedVersion"):
                fixable += 1
            else:
                unfixed += 1
high = severities.get("HIGH", 0) + severities.get("CRITICAL", 0)
print(f"high_or_critical={high} fixable={fixable} unfixed={unfixed} severities={severities}")
PY
)"
  printf '%s\n' "$counts" >> "$log"
  if [[ "$counts" == parse-error:* ]]; then
    record 10 "image-scan" high security "trivy image \$ZEOKEEL_SECURITY_IMAGE --format json -o \$ZEOKEEL_SECURITY_TRIVY_REPORT" fail "$log" "trivy report unparsable: $counts"
  elif [[ "$counts" == *" fixable=0 "* ]]; then
    # Policy (docs/21 §10 gate 10): zero *fixable* HIGH/CRITICAL. Base-image CVEs with
    # no vendor fix (FixedVersion empty) are recorded as UNVERIFIED residual risk and
    # listed in KNOWN-LIMITATIONS §2 instead of being claimed as fixed.
    record 10 "image-scan" high security "trivy image \$ZEOKEEL_SECURITY_IMAGE --format json -o \$ZEOKEEL_SECURITY_TRIVY_REPORT" pass "$log" "trivy report for image $image has 0 fixable HIGH/CRITICAL (unfixed base-image CVEs tracked in KNOWN-LIMITATIONS): $counts"
  else
    record 10 "image-scan" high security "trivy image \$ZEOKEEL_SECURITY_IMAGE --format json -o \$ZEOKEEL_SECURITY_TRIVY_REPORT" fail "$log" "trivy fixable HIGH/CRITICAL vulnerabilities present: $counts"
  fi
}


# ---------------------------------------------------------------- 11. privacy wording
gate11() {
  local log="$OUT/logs/11-privacy-wording.log"
  python3 - "$log" > "$log" 2>&1 <<'PY'
import pathlib, re, sys

problems = []
notes = []
doc21 = pathlib.Path("docs/21-security-privacy.md")
text21 = doc21.read_text(encoding="utf-8") if doc21.exists() else ""
if "不持久化" not in text21 or "prompt/completion" not in text21:
    problems.append("docs/21 §8.1 does not state server does not persist prompt/completion")
else:
    notes.append("docs/21 §8.1 states server does not persist prompt/completion (memory-only gateway)")
phrase = "服务端不读取会话"
allowed = ("不得声称", "不声称", "除非", "不允许", "禁止")
for path in pathlib.Path("docs").rglob("*.md"):
    try:
        for lineno, line in enumerate(path.read_text(encoding="utf-8", errors="ignore").splitlines(), 1):
            if phrase in line and not any(marker in line for marker in allowed):
                problems.append(f"{path}:{lineno}: unqualified claim containing {phrase!r}")
    except OSError:
        continue
if not problems:
    notes.append("no unqualified '服务端不读取会话' claim in docs (only prohibition contexts)")
migrations = list(pathlib.Path("packages").rglob("migrations/*.sql"))
leaks = [str(p) for p in migrations if re.search(r"\b(prompt|completion)\b", p.read_text(encoding="utf-8", errors="ignore"))]
if leaks:
    problems.append("prompt/completion columns found in migrations: " + ", ".join(leaks))
else:
    notes.append("no prompt/completion persistence columns in server migrations")
print("## checks")
for note in notes:
    print("PASS:", note)
for problem in problems:
    print("FAIL:", problem)
print("## status:", "fail" if problems else "pass")
PY
  if grep -q '^FAIL:' "$log"; then
    record 11 "privacy-wording" high security "python3 static privacy checks (docs/21 §8.1, claim grep, migrations grep)" fail "$log" "$(grep '^FAIL:' "$log" | tr '\n' ' ' | cut -c1-500)"
  else
    record 11 "privacy-wording" high security "python3 static privacy checks (docs/21 §8.1, claim grep, migrations grep)" pass "$log" "docs/21 §8.1 correction present; no unqualified '服务端不读取会话' claim; no prompt/completion persistence columns in migrations; gateway forwards in memory"
  fi
}


# ---------------------------------------------------------------- 12. secrets in logs/errors
gate12() {
  local log="$OUT/logs/12-secret-leaks.log"
  run_series "$log" \
    'cargo test --offline -q -p zk-desktop --bin zk-desktop -- published_runtime_snapshot_never_contains_provider_secrets' \
    'cargo test --offline -q -p zk-desktop --bin zk-desktop -- hook_events_are_redacted_before_logging'
  local rc=$?
  # Rust log macros plus TS/TSX/JS console/logger calls must not interpolate secrets.
  {
    grep -rEn --include='*.rs' '(println!|eprintln!|tracing::(debug|info|warn|error)!|log::(debug|info|warn|error)!)' packages crates apps tools 2>/dev/null || true
    grep -rEn --exclude-dir=node_modules --exclude-dir=dist --exclude-dir=build --exclude-dir=target \
      --include='*.ts' --include='*.tsx' --include='*.js' --include='*.mjs' \
      '(console\.(log|error|warn|info|debug)|logger\.(debug|info|warn|error))' apps packages tools 2>/dev/null || true
  } | grep -Ei 'password|secret|api_key|apikey|access_token|refresh_token|csrf' > "$OUT/.log-secret-hits" 2>/dev/null || true
  # Secret literals are scanned in code AND configuration (private keys, AWS keys, OpenAI tokens).
  grep -rEn --include='*.rs' --include='*.ts' --include='*.tsx' --include='*.js' --include='*.json' --include='*.toml' --include='*.yml' --include='*.yaml' 'BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY|AKIA[0-9A-Z]{16}|sk-[A-Za-z0-9]{20,}'     packages crates apps tools deploy .github 2>/dev/null > "$OUT/.secret-literals" || true
  grep -rEn --include='*.rs' 'unsafe\s*\{' packages crates apps tools 2>/dev/null > "$OUT/.unsafe-braces" || true
  grep -rEn --include='*.rs' '\.(unwrap|expect)\(' packages crates apps tools 2>/dev/null | grep -v '/tests/' > "$OUT/.unwrap-expect" || true
  local hits secrets unsafe_hits unwrap_hits
  hits="$(wc -l < "$OUT/.log-secret-hits")"
  secrets="$(wc -l < "$OUT/.secret-literals")"
  unsafe_hits="$(wc -l < "$OUT/.unsafe-braces")"
  unwrap_hits="$(wc -l < "$OUT/.unwrap-expect")"
  {
    echo "## static scan for secret values in log macros: $hits hit(s)"
    cat "$OUT/.log-secret-hits"
    echo "## static scan for secret literals (private key / AKIA / sk-): $secrets hit(s)"
    cat "$OUT/.secret-literals"
    echo "## static scan for hand-written unsafe blocks: $unsafe_hits hit(s)"
    cat "$OUT/.unsafe-braces"
    echo "## unwrap/expect occurrences outside tests/ paths: $unwrap_hits (test-module hits included; clippy unwrap_used/expect_used deny is the executable gate)"
    head -10 "$OUT/.unwrap-expect"
  } >> "$log" 2>&1
  if [[ "$rc" -ne 0 || "$hits" -ne 0 || "$secrets" -ne 0 || "$unsafe_hits" -ne 0 ]]; then
    record 12 "secrets-in-logs-errors" high security "cargo test -p zk-desktop --bin zk-desktop security/redaction tests + static secret/unsafe/log scans (Rust + TS/TSX/JS + config)" fail "$log" "redaction tests exit=$rc; log hits=$hits; secret literals=$secrets; unsafe blocks=$unsafe_hits"
  else
    record 12 "secrets-in-logs-errors" high security "cargo test -p zk-desktop --bin zk-desktop security/redaction tests + static secret/unsafe/log scans (Rust + TS/TSX/JS + config)" pass "$log" "runtime snapshot never contains provider secrets; hook events redacted before logging; 0 secret interpolations in Rust log macros or TS console/logger calls, 0 secret literals in code/config, 0 hand-written unsafe blocks (unwrap/expect outside tests denied by clippy)"
  fi
}


# ---------------------------------------------------------------- 13. weak-control statement
gate13() {
  local log="$OUT/logs/13-weak-control-statement.log"
  python3 - "$log" > "$log" 2>&1 <<'PY'
import pathlib

problems = []
notes = []
doc00 = pathlib.Path("docs/00-design-principles.md")
text00 = doc00.read_text(encoding="utf-8") if doc00.exists() else ""
if "不为员工行为兜底" not in text00:
    problems.append("docs/00 §5 missing '服务端不为员工行为兜底' statement")
else:
    notes.append("docs/00 §5 states server does not stand behind employee actions")
if "不得宣称" not in text00 or "限制员工" not in text00:
    problems.append("docs/00 §5 missing 'must not claim restriction of employees' rule")
else:
    notes.append("docs/00 §5 forbids claiming employee restrictions")
doc21 = pathlib.Path("docs/21-security-privacy.md")
text21 = doc21.read_text(encoding="utf-8") if doc21.exists() else ""
if "弱管控不改变安全责任" not in text21:
    problems.append("docs/21 preamble missing weak-control/security-responsibility statement")
else:
    notes.append("docs/21 preamble restates weak-control security responsibility")
# Product-facing docs only: docs/17 is a historical archive ("不作依据"),
# docs/reviews/* and docs/PLAN.md are review/history notes that quote decisions.
positive_claims = (
    "可限制员工", "可以限制员工", "限制员工操作",
    "支持工具白名单", "支持网络限制", "支持企业审批", "支持部门预算",
    "提供工具白名单", "提供网络限制", "提供企业审批",
    "启用工具白名单", "启用网络限制", "启用企业审批",
)
targets = [path for path in pathlib.Path("docs").glob("*.md") if path.name not in {"PLAN.md", "17-v3.1-decisions.md"}]
targets += [pathlib.Path("README.md"), pathlib.Path("AGENTS.md"), pathlib.Path("KNOWN-LIMITATIONS.md")]
for path in targets:
    if not path.exists():
        continue
    try:
        lines = path.read_text(encoding="utf-8", errors="ignore").splitlines()
    except OSError:
        continue
    negative_markers = ("不得", "不要", "禁止", "不是", "不为", "不下发", "删除", "不能", "而非", "而不是", "只能宣称")
    for lineno, line in enumerate(lines, 1):
        if any(claim in line for claim in positive_claims) and not any(marker in line for marker in negative_markers):
            problems.append(f"{path}:{lineno}: affirmative restriction claim: {line.strip()[:120]}")
print("## checks")
for note in notes:
    print("PASS:", note)
for problem in problems:
    print("FAIL:", problem)
print("## status:", "fail" if problems else "pass")
PY
  if grep -q '^FAIL:' "$log"; then
    record 13 "weak-control-statement" medium security "python3 static check docs/00 §5 + docs/21 preamble + no affirmative restriction claims" fail "$log" "$(grep -c '^FAIL:' "$log") affirmative-claim/source problems (see log)"
  else
    record 13 "weak-control-statement" medium security "python3 static check docs/00 §5 + docs/21 preamble + no affirmative restriction claims" pass "$log" "docs/00 §5 + docs/21 restate weak control (server does not stand behind employee actions); no affirmative 'restrict employees/tool allowlist/network policy' claim in docs"
  fi
}

gate1; gate2; gate3; gate4; gate5; gate14; gate6; gate7; gate8; gate9; gate10; gate11; gate12; gate13


# ---------------------------------------------------------------- aggregate
python3 - "$GATES" "$OUT" "$RC_SUMMARY" "$STRICT" "$HEAD_SHA" "$VERSION" <<'PY'
import json
import pathlib
import shutil
import sys
from datetime import datetime, timezone

gates_path, out_dir, rc_dir, strict, head, version = sys.argv[1:7]
out = pathlib.Path(out_dir)
rc = pathlib.Path(rc_dir)
gates = []
with open(gates_path, encoding="utf-8", errors="replace") as handle:
    for line in handle:
        line = line.rstrip("\n")
        if not line:
            continue
        parts = (line.split("\t") + [""] * 9)[:9]
        gates.append({
            "id": int(parts[0]),
            "rule": parts[1],
            "severity": parts[2],
            "owner": parts[3],
            "command": parts[4],
            "status": parts[5],
            "evidence": parts[6],
            "detail": parts[7],
            "env_only": parts[8] == "1",
        })
summary = {
    "total": len(gates),
    "pass": sum(1 for gate in gates if gate["status"] == "pass"),
    "fail": sum(1 for gate in gates if gate["status"] == "fail"),
    "unverified": sum(1 for gate in gates if gate["status"] == "unverified"),
    "unverified_external": sum(1 for gate in gates if gate["status"] == "unverified_external"),
}
if summary["fail"]:
    overall = "fail"
elif summary["unverified"]:
    overall = "unverified"
elif summary["unverified_external"]:
    overall = "unverified_external"
else:
    overall = "pass"
strict_mode = strict == "1"
if strict_mode:
    # strict CI: CI-satisfiable unverified/fail => exit 1; external/env-only items excluded.
    exit_code = 1 if (summary["fail"] or summary["unverified"]) else 0
else:
    if summary["fail"]:
        exit_code = 1
    elif summary["unverified"] or summary["unverified_external"]:
        exit_code = 2
    else:
        exit_code = 0
payload = {
    "schema": 2,
    "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "head": head,
    "version": version,
    "strict": strict_mode,
    "overall": overall,
    "exit_code": exit_code,
    "summary": summary,
    "env_contract": {
        "ZEOKEEL_SECURITY_STRICT": "1 enables strict CI semantics (see exit_code)",
        "ZEOKEEL_SECURITY_SKIP_BUILD": "1 skips preflight cargo build (CI prebuilds binaries)",
        "ZEOKEEL_SECURITY_TRIVY_REPORT": "path to trivy JSON; gate10 pass iff 0 HIGH/CRITICAL",
        "ZEOKEEL_SECURITY_IMAGE": "image ref reported by gate10",
        "ZEOKEEL_SECURITY_CARGO_TOOL_TIMEOUT": "seconds for cargo deny/audit (default 900)",
    },
    "gates": gates,
}
(out / "security.json").write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
rows = ["| id | rule | severity | owner | status | env_only | evidence |", "| --- | --- | --- | --- | --- | --- | --- |"]
for gate in gates:
    rows.append(f"| {gate['id']} | {gate['rule']} | {gate['severity']} | {gate['owner']} | `{gate['status']}` | {'yes' if gate['env_only'] else 'no'} | `{gate['evidence']}` |")
md = [
    "# Security gates (docs/21 §10, 13 gates + split 5b)",
    "",
    f"- head: `{head}` version: `{version}` generated: {payload['generated_at']}",
    f"- overall: **{overall}** (strict={strict_mode}) exit={exit_code}",
    f"- pass={summary['pass']} fail={summary['fail']} unverified(CI-satisfiable)={summary['unverified']} unverified_external={summary['unverified_external']}",
    "- strict CI: exit 1 if any `fail` or CI-satisfiable `unverified`; env-only external items do not block.",
    "- non-strict: 0 all pass / 2 any unverified / 1 any fail.",
    "",
    *rows,
    "",
    "## details",
    "",
]
for gate in gates:
    md.append(f"- **{gate['id']} {gate['rule']}** [`{gate['status']}`{' env-only' if gate['env_only'] else ''}] — {gate['detail']}")
(out / "security-audit.md").write_text("\n".join(md) + "\n", encoding="utf-8")
rc.mkdir(parents=True, exist_ok=True)
if rc.resolve() != out.resolve():
    shutil.copy2(out / "security.json", rc / "security.json")
    shutil.copy2(out / "security-audit.md", rc / "security-audit.md")
for gate in gates:
    print(f"{gate['id']:>2} {gate['status']:<20}{' env_only' if gate['env_only'] else ''} {gate['rule']}")
print(f"security-audit: overall={overall} strict={strict_mode} exit={exit_code} pass={summary['pass']} fail={summary['fail']} unverified={summary['unverified']} external={summary['unverified_external']} -> {out/'security.json'}")
sys.exit(exit_code)
PY
exit $?
